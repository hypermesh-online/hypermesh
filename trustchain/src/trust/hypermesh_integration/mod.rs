// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Trust Integration with Byzantine Fault Detection
//!
//! Integrates TrustChain certificate authority with HyperMesh asset system,
//! providing binary authentication, Byzantine fault detection, and remote proxy management.

pub mod operations;
pub mod types;

// Re-export all public types for backward compatibility
pub use operations::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_trust_validator_creation() {
        let config = TrustValidatorConfig::default();
        let _validator = HyperMeshTrustValidator::new(config).await;
    }

    #[test]
    fn test_authenticated_asset_creation() {
        let asset = AuthenticatedAsset {
            asset_id: hypermesh_lib::AssetId::from("test-asset-001"),
            uuid: Uuid::new_v4(),
            asset_kind: TrustAssetKind::Cpu,
            network_id: "test-network".to_string(),
        };
        assert_eq!(asset.asset_kind, TrustAssetKind::Cpu);
        assert_eq!(asset.network_id, "test-network");
    }

    #[test]
    fn test_authentication_status() {
        let status = AuthenticationStatus {
            authenticated: true,
            certificate_valid: true,
            state_verified: true,
            last_checked: SystemTime::now(),
            expiry: SystemTime::now() + std::time::Duration::from_secs(3600),
        };
        assert!(status.authenticated);
        assert!(status.certificate_valid);
        assert!(status.state_verified);
    }

    #[test]
    fn test_byzantine_fault_types() {
        let fault_types = [
            ByzantineFaultType::DoubleSigning,
            ByzantineFaultType::EquivocationAttack,
            ByzantineFaultType::NothingAtStake,
            ByzantineFaultType::LongRangeAttack,
        ];
        assert_eq!(fault_types.len(), 4);
    }

    #[test]
    fn test_proxy_connection_types() {
        let proxy_types = [
            ProxyType::Direct,
            ProxyType::Encrypted,
            ProxyType::Federated,
            ProxyType::Anonymous,
        ];
        assert_eq!(proxy_types.len(), 4);
    }

    #[test]
    fn test_alert_thresholds() {
        let thresholds = AlertThresholds {
            byzantine_confidence: 0.8,
            performance_degradation: 0.5,
            availability_threshold: 0.95,
        };
        assert!(thresholds.byzantine_confidence > 0.7);
        assert!(thresholds.availability_threshold > 0.9);
    }

    // --- Items 1.3-1.8: Cross-crate identity integration ---

    fn make_test_node() -> AuthenticatedNode {
        AuthenticatedNode {
            node_id: hypermesh_lib::NodeId::from_public_key(b"test-falcon-pubkey"),
            public_key: "test-pubkey-hex".to_string(),
            network_address: std::net::Ipv6Addr::LOCALHOST,
            node_type: NodeType::Full,
        }
    }

    #[test]
    fn test_trust_asset_kind_includes_transmission() {
        // TrustAssetKind must include Transmission (R10).
        let kind = TrustAssetKind::Transmission;
        let system: hypermesh_lib::asset::SystemAssetKind = kind.into();
        assert_eq!(system, hypermesh_lib::asset::SystemAssetKind::Transmission);
    }

    #[test]
    fn test_trust_asset_kind_roundtrip_all_variants() {
        use hypermesh_lib::asset::SystemAssetKind;
        let all = [
            SystemAssetKind::Cpu,
            SystemAssetKind::Gpu,
            SystemAssetKind::Memory,
            SystemAssetKind::Storage,
            SystemAssetKind::Network,
            SystemAssetKind::Container,
            SystemAssetKind::Economic,
            SystemAssetKind::Blockchain,
            SystemAssetKind::Dns,
            SystemAssetKind::Transmission,
        ];
        for original in all {
            let trust: TrustAssetKind = original.into();
            let back: SystemAssetKind = trust.into();
            assert_eq!(original, back);
        }
    }

    #[test]
    fn test_authenticated_node_to_scoped_identity() {
        let node = make_test_node();
        let scope = hypermesh_lib::IdentityScope::private_network();
        let identity = node.to_scoped_identity(scope);

        assert_eq!(identity.node_id, node.node_id);
        assert_eq!(identity.workload_type, hypermesh_lib::WorkloadType::Node);
        assert_eq!(identity.scope, scope);
        assert!(identity.label.is_none());
    }

    #[test]
    fn test_node_identity_construction() {
        let node = make_test_node();
        let scope = hypermesh_lib::IdentityScope::public_network();
        let fingerprint = [0xABu8; 32];
        let ni = NodeIdentity {
            node: node.clone(),
            scope,
            certificate_fingerprint: Some(fingerprint),
        };
        assert_eq!(ni.node.node_id, node.node_id);
        assert_eq!(ni.scope.blockchain_scope, hypermesh_lib::BlockchainScope::Network);
        assert!(ni.scope.tracked);
        assert_eq!(ni.certificate_fingerprint, Some(fingerprint));
    }

    #[test]
    fn test_service_identity_construction() {
        let host = hypermesh_lib::NodeId::from_public_key(b"host-key");
        let si = ServiceIdentity {
            service_id: hypermesh_lib::AssetId::from("svc-001"),
            host_node: host,
            scope: hypermesh_lib::IdentityScope::anonymous_device(),
            service_name: "dns-resolver".to_string(),
        };
        assert_eq!(si.service_name, "dns-resolver");
        assert_eq!(si.host_node, host);
        assert!(!si.scope.tracked);
    }

    #[test]
    fn test_agent_identity_construction() {
        let controller = hypermesh_lib::NodeId::from_public_key(b"controller-key");
        let ai = AgentIdentity {
            agent_id: hypermesh_lib::AssetId::from("agent-007"),
            controlling_node: controller,
            scope: hypermesh_lib::IdentityScope::private_network(),
            capabilities: vec!["sign".to_string(), "relay".to_string()],
        };
        assert_eq!(ai.capabilities.len(), 2);
        assert_eq!(ai.controlling_node, controller);
    }

    #[test]
    fn test_certificate_subject_type_from_workload() {
        assert_eq!(
            CertificateSubjectType::from(hypermesh_lib::WorkloadType::Node),
            CertificateSubjectType::Node,
        );
        assert_eq!(
            CertificateSubjectType::from(hypermesh_lib::WorkloadType::Service),
            CertificateSubjectType::Service,
        );
        assert_eq!(
            CertificateSubjectType::from(hypermesh_lib::WorkloadType::Agent),
            CertificateSubjectType::Agent,
        );
    }

    #[test]
    fn test_identity_scope_extension_roundtrip() {
        let ext = IdentityScopeExtension {
            subject_type: CertificateSubjectType::Service,
            blockchain_scope: hypermesh_lib::BlockchainScope::Network,
            tracked: true,
            workload_type: hypermesh_lib::WorkloadType::Service,
        };
        let bytes = ext.to_bytes();
        assert_eq!(bytes, [1, 1, 1, 1]); // Service=1, Network=1, tracked=1, Service=1

        let decoded = IdentityScopeExtension::from_bytes(&bytes)
            .expect("test: decode should succeed");
        assert_eq!(decoded.subject_type, CertificateSubjectType::Service);
        assert_eq!(decoded.blockchain_scope, hypermesh_lib::BlockchainScope::Network);
        assert!(decoded.tracked);
        assert_eq!(decoded.workload_type, hypermesh_lib::WorkloadType::Service);
    }

    #[test]
    fn test_identity_scope_extension_from_scope() {
        let scope = hypermesh_lib::IdentityScope::anonymous_device();
        let ext = IdentityScopeExtension::from_scope(
            &scope,
            hypermesh_lib::WorkloadType::Agent,
        );
        assert_eq!(ext.subject_type, CertificateSubjectType::Agent);
        assert_eq!(ext.blockchain_scope, hypermesh_lib::BlockchainScope::Device);
        assert!(!ext.tracked);
        assert_eq!(ext.workload_type, hypermesh_lib::WorkloadType::Agent);
    }

    #[test]
    fn test_identity_scope_extension_invalid_bytes() {
        // Invalid subject_type
        assert!(IdentityScopeExtension::from_bytes(&[99, 0, 0, 0]).is_none());
        // Invalid blockchain_scope
        assert!(IdentityScopeExtension::from_bytes(&[0, 5, 0, 0]).is_none());
        // Invalid tracked
        assert!(IdentityScopeExtension::from_bytes(&[0, 0, 2, 0]).is_none());
        // Invalid workload_type
        assert!(IdentityScopeExtension::from_bytes(&[0, 0, 0, 7]).is_none());
    }

    #[test]
    fn test_identity_scope_extension_all_node_combos() {
        // Device + untracked Node
        let ext = IdentityScopeExtension {
            subject_type: CertificateSubjectType::Node,
            blockchain_scope: hypermesh_lib::BlockchainScope::Device,
            tracked: false,
            workload_type: hypermesh_lib::WorkloadType::Node,
        };
        let bytes = ext.to_bytes();
        assert_eq!(bytes, [0, 0, 0, 0]);
        assert!(IdentityScopeExtension::from_bytes(&bytes).is_some());

        // Network + tracked Agent
        let ext2 = IdentityScopeExtension {
            subject_type: CertificateSubjectType::Agent,
            blockchain_scope: hypermesh_lib::BlockchainScope::Network,
            tracked: true,
            workload_type: hypermesh_lib::WorkloadType::Agent,
        };
        let bytes2 = ext2.to_bytes();
        assert_eq!(bytes2, [2, 1, 1, 2]);
        assert!(IdentityScopeExtension::from_bytes(&bytes2).is_some());
    }
}
