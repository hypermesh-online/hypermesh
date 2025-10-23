//! Transport metrics collection

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

pub struct TransportMetrics {
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    connections_established: AtomicU64,
    start_time: Instant,
}

impl TransportMetrics {
    pub fn new() -> Self {
        Self {
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            connections_established: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }
    
    pub fn record_bytes_sent(&self, bytes: usize) {
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
    }
    
    pub fn record_bytes_received(&self, bytes: usize) {
        self.bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
    }
    
    pub fn record_connection_established(&self) {
        self.connections_established.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_stats(&self, active_connections: usize) -> crate::TransportStats {
        let bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        let bytes_received = self.bytes_received.load(Ordering::Relaxed);
        let total_connections = self.connections_established.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs() as f64;
        
        let throughput_gbps = if elapsed > 0.0 {
            ((bytes_sent + bytes_received) as f64 * 8.0) / (elapsed * 1_000_000_000.0)
        } else {
            0.0
        };
        
        crate::TransportStats {
            bytes_sent,
            bytes_received,
            active_connections,
            total_connections,
            throughput_gbps,
            avg_latency_us: 500, // Placeholder
        }
    }
}