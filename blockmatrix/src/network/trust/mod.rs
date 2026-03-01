// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Network Trust Architecture
//!
//! This module implements network-specific trust handlers for the four network types:
//! - Anonymous: No persistent identity, ephemeral connections
//! - P2P: Direct peer trust exchange without intermediary CA
//! - Federated: Federation gateway acts as trust anchor
//! - Public: Global CA with blockchain-registered certificates
//!
//! Each network maintains completely isolated trust models with no cross-network data leakage.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// Re-export handlers
pub mod anonymous;
pub mod federated;
pub mod p2p;
pub mod public;

#[cfg(test)]
mod tests;

pub use anonymous::AnonymousNetworkHandler;
pub use federated::FederatedNetworkHandler;
pub use p2p::P2PNetworkHandler;
pub use public::PublicNetworkHandler;

/// Network type determines trust model
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkType {
    /// No persistent identity, ephemeral everything
    Anonymous,
    /// Direct peer-to-peer with self-signed certificates
    P2P,
    /// Federation-specific trust anchor
    Federated { gateway_url: String },
    /// Global blockchain-registered certificates
    Public,
}

impl NetworkType {
    /// Get human-readable name
    pub fn name(&self) -> &str {
        match self {
            NetworkType::Anonymous => "Anonymous",
            NetworkType::P2P => "P2P",
            NetworkType::Federated { .. } => "Federated",
            NetworkType::Public => "Public",
        }
    }
}

/// Network identifier re-exported from hypermesh_lib (128-bit, UUID-compatible).
///
/// Construction:
///   - Random: `new_random_network_id()`
///   - From bytes: `NetworkId([u8; 16])`
///   - From UUID: `NetworkId(*uuid.as_bytes())`
pub use hypermesh_lib::NetworkId;

/// Create a new random NetworkId (replaces the old `NetworkId::new_v4()`).
pub fn new_random_network_id() -> NetworkId {
    NetworkId(*Uuid::new_v4().as_bytes())
}

/// Convert a NetworkId to a UUID (replaces the old `.as_uuid()` method).
pub fn network_id_as_uuid(id: &NetworkId) -> Uuid {
    Uuid::from_bytes(id.0)
}

/// Peer identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(String);

impl PeerId {
    /// Create from string
    pub fn new(id: String) -> Self {
        PeerId(id)
    }

    /// Create from certificate
    pub fn from_cert(cert: &Certificate) -> Self {
        // Use certificate fingerprint or subject as ID
        PeerId(cert.fingerprint.clone())
    }

    /// Get string representation
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Certificate abstraction for network trust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    /// Certificate subject (node identity)
    pub subject: String,
    /// Certificate issuer (CA or self for self-signed)
    pub issuer: String,
    /// Public key bytes
    pub public_key: Vec<u8>,
    /// Signature bytes
    pub signature: Vec<u8>,
    /// Certificate fingerprint (hash)
    pub fingerprint: String,
    /// Expiration timestamp
    pub expires_at: u64,
    /// Network type this certificate is for
    pub network_type: NetworkType,
    /// Whether registered on blockchain (for Public network)
    pub blockchain_registered: bool,
}

impl Certificate {
    /// Check if certificate is from specified issuer
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Check if certificate is blockchain registered
    pub fn is_blockchain_registered(&self) -> bool {
        self.blockchain_registered
    }

    /// Check if certificate is self-signed
    pub fn is_self_signed(&self) -> bool {
        self.subject == self.issuer
    }

    /// Check if certificate is expired
    pub fn is_expired(&self) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.expires_at < now
    }
}

/// Ephemeral key for anonymous connections
#[derive(Debug, Clone)]
pub struct EphemeralKey {
    /// Session ID
    pub session_id: Uuid,
    /// Public key bytes
    pub public_key: Vec<u8>,
    /// Private key bytes (zeroized on drop)
    private_key: Vec<u8>,
}

impl EphemeralKey {
    /// Create new ephemeral key with random key material.
    /// Uses random bytes until FALCON-1024 key generation is integrated.
    pub fn generate() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut public_key = vec![0u8; 32];
        rng.fill(public_key.as_mut_slice());
        let mut private_key = vec![0u8; 32];
        rng.fill(private_key.as_mut_slice());

        EphemeralKey {
            session_id: Uuid::new_v4(),
            public_key,
            private_key,
        }
    }
}

impl Drop for EphemeralKey {
    fn drop(&mut self) {
        // Zero out private key material
        self.private_key.iter_mut().for_each(|b| *b = 0);
    }
}

/// Asset request with network context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRequest {
    /// Asset ID being requested
    pub asset_id: String,
    /// Network context
    pub network_type: NetworkType,
    /// Requester peer ID (if applicable)
    pub peer_id: Option<PeerId>,
    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// Asset response with network validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetResponse {
    /// Asset ID
    pub asset_id: String,
    /// Asset data (if authorized)
    pub data: Option<Vec<u8>>,
    /// Authorization status
    pub authorized: bool,
    /// Response metadata
    pub metadata: HashMap<String, String>,
}

