// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ + Proof of State Protocol Integration
//!
//! This module implements protocol-level integration of Proof of State validation
//! with STOQ transport, enabling intelligent protocol behavior based on network type.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use parking_lot::RwLock;
use dashmap::DashMap;
use tracing::{debug, info, warn, error};
use bytes::Bytes;

use super::pos_validator::{PosToken, PosTokenValidator, ValidationResult};
use crate::transport::certificate_strategy::NetworkType;
use crate::extensions::PacketToken;

/// Privacy tier for protocol-level enforcement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyTier {
    /// No validation, no logging, ephemeral
    Anonymous,
    /// Peer-only validation, minimal logging
    P2P,
    /// Federation-scoped validation and logging
    Federated,
    /// Full PoS validation, full transparency
    Public,
}

impl From<&NetworkType> for PrivacyTier {
    fn from(network_type: &NetworkType) -> Self {
        match network_type {
            NetworkType::Anonymous => PrivacyTier::Anonymous,
            NetworkType::P2P => PrivacyTier::P2P,
            NetworkType::Federated { .. } => PrivacyTier::Federated,
            NetworkType::Public => PrivacyTier::Public,
        }
    }
}

impl PrivacyTier {
    /// Check if this tier requires PoS validation
    pub fn requires_pos_validation(&self) -> bool {
        matches!(self, PrivacyTier::Public)
    }

    /// Check if this tier allows logging
    pub fn allows_logging(&self) -> bool {
        !matches!(self, PrivacyTier::Anonymous)
    }

    /// Check if this tier requires full 4-proof validation
    pub fn requires_full_proofs(&self) -> bool {
        matches!(self, PrivacyTier::Public)
    }

    /// Get connection timeout for this tier
    pub fn connection_timeout(&self) -> Duration {
        match self {
            PrivacyTier::Anonymous => Duration::from_secs(30),
            PrivacyTier::P2P => Duration::from_secs(60),
            PrivacyTier::Federated => Duration::from_secs(120),
            PrivacyTier::Public => Duration::from_secs(300),
        }
    }
}

