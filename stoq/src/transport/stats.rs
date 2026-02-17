// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport Statistics and Performance Tracking

use std::sync::atomic::{AtomicU64, Ordering};

/// Connection pool statistics for monitoring
#[derive(Debug, Clone)]
pub struct ConnectionPoolStats {
    pub total_connections: usize,
    pub total_healthy: usize,
    pub pool_details: Vec<(String, usize, usize)>, // (endpoint, total, healthy)
    pub reuse_count: u64,
    pub eviction_count: u64,
    pub health_check_count: u64,
    pub unhealthy_removed: u64,
}

/// Performance statistics for transport monitoring
#[derive(Debug, Default)]
pub struct PerformanceStats {
    pub total_bytes_sent: AtomicU64,
    pub total_bytes_received: AtomicU64,
    pub peak_throughput_gbps: AtomicU64, // Stored as u64 * 1000 for precision
    pub zero_copy_operations: AtomicU64,
    pub frame_batches_sent: AtomicU64,
    pub memory_pool_hits: AtomicU64,
    pub memory_pool_misses: AtomicU64,
    pub connection_reuse_count: AtomicU64,
    pub connection_pool_evictions: AtomicU64,
    pub connection_health_checks: AtomicU64,
    pub unhealthy_connections_removed: AtomicU64,
}

impl PerformanceStats {
    /// Get peak throughput in Gbps
    pub fn peak_throughput(&self) -> f64 {
        self.peak_throughput_gbps.load(Ordering::Relaxed) as f64 / 1000.0
    }

    /// Update peak throughput
    pub fn update_peak_throughput(&self, throughput_gbps: f64) {
        let throughput_u64 = (throughput_gbps * 1000.0) as u64;
        let current_peak = self.peak_throughput_gbps.load(Ordering::Relaxed);
        if throughput_u64 > current_peak {
            self.peak_throughput_gbps.store(throughput_u64, Ordering::Relaxed);
        }
    }

    /// Increment connection reuse count
    pub fn record_connection_reuse(&self) {
        self.connection_reuse_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment pool eviction count
    pub fn record_pool_eviction(&self) {
        self.connection_pool_evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment health check count
    pub fn record_health_check(&self) {
        self.connection_health_checks.fetch_add(1, Ordering::Relaxed);
    }

    /// Record unhealthy connections removed
    pub fn record_unhealthy_removed(&self, count: usize) {
        self.unhealthy_connections_removed.fetch_add(count as u64, Ordering::Relaxed);
    }

    /// Increment zero-copy operations
    pub fn record_zero_copy(&self) {
        self.zero_copy_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment frame batches sent
    pub fn record_frame_batch(&self) {
        self.frame_batches_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Record memory pool hit
    pub fn record_pool_hit(&self) {
        self.memory_pool_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record memory pool miss
    pub fn record_pool_miss(&self) {
        self.memory_pool_misses.fetch_add(1, Ordering::Relaxed);
    }
}
