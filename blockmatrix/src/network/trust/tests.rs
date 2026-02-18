// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for multi-network trust handlers

use super::*;
use tokio;

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test that all four network handlers can be created
    #[tokio::test]
    async fn test_create_all_handlers() {
        let anonymous = AnonymousNetworkHandler::new();
        assert_eq!(anonymous.network_type(), NetworkType::Anonymous);

        let p2p = P2PNetworkHandler::new();
        assert_eq!(p2p.network_type(), NetworkType::P2P);

        let federated = FederatedNetworkHandler::new();
        match federated.network_type() {
            NetworkType::Federated { .. } => {},
            _ => panic!("Wrong network type for federated"),
        }

        let public = PublicNetworkHandler::new();
        assert_eq!(public.network_type(), NetworkType::Public);
    }

    /// Test anonymous network bootstrap
    #[tokio::test]
    async fn test_anonymous_bootstrap() {
        let handler = AnonymousNetworkHandler::new();
        let config = NetworkConfig {
            network_type: NetworkType::Anonymous,
            peer_addresses: vec![],
            federation_gateway: None,
            dns_name: None,
            proof_of_state: None,
        };

        let connection = handler.bootstrap(config).await.unwrap();

        // Anonymous should have no certificate
        assert!(connection.certificate.is_none());
        assert_eq!(connection.network_type, NetworkType::Anonymous);

        // Should be able to connect
        handler.connect().await.unwrap();

        // Should accept all anonymous peers
        let peer = PeerInfo {
            peer_id: PeerId::new("anon-peer".to_string()),
            address: "127.0.0.1:8080".to_string(),
            certificate: None,
            network_type: NetworkType::Anonymous,
        };
        assert!(handler.validate_peer(&peer).await.unwrap());

        // Disconnect should clear all data
        handler.disconnect().await.unwrap();
    }

    /// Test P2P network with self-signed certificates
    #[tokio::test]
    async fn test_p2p_bootstrap() {
        let handler = P2PNetworkHandler::new();
        let config = NetworkConfig {
            network_type: NetworkType::P2P,
            peer_addresses: vec!["127.0.0.1:8080".to_string()],
            federation_gateway: None,
            dns_name: None,
            proof_of_state: None,
        };

        let connection = handler.bootstrap(config).await.unwrap();

        // P2P should have self-signed certificate
        assert!(connection.certificate.is_some());
        assert!(connection.certificate.as_ref().unwrap().is_self_signed());
        assert_eq!(connection.network_type, NetworkType::P2P);

        // Should be able to connect
        handler.connect().await.unwrap();
    }

    /// Test federated network requires gateway
    #[tokio::test]
    async fn test_federated_requires_gateway() {
        let handler = FederatedNetworkHandler::new();
        let config = NetworkConfig {
            network_type: NetworkType::Federated {
                gateway_url: "bank.internal".to_string()
            },
            peer_addresses: vec![],
            federation_gateway: None, // Missing gateway
            dns_name: None,
            proof_of_state: None,
        };

        let result = handler.bootstrap(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("gateway URL required"));
    }

    /// Test federated network with gateway
    #[tokio::test]
    async fn test_federated_bootstrap() {
        let handler = FederatedNetworkHandler::new();
        let config = NetworkConfig {
            network_type: NetworkType::Federated {
                gateway_url: "bank.internal".to_string()
            },
            peer_addresses: vec![],
            federation_gateway: Some("bank.internal".to_string()),
            dns_name: None,
            proof_of_state: None,
        };

        let connection = handler.bootstrap(config).await.unwrap();

        // Federated should have certificate from gateway
        assert!(connection.certificate.is_some());
        match connection.network_type {
            NetworkType::Federated { gateway_url } => {
                assert_eq!(gateway_url, "bank.internal");
            }
            _ => panic!("Wrong network type"),
        }
    }

    /// Test public network requires proof of state
    #[tokio::test]
    async fn test_public_requires_proof() {
        let handler = PublicNetworkHandler::new();
        let config = NetworkConfig {
            network_type: NetworkType::Public,
            peer_addresses: vec![],
            federation_gateway: None,
            dns_name: None,
            proof_of_state: None, // Missing proof
        };

        let result = handler.bootstrap(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Proof of State required"));
    }

    /// Test public network with full proof
    #[tokio::test]
    async fn test_public_bootstrap() {
        let handler = PublicNetworkHandler::new();
        let proof = ProofOfState {
            proof_of_space: vec![1, 2, 3],
            proof_of_stake: vec![4, 5, 6],
            proof_of_work: vec![7, 8, 9],
            proof_of_time: vec![10, 11, 12],
        };

        let config = NetworkConfig {
            network_type: NetworkType::Public,
            peer_addresses: vec![],
            federation_gateway: None,
            dns_name: Some("node.hypermesh".to_string()),
            proof_of_state: Some(proof),
        };

        let connection = handler.bootstrap(config).await.unwrap();

        // Public should have blockchain-registered certificate
        assert!(connection.certificate.is_some());
        assert!(connection.certificate.as_ref().unwrap().is_blockchain_registered());
        assert_eq!(connection.network_type, NetworkType::Public);
    }

    /// Test network isolation - peers from different networks shouldn't validate
    #[tokio::test]
    async fn test_network_isolation() {
        let anonymous = AnonymousNetworkHandler::new();
        let p2p = P2PNetworkHandler::new();
        let public = PublicNetworkHandler::new();

        // Create peer from different network
        let public_peer = PeerInfo {
            peer_id: PeerId::new("public-peer".to_string()),
            address: "127.0.0.1:8080".to_string(),
            certificate: Some(Certificate {
                subject: "public".to_string(),
                issuer: "trust.hypermesh.online".to_string(),
                public_key: vec![0; 32],
                signature: vec![0; 64],
                fingerprint: "public".to_string(),
                expires_at: u64::MAX,
                network_type: NetworkType::Public,
                blockchain_registered: true,
            }),
            network_type: NetworkType::Public,
        };

        // Anonymous shouldn't validate public peer
        assert!(!anonymous.validate_peer(&public_peer).await.unwrap());

        // P2P shouldn't validate public peer
        assert!(!p2p.validate_peer(&public_peer).await.unwrap());

        // But public should validate public peer
        assert!(public.validate_peer(&public_peer).await.unwrap());
    }

    /// Test asset request handling varies by network
    #[tokio::test]
    async fn test_asset_request_handling() {
        let anonymous = AnonymousNetworkHandler::new();
        let p2p = P2PNetworkHandler::new();

        let request = AssetRequest {
            asset_id: "test-asset".to_string(),
            network_type: NetworkType::Anonymous,
            peer_id: None,
            metadata: std::collections::HashMap::new(),
        };

        // Anonymous should allow access (public assets)
        let anon_response = anonymous.handle_asset_request(request.clone()).await.unwrap();
        assert!(anon_response.authorized);
        assert_eq!(anon_response.metadata.get("network").unwrap(), "anonymous");

        // P2P should deny access (no trusted peer)
        let p2p_request = AssetRequest {
            network_type: NetworkType::P2P,
            ..request
        };
        let p2p_response = p2p.handle_asset_request(p2p_request).await.unwrap();
        assert!(!p2p_response.authorized); // No trusted peers yet
    }

    /// Test certificate validation
    #[test]
    fn test_certificate_properties() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Self-signed certificate
        let self_signed = Certificate {
            subject: "node".to_string(),
            issuer: "node".to_string(),
            public_key: vec![0; 32],
            signature: vec![0; 64],
            fingerprint: "test".to_string(),
            expires_at: now + 3600, // Valid for 1 hour
            network_type: NetworkType::P2P,
            blockchain_registered: false,
        };

        assert!(self_signed.is_self_signed());
        assert!(!self_signed.is_blockchain_registered());
        assert!(!self_signed.is_expired());

        // Expired certificate
        let expired = Certificate {
            expires_at: now - 3600, // Expired 1 hour ago
            ..self_signed.clone()
        };
        assert!(expired.is_expired());

        // Blockchain-registered certificate
        let blockchain = Certificate {
            subject: "blockchain-node".to_string(),
            issuer: "trust.hypermesh.online".to_string(),
            blockchain_registered: true,
            ..self_signed
        };
        assert!(!blockchain.is_self_signed());
        assert!(blockchain.is_blockchain_registered());
    }
}

