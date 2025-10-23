//! Metrics collection and reporting for Nexus components

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Thread-safe metrics collector
#[derive(Debug)]
pub struct MetricsCollector {
    counters: Arc<HashMap<String, AtomicU64>>,
    gauges: Arc<HashMap<String, AtomicU64>>,
    histograms: Arc<HashMap<String, Histogram>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(HashMap::new()),
            gauges: Arc::new(HashMap::new()),
            histograms: Arc::new(HashMap::new()),
        }
    }

    /// Increment a counter
    pub fn increment_counter(&self, name: &str, value: u64) {
        if let Some(counter) = self.counters.get(name) {
            counter.fetch_add(value, Ordering::Relaxed);
        }
    }

    /// Set a gauge value
    pub fn set_gauge(&self, name: &str, value: u64) {
        if let Some(gauge) = self.gauges.get(name) {
            gauge.store(value, Ordering::Relaxed);
        }
    }

    /// Record a histogram value
    pub fn record_histogram(&self, name: &str, value: Duration) {
        if let Some(histogram) = self.histograms.get(name) {
            histogram.record(value);
        }
    }

    /// Get counter value
    pub fn get_counter(&self, name: &str) -> u64 {
        self.counters
            .get(name)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get gauge value
    pub fn get_gauge(&self, name: &str) -> u64 {
        self.gauges
            .get(name)
            .map(|g| g.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple histogram implementation for latency tracking
#[derive(Debug)]
pub struct Histogram {
    samples: parking_lot::Mutex<Vec<u64>>,
    count: AtomicU64,
    sum: AtomicU64,
}

impl Histogram {
    pub fn new() -> Self {
        Self {
            samples: parking_lot::Mutex::new(Vec::new()),
            count: AtomicU64::new(0),
            sum: AtomicU64::new(0),
        }
    }

    pub fn record(&self, duration: Duration) {
        let micros = duration.as_micros() as u64;
        
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(micros, Ordering::Relaxed);
        
        let mut samples = self.samples.lock();
        samples.push(micros);
        
        // Keep only last 1000 samples for percentile calculation
        if samples.len() > 1000 {
            samples.remove(0);
        }
    }

    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    pub fn sum(&self) -> u64 {
        self.sum.load(Ordering::Relaxed)
    }

    pub fn average(&self) -> f64 {
        let count = self.count();
        if count == 0 {
            0.0
        } else {
            self.sum() as f64 / count as f64
        }
    }

    pub fn percentile(&self, p: f64) -> u64 {
        let mut samples = self.samples.lock();
        if samples.is_empty() {
            return 0;
        }
        
        samples.sort_unstable();
        let index = ((samples.len() - 1) as f64 * p / 100.0) as usize;
        samples[index]
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer helper for measuring operation duration
pub struct Timer {
    start: Instant,
    name: String,
    collector: Arc<MetricsCollector>,
}

impl Timer {
    pub fn new(name: String, collector: Arc<MetricsCollector>) -> Self {
        Self {
            start: Instant::now(),
            name,
            collector,
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.collector.record_histogram(&self.name, duration);
    }
}

/// Common metrics for Nexus components
pub mod common {
    pub const CONNECTIONS_ACTIVE: &str = "connections.active";
    pub const CONNECTIONS_TOTAL: &str = "connections.total";
    pub const MESSAGES_SENT: &str = "messages.sent";
    pub const MESSAGES_RECEIVED: &str = "messages.received";
    pub const BYTES_SENT: &str = "bytes.sent";
    pub const BYTES_RECEIVED: &str = "bytes.received";
    pub const ERRORS_TOTAL: &str = "errors.total";
    pub const REQUEST_DURATION: &str = "request.duration";
    pub const MEMORY_USAGE: &str = "memory.usage";
    pub const CPU_USAGE: &str = "cpu.usage";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        
        collector.increment_counter("test", 5);
        collector.set_gauge("memory", 1024);
        
        assert_eq!(collector.get_counter("test"), 5);
        assert_eq!(collector.get_gauge("memory"), 1024);
    }

    #[test] 
    fn test_histogram() {
        let hist = Histogram::new();
        
        hist.record(Duration::from_millis(100));
        hist.record(Duration::from_millis(200));
        hist.record(Duration::from_millis(300));
        
        assert_eq!(hist.count(), 3);
        assert_eq!(hist.average(), 200_000.0); // microseconds
    }
}