// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Default implementation of IsolationManager
//!
//! Provides complete network isolation with:
//! - Per-network connection pools
//! - Packet origin tracking
//! - Boundary violation detection
//! - Comprehensive violation logging

use super::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Network-specific isolation configuration
#[derive(Debug, Clone)]
struct NetworkIsolationConfig {
    /// Network identifier
    _network_id: NetworkId,
    /// Network type (Anonymous, P2P, Federated, Public)
    _network_type: NetworkType,
    /// Isolated connection pool for this network
    _connection_pool: Arc<ConnectionPool>,
    /// Packet filter for boundary enforcement
    packet_filter: PacketFilter,
    /// Creation timestamp
    _created_at: Timestamp,
}

/// Packet filter for network boundary enforcement
#[derive(Debug, Clone)]
struct PacketFilter {
    /// Network this filter belongs to
    network_id: NetworkId,
    /// Allow list of destination networks (usually just self)
    allowed_destinations: HashSet<NetworkId>,
    /// Strict mode - reject any cross-boundary attempt
    strict_mode: bool,
}

impl PacketFilter {
    /// Create new packet filter for network
    fn new(network_id: NetworkId) -> Self {
        let mut allowed_destinations = HashSet::new();
        allowed_destinations.insert(network_id);

        PacketFilter {
            network_id,
            allowed_destinations,
            strict_mode: true,
        }
    }

    /// Validate packet against filter rules
    fn validate(&self, packet: &Packet) -> Result<()> {
        // In strict mode, only allow same-network communication
        if self.strict_mode {
            if packet.source_network != self.network_id {
                return Err(anyhow!(
                    "Packet source mismatch: expected {}, got {}",
                    self.network_id,
                    packet.source_network
                ));
            }

            if packet.destination_network != self.network_id {
                return Err(anyhow!(
                    "Cross-network packet rejected: {} -> {}",
                    packet.source_network,
                    packet.destination_network
                ));
            }
        }

        // Check allowed destinations
        if !self
            .allowed_destinations
            .contains(&packet.destination_network)
        {
            return Err(anyhow!(
                "Destination network {} not in allowed list",
                packet.destination_network
            ));
        }

        Ok(())
    }
}

/// Default implementation of IsolationManager
pub struct DefaultIsolationManager {
    /// Network configurations
    network_configs: Arc<RwLock<HashMap<NetworkId, NetworkIsolationConfig>>>,

    /// Packet origin tracking
    packet_origins: Arc<RwLock<HashMap<IsolationPacketId, NetworkId>>>,

    /// Per-network connection pools
    connection_pools: Arc<RwLock<HashMap<NetworkId, Arc<ConnectionPool>>>>,

    /// Violation log
    violations: Arc<RwLock<Vec<IsolationViolation>>>,

    /// Statistics
    stats: Arc<RwLock<IsolationStats>>,

    /// Maximum violations to keep in memory
    max_violations: usize,
}

impl DefaultIsolationManager {
    /// Create new isolation manager
    pub fn new() -> Self {
        Self {
            network_configs: Arc::new(RwLock::new(HashMap::new())),
            packet_origins: Arc::new(RwLock::new(HashMap::new())),
            connection_pools: Arc::new(RwLock::new(HashMap::new())),
            violations: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(IsolationStats::default())),
            max_violations: 1000,
        }
    }

    /// Create with custom violation limit
    pub fn with_violation_limit(max_violations: usize) -> Self {
        Self {
            network_configs: Arc::new(RwLock::new(HashMap::new())),
            packet_origins: Arc::new(RwLock::new(HashMap::new())),
            connection_pools: Arc::new(RwLock::new(HashMap::new())),
            violations: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(IsolationStats::default())),
            max_violations,
        }
    }

    /// Record a violation
    async fn record_violation(&self, violation: IsolationViolation) {
        let mut violations = self.violations.write().await;
        let mut stats = self.stats.write().await;

        // Update statistics
        stats.violations_detected += 1;
        let violation_type_str = violation.violation_type.to_string();
        *stats
            .violations_by_type
            .entry(violation_type_str)
            .or_insert(0) += 1;

        // Add to violation log
        violations.push(violation.clone());

        // Trim if over limit
        let vlen = violations.len();
        if vlen > self.max_violations {
            violations.drain(0..vlen - self.max_violations);
        }

        warn!(
            "Isolation violation: {} from {} to {}",
            violation.violation_type, violation.source_network, violation.destination_network
        );
    }

    /// Update statistics for packet validation
    async fn update_packet_stats(&self, accepted: bool) {
        let mut stats = self.stats.write().await;
        stats.packets_validated += 1;
        if !accepted {
            stats.packets_rejected += 1;
        }
    }

    /// Validate network exists
    async fn validate_network_exists(&self, network_id: &NetworkId) -> Result<()> {
        let configs = self.network_configs.read().await;
        if !configs.contains_key(network_id) {
            return Err(anyhow!("Network {network_id} not configured"));
        }
        Ok(())
    }
}

