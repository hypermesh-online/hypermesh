//! Connection Pool Management for HyperMesh Transport

use std::sync::Arc;
use std::time::Duration;
use std::net::SocketAddr;
use std::collections::HashMap;
use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::time::{interval, Instant};
use tracing::{debug, warn, info};
use serde::{Serialize, Deserialize};

use super::config::ConnectionPoolConfig;
use super::error::{TransportError, Result};
use super::HyperMeshConnection;

/// STOQ connection (alias for HyperMeshConnection)
pub type StoqConnection = HyperMeshConnection;

/// Endpoint representation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Endpoint {
    /// Address
    pub address: SocketAddr,
    /// Node ID
    pub node_id: String,
}

/// Connection pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    /// Number of active connections
    pub active_connections: usize,
    /// Number of idle connections
    pub idle_connections: usize,
    /// Pool capacity utilization percentage
    pub utilization_percent: f64,
    /// Average connection age in seconds
    pub avg_connection_age_secs: f64,
    /// Number of connections created
    pub total_created: u64,
    /// Number of connections removed
    pub total_removed: u64,
    /// Number of connections reused
    pub total_reused: u64,
}

/// Connection pool entry with metadata
#[derive(Clone)]
struct PoolEntry {
    connection: Arc<HyperMeshConnection>,
    created_at: Instant,
    last_used: Arc<RwLock<Instant>>,
    use_count: Arc<RwLock<u64>>,
}

impl PoolEntry {
    fn new(connection: HyperMeshConnection) -> Self {
        let now = Instant::now();
        Self {
            connection: Arc::new(connection),
            created_at: now,
            last_used: Arc::new(RwLock::new(now)),
            use_count: Arc::new(RwLock::new(0)),
        }
    }

    fn touch(&self) {
        *self.last_used.write() = Instant::now();
        *self.use_count.write() += 1;
    }

    fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    fn idle_time(&self) -> Duration {
        self.last_used.read().elapsed()
    }

    fn is_expired(&self, max_age: Duration, max_idle: Duration) -> bool {
        self.age() > max_age || self.idle_time() > max_idle
    }
}

/// Connection pool for managing and reusing connections
pub struct ConnectionPool {
    config: ConnectionPoolConfig,
    connections: Arc<DashMap<SocketAddr, Vec<PoolEntry>>>,
    stats: Arc<RwLock<PoolStats>>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: ConnectionPoolConfig) -> Self {
        let pool = Self {
            config,
            connections: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(PoolStats {
                active_connections: 0,
                idle_connections: 0,
                utilization_percent: 0.0,
                avg_connection_age_secs: 0.0,
                total_created: 0,
                total_removed: 0,
                total_reused: 0,
            })),
        };

        // Start background maintenance task
        if pool.config.enable_reuse {
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                pool_clone.maintenance_task().await;
            });
        }

        pool
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self, addr: &SocketAddr) -> Option<HyperMeshConnection> {
        if !self.config.enable_reuse {
            return None;
        }

        let mut entry_to_use = None;

        // Find a reusable connection
        if let Some(mut entries) = self.connections.get_mut(addr) {
            entries.retain(|entry| {
                let conn = &entry.connection;
                conn.is_active() && !entry.is_expired(
                    self.config.max_connection_age,
                    self.config.idle_timeout,
                )
            });

            if let Some(entry) = entries.first() {
                entry.touch();
                entry_to_use = Some(entry.clone());

                // Update stats
                let mut stats = self.stats.write();
                stats.total_reused += 1;

                debug!("Reusing connection to {}", addr);
            }
        }

        entry_to_use.map(|entry| (*entry.connection).clone())
    }

    /// Add a connection to the pool
    pub async fn add_connection(&self, addr: SocketAddr, connection: HyperMeshConnection) {
        if !self.config.enable_reuse {
            return;
        }

        // Check pool size limit
        let total_connections = self.count_total_connections();
        if total_connections >= self.config.max_pool_size {
            warn!("Connection pool is full ({}/{}), not adding connection to {}",
                  total_connections, self.config.max_pool_size, addr);
            return;
        }

        let entry = PoolEntry::new(connection);

        self.connections.entry(addr)
            .or_insert_with(Vec::new)
            .push(entry);

        // Update stats
        let mut stats = self.stats.write();
        stats.total_created += 1;
        stats.idle_connections += 1;

        debug!("Added connection to pool for {}", addr);
    }

    /// Remove a specific connection from the pool
    pub async fn remove_connection(&self, addr: &SocketAddr, conn_id: &str) {
        if let Some(mut entries) = self.connections.get_mut(addr) {
            entries.retain(|entry| entry.connection.id() != conn_id);

            // Update stats
            let mut stats = self.stats.write();
            stats.total_removed += 1;

            debug!("Removed connection {} from pool", conn_id);
        }
    }

    /// Close all connections in the pool
    pub async fn close_all(&self) {
        info!("Closing all pooled connections");

        let connections: Vec<_> = self.connections.iter()
            .flat_map(|entry| entry.value().clone())
            .collect();

        for entry in connections {
            entry.connection.close();
        }

        self.connections.clear();

        // Update stats
        let mut stats = self.stats.write();
        stats.active_connections = 0;
        stats.idle_connections = 0;
        stats.utilization_percent = 0.0;
    }

    /// Get current pool statistics
    pub fn get_stats(&self) -> PoolStats {
        let stats = self.stats.read();
        stats.clone()
    }

    /// Count total connections in pool
    fn count_total_connections(&self) -> usize {
        self.connections.iter()
            .map(|entry| entry.value().len())
            .sum()
    }

    /// Background maintenance task
    async fn maintenance_task(&self) {
        let mut ticker = interval(self.config.health_check_interval);

        loop {
            ticker.tick().await;

            let mut total_age = 0.0;
            let mut total_count = 0;
            let mut removed_count = 0;

            // Clean up expired connections
            for mut entry in self.connections.iter_mut() {
                let addr = entry.key().clone();
                let initial_count = entry.value().len();

                entry.value_mut().retain(|pool_entry| {
                    let should_keep = pool_entry.connection.is_active() &&
                        !pool_entry.is_expired(
                            self.config.max_connection_age,
                            self.config.idle_timeout,
                        );

                    if should_keep {
                        total_age += pool_entry.age().as_secs_f64();
                        total_count += 1;
                    }

                    should_keep
                });

                removed_count += initial_count - entry.value().len();

                if entry.value().is_empty() {
                    drop(entry);
                    self.connections.remove(&addr);
                }
            }

            // Update statistics
            let mut stats = self.stats.write();
            stats.total_removed += removed_count as u64;
            stats.idle_connections = total_count;
            stats.avg_connection_age_secs = if total_count > 0 {
                total_age / total_count as f64
            } else {
                0.0
            };
            stats.utilization_percent = (total_count as f64 / self.config.max_pool_size as f64) * 100.0;

            if removed_count > 0 {
                debug!("Pool maintenance: removed {} expired connections", removed_count);
            }
        }
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            connections: Arc::clone(&self.connections),
            stats: Arc::clone(&self.stats),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_entry_lifecycle() {
        // Create a mock connection (would need proper mock in real tests)
        // For now, just test the pool entry logic

        let config = ConnectionPoolConfig::default();
        let pool = ConnectionPool::new(config);

        assert_eq!(pool.count_total_connections(), 0);
    }

    #[tokio::test]
    async fn test_pool_stats() {
        let config = ConnectionPoolConfig::default();
        let pool = ConnectionPool::new(config);

        let stats = pool.get_stats();
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.idle_connections, 0);
        assert_eq!(stats.total_created, 0);
    }
}