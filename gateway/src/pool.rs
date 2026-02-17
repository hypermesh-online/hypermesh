// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::{anyhow, Result};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use h3::client::SendRequest;
use bytes::Bytes;
use quinn::{ClientConfig, Endpoint};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Connection pool for managing HTTP/3 connections to backend servers
#[derive(Clone)]
pub struct ConnectionPool {
    endpoint: Endpoint,
    backend_addr: SocketAddr,
    connections: Arc<DashMap<usize, PooledConnection>>,
    next_id: Arc<AtomicUsize>,
    max_connections: usize,
    idle_timeout: Duration,
    stats: Arc<PoolStats>,
}

/// Statistics for the connection pool
#[derive(Default)]
pub struct PoolStats {
    total_connections: AtomicU64,
    active_connections: AtomicUsize,
    failed_connections: AtomicU64,
    requests_served: AtomicU64,
}

/// A pooled connection with metadata
struct PooledConnection {
    connection: Arc<ArcSwap<Option<SendRequest<h3_quinn::OpenStreams, Bytes>>>>,
    last_used: Arc<ArcSwap<Instant>>,
    created_at: Instant,
    request_count: Arc<AtomicU64>,
    healthy: Arc<ArcSwap<bool>>,
}

impl ConnectionPool {
    /// Create a new connection pool for a backend server
    pub async fn new(
        backend_addr: SocketAddr,
        max_connections: usize,
        idle_timeout: Duration,
    ) -> Result<Self> {
        // Create client configuration
        let mut roots = rustls::RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs().certs {
            roots.add(cert)?;
        }

        let mut tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();

        tls_config.alpn_protocols = vec![b"h3".to_vec()];

        let client_config = ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?
        ));

        // Create endpoint
        let mut endpoint = Endpoint::client("[::]:0".parse()?)?;
        endpoint.set_default_client_config(client_config);

        let pool = Self {
            endpoint,
            backend_addr,
            connections: Arc::new(DashMap::new()),
            next_id: Arc::new(AtomicUsize::new(0)),
            max_connections,
            idle_timeout,
            stats: Arc::new(PoolStats::default()),
        };

        // Pre-warm the pool with initial connections
        pool.warm_pool().await;

        Ok(pool)
    }

    /// Warm the pool by creating initial connections
    async fn warm_pool(&self) {
        let initial_connections = self.max_connections / 2;
        for _ in 0..initial_connections {
            tokio::spawn({
                let pool = self.clone();
                async move {
                    if let Err(e) = pool.create_connection().await {
                        warn!("Failed to create initial connection: {}", e);
                    }
                }
            });
        }
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Result<SendRequest<h3_quinn::OpenStreams, Bytes>> {
        // Try to find a healthy, idle connection
        let now = Instant::now();

        for entry in self.connections.iter() {
            let conn = entry.value();
            let last_used = **conn.last_used.load();

            // Check if connection is healthy and not too old
            if **conn.healthy.load() && now.duration_since(last_used) < self.idle_timeout {
                if let Some(connection) = &**conn.connection.load() {
                    // Update last used time
                    conn.last_used.store(Arc::new(now));
                    conn.request_count.fetch_add(1, Ordering::Relaxed);
                    self.stats.requests_served.fetch_add(1, Ordering::Relaxed);

                    debug!("Reusing connection from pool");
                    return Ok(connection.clone());
                }
            }
        }

        // No healthy connection found, create a new one
        self.create_connection().await
    }

    /// Create a new connection to the backend
    async fn create_connection(&self) -> Result<SendRequest<h3_quinn::OpenStreams, Bytes>> {
        // Check if we've reached max connections
        let active = self.stats.active_connections.load(Ordering::Relaxed);
        if active >= self.max_connections {
            // Try to clean up old connections first
            self.cleanup_connections().await;

            // If still at max, return error
            if self.stats.active_connections.load(Ordering::Relaxed) >= self.max_connections {
                return Err(anyhow!("Connection pool at maximum capacity"));
            }
        }

        // Create new QUIC connection
        let quic_conn = self
            .endpoint
            .connect(self.backend_addr, "localhost")?
            .await?;

        // Create HTTP/3 connection
        let (mut driver, send_request) = h3::client::new(h3_quinn::Connection::new(quic_conn))
            .await?;

        // Spawn driver task
        tokio::spawn(async move {
            let _ = futures::future::poll_fn(|cx| driver.poll_close(cx)).await;
        });

        // Store in pool
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Instant::now();

        let pooled = PooledConnection {
            connection: Arc::new(ArcSwap::from_pointee(Some(send_request.clone()))),
            last_used: Arc::new(ArcSwap::from_pointee(now)),
            created_at: now,
            request_count: Arc::new(AtomicU64::new(1)),
            healthy: Arc::new(ArcSwap::from_pointee(true)),
        };

        self.connections.insert(id, pooled);
        self.stats.total_connections.fetch_add(1, Ordering::Relaxed);
        self.stats.active_connections.fetch_add(1, Ordering::Relaxed);

        info!("Created new connection to {:?}", self.backend_addr);
        Ok(send_request)
    }

    /// Clean up old or unhealthy connections
    async fn cleanup_connections(&self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for entry in self.connections.iter() {
            let (id, conn) = entry.pair();
            let last_used = **conn.last_used.load();

            // Remove if unhealthy or idle too long
            if !**conn.healthy.load() || now.duration_since(last_used) > self.idle_timeout * 2 {
                to_remove.push(*id);
            }
        }

        for id in to_remove {
            if let Some(_) = self.connections.remove(&id) {
                self.stats.active_connections.fetch_sub(1, Ordering::Relaxed);
                debug!("Removed stale connection {}", id);
            }
        }
    }

    /// Mark a connection as unhealthy
    pub fn mark_unhealthy(&self, _conn: &SendRequest<h3_quinn::OpenStreams, Bytes>) {
        // In a real implementation, we'd track which specific connection failed
        // For now, we'll just increment the failed counter
        self.stats.failed_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStatus {
        PoolStatus {
            total_connections: self.stats.total_connections.load(Ordering::Relaxed),
            active_connections: self.stats.active_connections.load(Ordering::Relaxed),
            failed_connections: self.stats.failed_connections.load(Ordering::Relaxed),
            requests_served: self.stats.requests_served.load(Ordering::Relaxed),
        }
    }

    /// Check if the backend is healthy by attempting a connection
    pub async fn health_check(&self) -> Result<Duration> {
        let start = Instant::now();

        match self.get_connection().await {
            Ok(_conn) => {
                let latency = start.elapsed();
                Ok(latency)
            }
            Err(e) => {
                error!("Health check failed for {:?}: {}", self.backend_addr, e);
                Err(e)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub total_connections: u64,
    pub active_connections: usize,
    pub failed_connections: u64,
    pub requests_served: u64,
}