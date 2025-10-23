//! Network metrics collection

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Summary of metrics for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub total_requests: u64,
    pub total_failures: u64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
}

/// Network-wide metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub total_requests: u64,
    pub total_failures: u64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub requests_per_second: f64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

impl NetworkMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            total_failures: 0,
            avg_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            requests_per_second: 0.0,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    pub fn record_request_success(&self) {
        // For static metrics, we'd need to track this differently
        // In a real implementation, this would update atomic counters
    }

    pub fn record_request_failure(&self) {
        // For static metrics, we'd need to track this differently
        // In a real implementation, this would update atomic counters
    }

    pub fn update_service_counts(&self, _services: usize) {
        // For static metrics, we'd need to track this differently
        // In a real implementation, this would update service count metrics
    }

    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            total_requests: self.total_requests,
            total_failures: self.total_failures,
            success_rate: if self.total_requests > 0 {
                (self.total_requests - self.total_failures) as f64 / self.total_requests as f64
            } else {
                0.0
            },
            avg_latency_ms: self.avg_latency_ms,
        }
    }
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-connection metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    pub requests: u64,
    pub failures: u64,
    pub latency_ms: f64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub last_activity: std::time::SystemTime,
}

/// Metrics collector
pub struct MetricsCollector {
    total_requests: Arc<AtomicU64>,
    total_failures: Arc<AtomicU64>,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            total_requests: Arc::new(AtomicU64::new(0)),
            total_failures: Arc::new(AtomicU64::new(0)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        }
    }
    
    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_failure(&self) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_bytes_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }
    
    pub fn record_bytes_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }
    
    pub fn get_metrics(&self) -> NetworkMetrics {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        
        NetworkMetrics {
            total_requests,
            total_failures: self.total_failures.load(Ordering::Relaxed),
            avg_latency_ms: 0.0, // Would need histogram for accurate latency
            p99_latency_ms: 0.0,
            requests_per_second: total_requests as f64 / elapsed,
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
        }
    }
}