// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport monitoring, stats, metrics, and accessor methods

use std::sync::atomic::Ordering;
use std::sync::Arc;
use tracing::info;

use crate::transport::config::NetworkTier;
use crate::transport::stats::ConnectionPoolStats;

use crate::protocol::pos_fast_validator::PosFastValidator;
use crate::protocol::StoqPosIntegration;
use crate::protocol::StoqProtocolHandler;

use super::StoqTransport;

impl StoqTransport {
    /// Get transport statistics with performance metrics
    pub fn stats(&self) -> crate::TransportStats {
        let base_stats = self.metrics.get_stats(self.connections.len());

        // Add performance metrics
        let perf_stats = self.performance_stats.read();
        let (pool_available, pool_allocated) = self.memory_pool.stats();

        info!("Performance: {:.1} Gbps peak, Zero-copy ops: {}, Pool hits/misses: {}/{}, Frame batches: {}",
              perf_stats.peak_throughput(),
              perf_stats.zero_copy_operations.load(Ordering::Relaxed),
              perf_stats.memory_pool_hits.load(Ordering::Relaxed),
              perf_stats.memory_pool_misses.load(Ordering::Relaxed),
              perf_stats.frame_batches_sent.load(Ordering::Relaxed));

        info!(
            "Memory Pool Stats: Available buffers: {}, Allocated: {}",
            pool_available, pool_allocated
        );

        base_stats
    }

    /// Get active connections count
    pub fn active_connections(&self) -> usize {
        self.connections.len()
    }

    /// Get connection pool statistics for monitoring
    pub fn pool_stats(&self) -> ConnectionPoolStats {
        let mut total_connections = 0;
        let mut total_healthy = 0;
        let mut pool_details = Vec::new();

        for entry in self.connection_pool.iter() {
            let pool_key = entry.key().clone();
            let pool = entry.value();
            let pool_size = pool.len();
            let healthy_count = pool.iter().filter(|conn| conn.is_healthy()).count();

            total_connections += pool_size;
            total_healthy += healthy_count;
            pool_details.push((pool_key, pool_size, healthy_count));
        }

        let perf_stats = self.performance_stats.read();

        ConnectionPoolStats {
            total_connections,
            total_healthy,
            pool_details,
            reuse_count: perf_stats.connection_reuse_count.load(Ordering::Relaxed),
            eviction_count: perf_stats.connection_pool_evictions.load(Ordering::Relaxed),
            health_check_count: perf_stats.connection_health_checks.load(Ordering::Relaxed),
            unhealthy_removed: perf_stats
                .unhealthy_connections_removed
                .load(Ordering::Relaxed),
        }
    }

    /// Get transport performance statistics
    pub fn performance_stats(&self) -> (f64, u64, u64, u64) {
        let stats = self.performance_stats.read();
        let peak_gbps = stats.peak_throughput();
        let zero_copy_ops = stats.zero_copy_operations.load(Ordering::Relaxed);
        let pool_hits = stats.memory_pool_hits.load(Ordering::Relaxed);
        let frame_batches = stats.frame_batches_sent.load(Ordering::Relaxed);

        (peak_gbps, zero_copy_ops, pool_hits, frame_batches)
    }

    /// Get a rich transport snapshot including jitter, loss, and latency percentiles.
    ///
    /// This is the primary method for feeding transport data to engauge.
    pub fn get_transport_snapshot(&self) -> crate::transport::metrics::TransportSnapshot {
        self.metrics.get_transport_snapshot(self.connections.len())
    }

    /// Get detailed protocol metrics for monitoring
    pub fn get_protocol_metrics(&self) -> crate::transport::metrics::ProtocolMetrics {
        self.metrics.get_protocol_metrics()
    }

    /// Get interval-based metrics for rate calculations
    pub fn get_interval_metrics(&self) -> crate::transport::metrics::IntervalMetrics {
        self.metrics.get_interval_metrics()
    }

    /// Reset interval metrics for periodic reporting
    pub fn reset_interval_metrics(&self) {
        self.metrics.reset_interval_metrics();
    }

    /// Adapt transport configuration for detected network tier
    pub fn adapt_config_for_tier(&mut self, gbps: f64) {
        let tier = NetworkTier::from_gbps(gbps);
        self.config.adapt_for_network_tier(&tier);
        info!("Adapted STOQ configuration for network tier: {:?}", tier);
    }

    /// Get local address of the endpoint
    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.endpoint.local_addr()
    }

    /// Get the protocol handler
    pub fn protocol_handler(&self) -> &StoqProtocolHandler {
        &self.protocol_handler
    }

    /// Get access to the underlying Quinn endpoint for HTTP/3 integration
    /// This allows using STOQ's transport layer with h3 protocol implementations
    pub fn quinn_endpoint(&self) -> Arc<quinn::Endpoint> {
        self.endpoint.clone()
    }

    /// Get STOQ + PoS integration instance
    pub fn pos_integration(&self) -> &Arc<StoqPosIntegration> {
        &self.pos_integration
    }

    /// Get the fast PoS pre-validator for line-rate filtering
    pub fn pos_fast_validator(&self) -> &Arc<PosFastValidator> {
        &self.pos_fast_validator
    }
}