#[async_trait]
impl IsolationManager for DefaultIsolationManager {
    async fn configure_network(
        &self,
        network_id: NetworkId,
        network_type: NetworkType,
    ) -> Result<()> {
        debug!(
            "Configuring isolation for network {} ({})",
            network_id,
            network_type.name()
        );

        // Check if network already configured
        {
            let configs = self.network_configs.read().await;
            if configs.contains_key(&network_id) {
                return Err(anyhow!("Network {network_id} already configured"));
            }
        }

        // Create isolated connection pool
        let connection_pool = Arc::new(ConnectionPool::new(network_id));

        // Create network configuration
        let config = NetworkIsolationConfig {
            _network_id: network_id,
            _network_type: network_type.clone(),
            _connection_pool: connection_pool.clone(),
            packet_filter: PacketFilter::new(network_id),
            _created_at: Utc::now(),
        };

        // Store configuration
        self.network_configs
            .write()
            .await
            .insert(network_id, config.clone());

        // Store connection pool
        self.connection_pools
            .write()
            .await
            .insert(network_id, connection_pool);

        // Update stats
        self.stats.write().await.active_networks += 1;

        info!(
            "Configured isolation for network {} (type: {})",
            network_id,
            network_type.name()
        );
        Ok(())
    }

    async fn remove_network(&self, network_id: NetworkId) -> Result<()> {
        debug!("Removing isolation for network {}", network_id);

        // Validate network exists
        self.validate_network_exists(&network_id).await?;

        // Remove network configuration
        self.network_configs.write().await.remove(&network_id);

        // Close and remove connection pool
        if let Some(pool) = self.connection_pools.write().await.remove(&network_id) {
            pool.close_all().await?;
        }

        // Clean up packet tracking
        let mut origins = self.packet_origins.write().await;
        origins.retain(|_, net_id| *net_id != network_id);

        // Update stats
        self.stats.write().await.active_networks -= 1;

        info!("Removed isolation for network {}", network_id);
        Ok(())
    }

    async fn validate_packet(&self, packet: &Packet) -> Result<()> {
        debug!(
            "Validating packet {} from {} to {}",
            packet.id, packet.source_network, packet.destination_network
        );

        // Check if packet crosses network boundary
        if packet.crosses_boundary() {
            // VIOLATION: Packet attempting to cross network boundary
            let violation = IsolationViolation {
                violation_type: ViolationType::CrossNetworkPacket,
                source_network: packet.source_network,
                destination_network: packet.destination_network,
                timestamp: Utc::now(),
                packet_id: Some(packet.id.clone()),
                details: format!("Packet {} attempted to cross network boundary", packet.id),
            };

            self.record_violation(violation).await;
            self.update_packet_stats(false).await;

            return Err(anyhow!(
                "Isolation violation: packet {} cannot cross network boundary ({} -> {})",
                packet.id,
                packet.source_network,
                packet.destination_network
            ));
        }

        // Get source network configuration
        let configs = self.network_configs.read().await;
        if let Some(config) = configs.get(&packet.source_network) {
            // Apply packet filter
            if let Err(e) = config.packet_filter.validate(packet) {
                self.update_packet_stats(false).await;
                return Err(e);
            }
        } else {
            self.update_packet_stats(false).await;
            return Err(anyhow!(
                "Source network {} not configured",
                packet.source_network
            ));
        }

        // Track packet origin
        self.packet_origins
            .write()
            .await
            .insert(packet.id.clone(), packet.source_network);

        self.update_packet_stats(true).await;
        debug!("Packet {} validated successfully", packet.id);
        Ok(())
    }

    async fn get_connection_pool(&self, network_id: NetworkId) -> Result<Arc<ConnectionPool>> {
        self.connection_pools
            .read()
            .await
            .get(&network_id)
            .cloned()
            .ok_or_else(|| anyhow!("No connection pool for network: {network_id}"))
    }

    async fn check_violations(&self) -> Vec<IsolationViolation> {
        self.violations.read().await.clone()
    }

    async fn clear_violations(&self) -> Result<()> {
        self.violations.write().await.clear();

        // Clear violation stats
        let mut stats = self.stats.write().await;
        stats.violations_detected = 0;
        stats.violations_by_type.clear();

        info!("Cleared violation history");
        Ok(())
    }

    async fn get_stats(&self) -> IsolationStats {
        let stats = self.stats.read().await.clone();

        // Update connection count
        let mut total_connections = 0;
        let pools = self.connection_pools.read().await;
        for pool in pools.values() {
            total_connections += pool.connection_count().await;
        }

        IsolationStats {
            total_connections,
            ..stats
        }
    }
}

impl Default for DefaultIsolationManager {
    fn default() -> Self {
        Self::new()
    }
}