/// Matrix position for shard addressing
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatrixPosition {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl MatrixPosition {
    pub fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    pub fn origin() -> Self {
        Self { x: 0, y: 0, z: 0 }
    }

    /// Calculate euclidean distance to another position
    pub fn distance_to(&self, other: &MatrixPosition) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        let dz = (self.z - other.z) as f64;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Shard address with matrix position
#[derive(Debug, Clone)]
pub struct ShardAddress {
    /// Shard identifier
    pub shard_id: u32,
    /// Matrix position for this shard
    pub position: MatrixPosition,
    /// Network ID where shard is stored
    pub network_id: String,
    /// Node ID at this position (if known)
    pub node_id: Option<String>,
}

/// Asset verification information
#[derive(Debug, Clone)]
pub struct AssetVerification {
    /// Asset identifier
    pub asset_id: Vec<u8>,
    /// Content hash for verification
    pub content_hash: [u8; 32],
    /// Size in bytes
    pub size: u64,
    /// Verification timestamp
    pub verified_at: SystemTime,
}

/// Connection state with PoS validation
#[derive(Debug, Clone)]
struct ConnectionState {
    /// Connection ID
    pub connection_id: String,
    /// Privacy tier for this connection
    pub privacy_tier: PrivacyTier,
    /// Last validated PoS token (if applicable)
    pub last_pos_token: Option<PosToken>,
    /// Connection established time
    pub established_at: SystemTime,
    /// Last activity time
    pub last_activity: SystemTime,
    /// Number of validated packets
    pub packet_count: u64,
}

/// STOQ + PoS integration manager
pub struct StoqPosIntegration {
    /// PoS token validator
    pos_validator: Arc<PosTokenValidator>,

    /// Connection states by connection ID
    connection_states: Arc<DashMap<String, ConnectionState>>,

    /// Asset verification cache
    asset_cache: Arc<DashMap<Vec<u8>, AssetVerification>>,

    /// Shard address registry (shard_id -> address)
    shard_registry: Arc<DashMap<u32, ShardAddress>>,

    /// Privacy tier configuration
    default_tier: RwLock<PrivacyTier>,
}

impl StoqPosIntegration {
    /// Create new STOQ + PoS integration
    pub fn new(cache_ttl: Duration) -> Self {
        Self {
            pos_validator: Arc::new(PosTokenValidator::new(cache_ttl)),
            connection_states: Arc::new(DashMap::new()),
            asset_cache: Arc::new(DashMap::new()),
            shard_registry: Arc::new(DashMap::new()),
            default_tier: RwLock::new(PrivacyTier::Public), // Default to most secure
        }
    }

    /// Set default privacy tier
    pub fn set_default_tier(&self, tier: PrivacyTier) {
        *self.default_tier.write() = tier;
    }

    /// Validate connection based on network type
    pub async fn validate_connection(
        &self,
        connection_id: String,
        network_type: &NetworkType,
        pos_token: Option<&PosToken>,
    ) -> Result<bool> {
        let privacy_tier = PrivacyTier::from(network_type);

        // Anonymous connections always succeed without validation
        if privacy_tier == PrivacyTier::Anonymous {
            if privacy_tier.allows_logging() {
                debug!("Anonymous connection {} established (no validation)", connection_id);
            }

            self.register_connection(connection_id, privacy_tier, None);
            return Ok(true);
        }

        // P2P and Federated require connection-level validation (handled by certificate strategy)
        if privacy_tier == PrivacyTier::P2P || privacy_tier == PrivacyTier::Federated {
            info!("Connection {} established for {:?} network", connection_id, privacy_tier);
            self.register_connection(connection_id, privacy_tier, None);
            return Ok(true);
        }

        // Public network requires full PoS validation
        if privacy_tier == PrivacyTier::Public {
            let token = pos_token.ok_or_else(||
                anyhow!("Public network requires PoS token")
            )?;

            let validation = self.pos_validator.validate_token(token)?;

            if !validation.is_valid {
                warn!(
                    "PoS validation failed for connection {}: {:?}",
                    connection_id,
                    validation.errors
                );
                return Ok(false);
            }

            info!(
                "Connection {} validated with PoS (validation time: {:?})",
                connection_id,
                validation.validation_time
            );

            self.register_connection(connection_id, privacy_tier, Some(token.clone()));
            return Ok(true);
        }

        Ok(false)
    }

    /// Register a new connection
    fn register_connection(
        &self,
        connection_id: String,
        privacy_tier: PrivacyTier,
        pos_token: Option<PosToken>,
    ) {
        let now = SystemTime::now();

        let state = ConnectionState {
            connection_id: connection_id.clone(),
            privacy_tier,
            last_pos_token: pos_token,
            established_at: now,
            last_activity: now,
            packet_count: 0,
        };

        self.connection_states.insert(connection_id, state);
    }

    /// Validate asset hash at protocol level
    pub fn validate_asset_hash(
        &self,
        connection_id: &str,
        asset_id: &[u8],
        content_hash: &[u8; 32],
        data: &[u8],
    ) -> Result<bool> {
        // Get connection state
        let mut conn_state = self.connection_states
            .get_mut(connection_id)
            .ok_or_else(|| anyhow!("Unknown connection: {}", connection_id))?;

        // Update activity
        conn_state.last_activity = SystemTime::now();
        conn_state.packet_count += 1;

        // Check if logging is allowed
        if conn_state.privacy_tier.allows_logging() {
            debug!(
                "Validating asset hash for connection {} (tier: {:?})",
                connection_id,
                conn_state.privacy_tier
            );
        }

        // Compute hash of received data
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let computed_hash: [u8; 32] = hasher.finalize().into();

        // Compare hashes
        if &computed_hash != content_hash {
            if conn_state.privacy_tier.allows_logging() {
                warn!(
                    "Asset hash mismatch for connection {}: expected {:?}, got {:?}",
                    connection_id,
                    content_hash,
                    computed_hash
                );
            }
            return Ok(false);
        }

        // Cache verified asset
        let verification = AssetVerification {
            asset_id: asset_id.to_vec(),
            content_hash: *content_hash,
            size: data.len() as u64,
            verified_at: SystemTime::now(),
        };

        self.asset_cache.insert(asset_id.to_vec(), verification);

        if conn_state.privacy_tier.allows_logging() {
            debug!(
                "Asset hash validated for connection {} ({} bytes)",
                connection_id,
                data.len()
            );
        }

        Ok(true)
    }

    /// Register shard address for matrix-based distribution
    pub fn register_shard_address(
        &self,
        shard_id: u32,
        position: MatrixPosition,
        network_id: String,
        node_id: Option<String>,
    ) {
        let address = ShardAddress {
            shard_id,
            position,
            network_id,
            node_id,
        };

        self.shard_registry.insert(shard_id, address);

        debug!(
            "Registered shard {} at matrix position ({}, {}, {})",
            shard_id,
            position.x,
            position.y,
            position.z
        );
    }

    /// Get shard addresses for retrieval
    pub fn get_shard_addresses(&self, shard_ids: &[u32]) -> Vec<ShardAddress> {
        shard_ids
            .iter()
            .filter_map(|id| self.shard_registry.get(id).map(|entry| entry.clone()))
            .collect()
    }

    /// Calculate optimal shard positions using matrix topology
    pub fn calculate_shard_positions(
        &self,
        num_shards: usize,
        origin: MatrixPosition,
        min_distance: f64,
        max_distance: f64,
    ) -> Vec<MatrixPosition> {
        let mut positions = Vec::new();

        // Use golden ratio sphere packing algorithm
        let golden_ratio = 1.618033988749895;
        let angle_increment = 2.0 * std::f64::consts::PI / golden_ratio;

        for i in 0..num_shards {
            let t = i as f64 / num_shards as f64;
            let inclination = (1.0 - 2.0 * t).acos();
            let azimuth = angle_increment * i as f64;

            // Map to matrix coordinates with configurable distance
            let radius = min_distance + (max_distance - min_distance) * t;

            let x = origin.x + (radius * inclination.sin() * azimuth.cos()).round() as i64;
            let y = origin.y + (radius * inclination.sin() * azimuth.sin()).round() as i64;
            let z = origin.z + (radius * inclination.cos()).round() as i64;

            positions.push(MatrixPosition::new(x, y, z));
        }

        debug!(
            "Calculated {} shard positions (distance range: {:.1} - {:.1})",
            positions.len(),
            min_distance,
            max_distance
        );

        positions
    }

    /// Enforce privacy tier behavior for protocol operations
    pub fn enforce_privacy_tier(
        &self,
        connection_id: &str,
        operation: &str,
    ) -> Result<()> {
        let conn_state = self.connection_states
            .get(connection_id)
            .ok_or_else(|| anyhow!("Unknown connection: {}", connection_id))?;

        // Anonymous tier restrictions
        if conn_state.privacy_tier == PrivacyTier::Anonymous {
            // No persistent storage, no tracking
            if operation.contains("log") || operation.contains("store") {
                return Err(anyhow!(
                    "Operation '{}' not allowed for Anonymous connections",
                    operation
                ));
            }
        }

        // Public tier requirements
        if conn_state.privacy_tier == PrivacyTier::Public {
            // Require PoS token validation
            if conn_state.last_pos_token.is_none() {
                return Err(anyhow!(
                    "Public network operation '{}' requires PoS token",
                    operation
                ));
            }
        }

        Ok(())
    }

    /// Get connection statistics
    pub fn get_connection_stats(&self, connection_id: &str) -> Option<ConnectionStats> {
        self.connection_states.get(connection_id).map(|state| {
            ConnectionStats {
                connection_id: state.connection_id.clone(),
                privacy_tier: state.privacy_tier,
                established_at: state.established_at,
                last_activity: state.last_activity,
                packet_count: state.packet_count,
                has_pos_token: state.last_pos_token.is_some(),
            }
        })
    }

    /// Get overall statistics
    pub fn get_stats(&self) -> IntegrationStats {
        let mut stats = IntegrationStats::default();

        for entry in self.connection_states.iter() {
            stats.total_connections += 1;
            match entry.privacy_tier {
                PrivacyTier::Anonymous => stats.anonymous_connections += 1,
                PrivacyTier::P2P => stats.p2p_connections += 1,
                PrivacyTier::Federated => stats.federated_connections += 1,
                PrivacyTier::Public => stats.public_connections += 1,
            }
        }

        stats.cached_assets = self.asset_cache.len();
        stats.registered_shards = self.shard_registry.len();

        // Get PoS validator metrics
        let pos_metrics = self.pos_validator.get_metrics();
        stats.pos_validations = pos_metrics.total_validations;
        stats.pos_cache_hits = pos_metrics.cache_hits;
        stats.pos_failures = pos_metrics.failed_validations;

        stats
    }

    /// Cleanup expired connections
    pub fn cleanup_expired_connections(&self) {
        let now = SystemTime::now();
        let mut removed = 0;

        self.connection_states.retain(|_, state| {
            let timeout = state.privacy_tier.connection_timeout();
            let is_active = now.duration_since(state.last_activity)
                .map(|d| d < timeout)
                .unwrap_or(false);

            if !is_active {
                removed += 1;
            }
            is_active
        });

        if removed > 0 {
            info!("Cleaned up {} expired connections", removed);
        }
    }

    /// Cleanup expired asset cache entries
    pub fn cleanup_expired_assets(&self, ttl: Duration) {
        let now = SystemTime::now();
        let mut removed = 0;

        self.asset_cache.retain(|_, verification| {
            let is_valid = now.duration_since(verification.verified_at)
                .map(|d| d < ttl)
                .unwrap_or(false);

            if !is_valid {
                removed += 1;
            }
            is_valid
        });

        if removed > 0 {
            debug!("Cleaned up {} expired asset cache entries", removed);
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub connection_id: String,
    pub privacy_tier: PrivacyTier,
    pub established_at: SystemTime,
    pub last_activity: SystemTime,
    pub packet_count: u64,
    pub has_pos_token: bool,
}

/// Integration statistics
#[derive(Debug, Clone, Default)]
pub struct IntegrationStats {
    pub total_connections: usize,
    pub anonymous_connections: usize,
    pub p2p_connections: usize,
    pub federated_connections: usize,
    pub public_connections: usize,
    pub cached_assets: usize,
    pub registered_shards: usize,
    pub pos_validations: u64,
    pub pos_cache_hits: u64,
    pub pos_failures: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::pos_validator::{ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime};

    fn create_test_pos_token() -> PosToken {
        PosToken {
            id: vec![1, 2, 3, 4],
            proof_of_space: ProofOfSpace {
                commitment_hash: vec![5, 6, 7, 8],
                matrix_position: (10, 20, 30),
                capacity: 1024 * 1024,
            },
            proof_of_stake: ProofOfStake {
                owner_pubkey: vec![9, 10, 11, 12],
                stake_amount: 1000,
                staked_until: SystemTime::now() + Duration::from_secs(3600),
            },
            proof_of_work: ProofOfWork {
                difficulty: 10,
                nonce: 12345,
                work_hash: vec![13, 14, 15, 16],
            },
            proof_of_time: ProofOfTime {
                timestamp: SystemTime::now(),
                sequence: 1,
                prev_hash: vec![17, 18, 19, 20],
            },
            signature: vec![21, 22, 23, 24],
            expires_at: SystemTime::now() + Duration::from_secs(300),
            issuer_pubkey: Some(vec![25, 26, 27, 28]),
        }
    }

    #[tokio::test]
    async fn test_anonymous_connection() {
        let integration = StoqPosIntegration::new(Duration::from_secs(300));

        let result = integration.validate_connection(
            "conn1".to_string(),
            &NetworkType::Anonymous,
            None,
        ).await.unwrap();

        assert!(result);
        assert_eq!(integration.connection_states.len(), 1);
    }

    #[tokio::test]
    async fn test_public_connection_with_pos() {
        let integration = StoqPosIntegration::new(Duration::from_secs(300));
        let token = create_test_pos_token();

        let result = integration.validate_connection(
            "conn1".to_string(),
            &NetworkType::Public,
            Some(&token),
        ).await.unwrap();

        assert!(result);

        let stats = integration.get_connection_stats("conn1").unwrap();
        assert_eq!(stats.privacy_tier, PrivacyTier::Public);
        assert!(stats.has_pos_token);
    }

    #[tokio::test]
    async fn test_public_connection_without_pos_fails() {
        let integration = StoqPosIntegration::new(Duration::from_secs(300));

        let result = integration.validate_connection(
            "conn1".to_string(),
            &NetworkType::Public,
            None,
        ).await;

        assert!(result.is_err());
    }

    #[test]
    fn test_asset_hash_validation() {
        let integration = StoqPosIntegration::new(Duration::from_secs(300));

        // Register connection first
        integration.register_connection(
            "conn1".to_string(),
            PrivacyTier::Public,
            None,
        );

        let data = b"test asset data";

        // Compute correct hash
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash: [u8; 32] = hasher.finalize().into();

        let result = integration.validate_asset_hash(
            "conn1",
            b"asset123",
            &hash,
            data,
        ).unwrap();

        assert!(result);
        assert_eq!(integration.asset_cache.len(), 1);
    }

    #[test]
    fn test_shard_address_registration() {
        let integration = StoqPosIntegration::new(Duration::from_secs(300));

        integration.register_shard_address(
            1,
            MatrixPosition::new(10, 20, 30),
            "network1".to_string(),
            Some("node1".to_string()),
        );

        integration.register_shard_address(
            2,
            MatrixPosition::new(50, 60, 70),
            "network2".to_string(),
            None,
        );

        let addresses = integration.get_shard_addresses(&[1, 2]);
        assert_eq!(addresses.len(), 2);
        assert_eq!(addresses[0].shard_id, 1);
        assert_eq!(addresses[1].shard_id, 2);
    }

    #[test]
    fn test_shard_position_calculation() {
        let integration = StoqPosIntegration::new(Duration::from_secs(300));

        let positions = integration.calculate_shard_positions(
            10,
            MatrixPosition::origin(),
            5.0,
            50.0,
        );

        assert_eq!(positions.len(), 10);

        // Verify positions are distributed
        for i in 0..positions.len() {
            for j in (i+1)..positions.len() {
                let dist = positions[i].distance_to(&positions[j]);
                assert!(dist > 0.0, "Positions should be distinct");
            }
        }
    }

    #[test]
    fn test_privacy_tier_enforcement() {
        let integration = StoqPosIntegration::new(Duration::from_secs(300));

        // Anonymous connection
        integration.register_connection(
            "anon".to_string(),
            PrivacyTier::Anonymous,
            None,
        );

        // Should reject logging operations
        let result = integration.enforce_privacy_tier("anon", "log_data");
        assert!(result.is_err());

        // Public connection without PoS token
        integration.register_connection(
            "public".to_string(),
            PrivacyTier::Public,
            None,
        );

        // Should require PoS token
        let result = integration.enforce_privacy_tier("public", "send_data");
        assert!(result.is_err());
    }

    #[test]
    fn test_statistics() {
        let integration = StoqPosIntegration::new(Duration::from_secs(300));

        integration.register_connection("c1".to_string(), PrivacyTier::Anonymous, None);
        integration.register_connection("c2".to_string(), PrivacyTier::P2P, None);
        integration.register_connection("c3".to_string(), PrivacyTier::Federated, None);
        integration.register_connection("c4".to_string(), PrivacyTier::Public, None);

        let stats = integration.get_stats();
        assert_eq!(stats.total_connections, 4);
        assert_eq!(stats.anonymous_connections, 1);
        assert_eq!(stats.p2p_connections, 1);
        assert_eq!(stats.federated_connections, 1);
        assert_eq!(stats.public_connections, 1);
    }
}
