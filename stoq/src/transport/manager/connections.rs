// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport connection management - connect, accept, pool, shutdown

use anyhow::{Result, anyhow};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, debug};

use crate::transport::connection::{Connection, Endpoint};
use crate::transport::adaptive::AdaptiveConnection;

use super::StoqTransport;

impl StoqTransport {
    /// Connect to a remote endpoint with connection pooling for performance
    pub async fn connect(&self, endpoint: &Endpoint) -> Result<Arc<Connection>> {
        let pool_key = format!("{}:{}", endpoint.address, endpoint.port);

        // Try to reuse existing connection from pool for maximum performance
        if let Some(mut pool) = self.connection_pool.get_mut(&pool_key) {
            // Clean up unhealthy connections first
            pool.retain(|conn| conn.is_healthy());

            // Try to get a healthy connection
            if let Some(pooled_conn) = pool.pop() {
                debug!("Reusing pooled connection to [{}]:{}", endpoint.address, endpoint.port);
                pooled_conn.update_activity(); // Mark as recently used
                self.performance_stats.read().record_connection_reuse();
                return Ok(pooled_conn);
            }
        }

        debug!("Creating new connection to [{}]:{}", endpoint.address, endpoint.port);

        let socket_addr = endpoint.to_socket_addr();
        let connecting = self.endpoint.connect(socket_addr, endpoint.server_name.as_deref().unwrap_or("localhost"))?;

        let quinn_conn = connecting.await?;

        let quinn_conn_arc = Arc::new(quinn_conn);

        let connection = Arc::new(Connection::new_optimized(
            quinn_conn_arc.as_ref().clone(),
            endpoint.clone(),
            self.metrics.clone(),
            self.memory_pool.clone(),
            self.config.frame_batch_size,
            self.config.connection_idle_timeout,
        ));

        let conn_id = connection.id();
        self.connections.insert(conn_id.clone(), connection.clone());

        // Register connection with adaptation manager
        self.adaptation_manager.register_connection(conn_id.clone(), quinn_conn_arc.clone());

        // Create and store adaptive connection wrapper
        let adaptive_conn = Arc::new(AdaptiveConnection::new(quinn_conn_arc));
        self.adaptive_connections.insert(conn_id, adaptive_conn);

        self.metrics.record_connection_established();

        info!("Connected to {} with adaptive optimization (pool_size={})", socket_addr, self.config.connection_pool_size);
        Ok(connection)
    }

    /// Return connection to pool for reuse with LRU eviction
    pub fn return_to_pool(&self, connection: Arc<Connection>) {
        if !connection.is_active() {
            return; // Don't pool inactive connections
        }

        let pool_key = format!("{}:{}", connection.endpoint().address, connection.endpoint().port);
        let mut pool = self.connection_pool.entry(pool_key).or_insert_with(Vec::new);

        // Update activity before returning to pool
        connection.update_activity();

        if pool.len() >= self.config.connection_pool_size {
            // Pool is full, need to evict LRU connection
            // Find the least recently used connection
            let mut lru_idx = 0;
            let mut oldest_time = u64::MAX;

            for (idx, conn) in pool.iter().enumerate() {
                let activity = conn.last_activity();
                if activity < oldest_time {
                    oldest_time = activity;
                    lru_idx = idx;
                }
            }

            // Remove the LRU connection
            pool.remove(lru_idx);
            self.performance_stats.read().record_pool_eviction();
        }

        // Add the new connection
        pool.push(connection);
    }

    /// Clean up unhealthy connections from all pools
    pub fn cleanup_unhealthy_connections(&self) {
        let mut total_removed = 0;
        let mut total_remaining = 0;

        // Track that we're doing a health check
        self.performance_stats.read().record_health_check();

        for mut entry in self.connection_pool.iter_mut() {
            let pool_key = entry.key().clone();
            let pool = entry.value_mut();

            // Remove unhealthy connections
            let initial_size = pool.len();
            pool.retain(|conn| conn.is_healthy());
            let removed = initial_size - pool.len();

            if removed > 0 {
                debug!("Removed {} unhealthy connections from pool {}", removed, pool_key);
                total_removed += removed;
            }
            total_remaining += pool.len();
        }

        if total_removed > 0 {
            info!("Health check removed {} unhealthy connections, {} remaining in pools",
                  total_removed, total_remaining);
            self.performance_stats.read().record_unhealthy_removed(total_removed);
        }
    }

    /// Accept incoming connections
    pub async fn accept(&self) -> Result<Arc<Connection>> {
        let incoming = self.endpoint.accept().await.ok_or_else(|| anyhow!("No incoming connection"))?;
        let quinn_conn = incoming.await?;

        let remote_addr = quinn_conn.remote_address();
        let endpoint = Endpoint::new(
            match remote_addr {
                SocketAddr::V6(addr) => *addr.ip(),
                SocketAddr::V4(_) => return Err(anyhow!("IPv4 connections are not supported - STOQ is IPv6-only")),
            },
            remote_addr.port(),
        );

        let connection = Arc::new(Connection::new_optimized(
            quinn_conn,
            endpoint,
            self.metrics.clone(),
            self.memory_pool.clone(),
            self.config.frame_batch_size,
            self.config.connection_idle_timeout,
        ));

        self.connections.insert(connection.id(), connection.clone());
        self.metrics.record_connection_established();

        info!("Accepted connection from {}", remote_addr);
        Ok(connection)
    }

    /// Close all connections and connection pools
    pub async fn shutdown(&self) {
        info!("Shutting down STOQ transport");

        // Close all active connections
        for conn in self.connections.iter() {
            conn.close();
        }
        self.connections.clear();

        // Clear connection pools
        self.connection_pool.clear();

        // Close endpoint
        self.endpoint.close(0u32.into(), b"shutdown");

        info!("STOQ transport shutdown complete");
    }
}
