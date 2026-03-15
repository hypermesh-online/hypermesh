// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport PoS validation, shard addressing, and asset hash verification

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::transport::certificate_strategy::NetworkType;
use crate::transport::connection::PosValidationState;
use crate::transport::falcon;

use super::StoqTransport;

impl StoqTransport {
    /// Get FALCON transport for quantum-resistant operations
    pub fn falcon_transport(&self) -> Option<Arc<parking_lot::RwLock<falcon::FalconTransport>>> {
        self.falcon_transport.clone()
    }

    /// Sign data using FALCON quantum-resistant cryptography
    pub fn falcon_sign(&self, data: &[u8]) -> Result<Option<falcon::FalconSignature>> {
        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();
            Ok(Some(falcon_guard.sign_handshake_data(data)?))
        } else {
            Ok(None)
        }
    }

    /// Verify FALCON signature
    pub fn falcon_verify(
        &self,
        key_id: &str,
        signature: &falcon::FalconSignature,
        data: &[u8],
    ) -> Result<bool> {
        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();
            falcon_guard.verify_handshake_signature(key_id, signature, data)
        } else {
            Err(anyhow::anyhow!("FALCON transport not enabled"))
        }
    }

    /// Validate connection with PoS token (for public networks)
    ///
    /// After successful PoS validation, feeds the result to the eBPF layer
    /// so the XDP program can fast-path validated connections at kernel level.
    pub async fn validate_connection_with_pos(
        &self,
        connection_id: String,
        network_type: &NetworkType,
        pos_token: Option<&crate::protocol::PosToken>,
    ) -> Result<bool> {
        let is_valid = self
            .pos_integration
            .validate_connection(connection_id.clone(), network_type, pos_token)
            .await?;

        // Feed PoS validation result to eBPF so the XDP program can
        // cache the decision and fast-path future packets.
        if let Some(ref ebpf) = self.ebpf_transport {
            // Derive a content hash from the connection ID for eBPF keying.
            // In production this would use the PoS token's cryptographic hash.
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(connection_id.as_bytes());
            if let Some(token) = pos_token {
                hasher.update(&token.id);
            }
            let hash_bytes: [u8; 32] = hasher.finalize().into();
            let content_hash = hypermesh_lib::ContentHash::from_bytes(hash_bytes);

            if let Err(e) = ebpf
                .read()
                .inner()
                .set_pos_validation(content_hash, is_valid)
            {
                warn!("Failed to feed PoS validation to eBPF: {}", e);
            }
        }

        Ok(is_valid)
    }

    /// Perform bilateral PoS validation on a connection and update its state.
    ///
    /// This method MUST be called after the QUIC TLS handshake completes
    /// but BEFORE any application data is exchanged. It validates the
    /// peer's PoS token and marks the connection as authenticated or
    /// rejected. For Anonymous networks, the connection is marked as
    /// `NotRequired` (no PoS needed).
    ///
    /// Returns `true` if the connection passed PoS validation.
    pub async fn validate_and_gate_connection(
        &self,
        connection_id: &str,
        network_type: &NetworkType,
        pos_token: Option<&crate::protocol::PosToken>,
    ) -> Result<bool> {
        // Check if Anonymous — skip PoS
        if matches!(network_type, NetworkType::Anonymous) {
            if let Some(conn) = self.connections.get(connection_id) {
                conn.set_pos_state(PosValidationState::NotRequired);
            }
            debug!(
                "Connection {} is Anonymous — PoS not required",
                connection_id
            );
            return Ok(true);
        }

        // Perform PoS validation
        let is_valid = self
            .validate_connection_with_pos(
                connection_id.to_string(),
                network_type,
                pos_token,
            )
            .await?;

        // Update connection's PoS gate state
        if let Some(conn) = self.connections.get(connection_id) {
            if is_valid {
                conn.set_pos_state(PosValidationState::Validated);
                info!(
                    "Connection {} passed bilateral PoS validation",
                    connection_id
                );
            } else {
                conn.set_pos_state(PosValidationState::Rejected);
                warn!(
                    "Connection {} REJECTED by PoS validation — closing",
                    connection_id
                );
                conn.close();
            }
        }

        Ok(is_valid)
    }

    /// Register shard address for matrix-aware distribution
    pub fn register_shard_address(
        &self,
        shard_id: u32,
        position: crate::protocol::MatrixPosition,
        network_id: String,
        node_id: Option<String>,
    ) {
        self.pos_integration
            .register_shard_address(shard_id, position, network_id, node_id);
    }

    /// Get shard addresses for retrieval
    pub fn get_shard_addresses(&self, shard_ids: &[u32]) -> Vec<crate::protocol::ShardAddress> {
        self.pos_integration.get_shard_addresses(shard_ids)
    }

    /// Calculate optimal shard positions using matrix topology
    pub fn calculate_shard_positions(
        &self,
        num_shards: usize,
        origin: crate::protocol::MatrixPosition,
        min_distance: f64,
        max_distance: f64,
    ) -> Vec<crate::protocol::MatrixPosition> {
        self.pos_integration.calculate_shard_positions(
            num_shards,
            origin,
            min_distance,
            max_distance,
        )
    }

    /// Validate asset hash at protocol level
    pub fn validate_asset_hash(
        &self,
        connection_id: &str,
        asset_id: &[u8],
        content_hash: &[u8; 32],
        data: &[u8],
    ) -> Result<bool> {
        self.pos_integration
            .validate_asset_hash(connection_id, asset_id, content_hash, data)
    }

    /// Create a multi-path connection with this transport's PoS fast validator.
    ///
    /// The returned connection is pre-configured with the transport's
    /// validator for privacy-tier-aware path validation.
    pub fn create_multipath_connection(
        &self,
        peer_id: String,
        policy: crate::transport::multipath::PathPolicy,
    ) -> crate::transport::multipath::MultiPathConnection {
        crate::transport::multipath::MultiPathConnection::new(peer_id, policy)
            .with_pos_validator(self.pos_fast_validator.clone())
    }

    /// Get STOQ + PoS integration statistics
    pub fn get_pos_integration_stats(&self) -> crate::protocol::IntegrationStats {
        self.pos_integration.get_stats()
    }

    /// Cleanup expired connections and assets (call periodically).
    ///
    /// Also logs a note about eBPF stale state. The HyperMeshEbpf orchestrator
    /// currently has no bulk-clear API for PoS validations, so stale entries
    /// persist until overwritten. When a cleanup method is added to
    /// hypermesh-ebpf, call it here to evict entries whose connections expired.
    pub fn cleanup_expired(&self) {
        let conn_count_before = self.pos_integration.get_stats().total_connections;

        self.pos_integration.cleanup_expired_connections();
        self.pos_integration
            .cleanup_expired_assets(std::time::Duration::from_secs(3600)); // 1 hour TTL

        let conn_count_after = self.pos_integration.get_stats().total_connections;
        let removed = conn_count_before.saturating_sub(conn_count_after);

        if removed > 0 && self.ebpf_transport.is_some() {
            debug!(
                "Cleaned up {} expired connections; eBPF PoS cache entries \
                     may be stale until overwritten",
                removed
            );
        }
    }
}