// Unit tests (non-async)
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_network_types() {
        assert_eq!(NetworkType::Anonymous.name(), "Anonymous");
        assert_eq!(NetworkType::P2P.name(), "P2P");
        assert_eq!(NetworkType::Public.name(), "Public");

        let federated = NetworkType::Federated {
            gateway_url: "bank.internal".to_string()
        };
        assert_eq!(federated.name(), "Federated");
    }

    #[test]
    fn test_certificate_validation() {
        let cert = Certificate {
            subject: "node".to_string(),
            issuer: "node".to_string(),
            public_key: vec![0; 32],
            signature: vec![0; 64],
            fingerprint: "test".to_string(),
            expires_at: 0,
            network_type: NetworkType::P2P,
            blockchain_registered: false,
        };

        assert!(cert.is_self_signed());
        assert!(!cert.is_blockchain_registered());
        assert!(cert.is_expired()); // expires_at is 0
    }

    #[test]
    fn test_network_id() {
        let id1 = new_random_network_id();
        let id2 = new_random_network_id();
        assert_ne!(id1, id2); // Should be unique
    }

    #[test]
    fn test_ephemeral_key() {
        let key = EphemeralKey::generate();
        assert_eq!(key.public_key.len(), 32);
        assert_ne!(key.session_id, uuid::Uuid::nil());
    }
}