/// Peer information for validation
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer identifier
    pub peer_id: PeerId,
    /// Peer's network address
    pub address: String,
    /// Peer's certificate (if available)
    pub certificate: Option<Certificate>,
    /// Network type
    pub network_type: NetworkType,
}

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Network type to connect to
    pub network_type: NetworkType,
    /// Peer addresses for P2P mode
    pub peer_addresses: Vec<String>,
    /// Federation gateway URL for Federated mode
    pub federation_gateway: Option<String>,
    /// DNS name to register for Public mode
    pub dns_name: Option<String>,
    /// Proof of State for Public mode
    pub proof_of_state: Option<ProofOfState>,
}

/// Proof of State for blockchain registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfState {
    /// Proof of Space (WHERE)
    pub proof_of_space: Vec<u8>,
    /// Proof of Stake (WHO)
    pub proof_of_stake: Vec<u8>,
    /// Proof of Work (WHAT/HOW)
    pub proof_of_work: Vec<u8>,
    /// Proof of Time (WHEN)
    pub proof_of_time: Vec<u8>,
}

/// STOQ transport abstraction
#[derive(Clone)]
pub struct StoqTransport {
    /// Network type this transport is configured for
    network_type: NetworkType,
    /// Actual STOQ transport (placeholder)
    _inner: Arc<RwLock<Option<stoq::StoqTransport>>>,
}

impl std::fmt::Debug for StoqTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoqTransport")
            .field("network_type", &self.network_type)
            .field("connected", &"<transport>")
            .finish()
    }
}

impl StoqTransport {
    /// Create new STOQ transport for specific network type
    pub fn new_for_network(network_type: NetworkType) -> Result<Arc<Self>> {
        Ok(Arc::new(StoqTransport {
            network_type,
            _inner: Arc::new(RwLock::new(None)),
        }))
    }

    /// Exchange certificates with peer (for P2P mode)
    pub async fn exchange_certificate(
        &self,
        peer_addr: &str,
        _local_cert: &Certificate,
    ) -> Result<Certificate> {
        // Placeholder implementation
        // In production, would use STOQ protocol to exchange certificates
        Ok(Certificate {
            subject: format!("peer:{peer_addr}"),
            issuer: format!("peer:{peer_addr}"),
            public_key: vec![0; 32],
            signature: vec![0; 64],
            fingerprint: format!("fingerprint:{peer_addr}"),
            expires_at: 0,
            network_type: NetworkType::P2P,
            blockchain_registered: false,
        })
    }
}

/// Network connection with isolated context
#[derive(Clone, Debug)]
pub struct NetworkConnection {
    /// Unique network ID
    pub network_id: NetworkId,
    /// Network type
    pub network_type: NetworkType,
    /// STOQ transport instance
    pub stoq_transport: Arc<StoqTransport>,
    /// Certificate for this network (if applicable)
    pub certificate: Option<Certificate>,
}

/// Network handler trait for different network types
#[async_trait]
pub trait NetworkHandler: Send + Sync {
    /// Bootstrap the network connection
    async fn bootstrap(&self, config: NetworkConfig) -> Result<NetworkConnection>;

    /// Connect to the network
    async fn connect(&self) -> Result<()>;

    /// Validate a peer in this network's context
    async fn validate_peer(&self, peer: &PeerInfo) -> Result<bool>;

    /// Handle asset request with network-specific rules
    async fn handle_asset_request(&self, request: AssetRequest) -> Result<AssetResponse>;

    /// Disconnect from network
    async fn disconnect(&self) -> Result<()>;

    /// Network type identifier
    fn network_type(&self) -> NetworkType;
}

/// Generate ephemeral key for anonymous connections
pub fn generate_ephemeral_key() -> EphemeralKey {
    EphemeralKey::generate()
}

/// Request federation membership from gateway
pub async fn request_federation_membership(
    gateway_url: &str,
    _stoq: &Arc<StoqTransport>,
) -> Result<Certificate> {
    // Placeholder implementation
    // In production, would connect to federation gateway and request membership
    Ok(Certificate {
        subject: "federation-member".to_string(),
        issuer: gateway_url.to_string(),
        public_key: vec![0; 32],
        signature: vec![0; 64],
        fingerprint: format!("federation:{gateway_url}"),
        expires_at: 0,
        network_type: NetworkType::Federated {
            gateway_url: gateway_url.to_string(),
        },
        blockchain_registered: false,
    })
}

/// Request blockchain certificate from trust.hypermesh.online
pub async fn request_blockchain_certificate(
    _stoq: &Arc<StoqTransport>,
    _proof: &ProofOfState,
) -> Result<Certificate> {
    // Placeholder implementation
    // In production, would submit proof to trust.hypermesh.online
    Ok(Certificate {
        subject: "blockchain-node".to_string(),
        issuer: "trust.hypermesh.online".to_string(),
        public_key: vec![0; 32],
        signature: vec![0; 64],
        fingerprint: "blockchain:node".to_string(),
        expires_at: 0,
        network_type: NetworkType::Public,
        blockchain_registered: true,
    })
}

/// Register DNS name as blockchain asset
pub async fn register_dns_asset(_dns_name: &str, _cert: &Certificate) -> Result<()> {
    // Placeholder implementation
    // In production, would register DNS-as-Asset on blockchain
    Ok(())
}

// Placeholder for stoq module integration
mod stoq {
    pub struct StoqTransport;
    pub struct _Connection;
}
