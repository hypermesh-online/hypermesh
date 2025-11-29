//! Native metrics types for HyperMesh monitoring

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Counter metric - monotonically increasing value
pub struct Counter {
    value: AtomicU64,
    labels: RwLock<Vec<(String, String)>>,
}

impl Counter {
    /// Create new counter
    pub fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
            labels: RwLock::new(Vec::new()),
        }
    }

    /// Increment counter by 1
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment counter by value
    pub fn inc_by(&self, val: u64) {
        self.value.fetch_add(val, Ordering::Relaxed);
    }

    /// Get current value
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Add label
    pub fn with_label(&self, key: String, value: String) -> &Self {
        self.labels.write().unwrap().push((key, value));
        self
    }
}

/// Gauge metric - can go up or down
pub struct Gauge {
    value: Arc<RwLock<f64>>,
    labels: RwLock<Vec<(String, String)>>,
}

impl Gauge {
    /// Create new gauge
    pub fn new() -> Self {
        Self {
            value: Arc::new(RwLock::new(0.0)),
            labels: RwLock::new(Vec::new()),
        }
    }

    /// Set gauge value
    pub fn set(&self, val: f64) {
        *self.value.write().unwrap() = val;
    }

    /// Increment gauge
    pub fn inc(&self) {
        *self.value.write().unwrap() += 1.0;
    }

    /// Decrement gauge
    pub fn dec(&self) {
        *self.value.write().unwrap() -= 1.0;
    }

    /// Add to gauge
    pub fn add(&self, val: f64) {
        *self.value.write().unwrap() += val;
    }

    /// Subtract from gauge
    pub fn sub(&self, val: f64) {
        *self.value.write().unwrap() -= val;
    }

    /// Get current value
    pub fn get(&self) -> f64 {
        *self.value.read().unwrap()
    }

    /// Add label
    pub fn with_label(&self, key: String, value: String) -> &Self {
        self.labels.write().unwrap().push((key, value));
        self
    }
}

/// Histogram metric - tracks distribution of values
pub struct Histogram {
    buckets: Vec<HistogramBucket>,
    sum: Arc<RwLock<f64>>,
    count: AtomicUsize,
    labels: RwLock<Vec<(String, String)>>,
}

struct HistogramBucket {
    upper_bound: f64,
    count: AtomicUsize,
}

impl Histogram {
    /// Create new histogram with default buckets
    pub fn new() -> Self {
        Self::with_buckets(vec![
            0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])
    }

    /// Create histogram with custom buckets
    pub fn with_buckets(buckets: Vec<f64>) -> Self {
        let buckets = buckets
            .into_iter()
            .map(|upper_bound| HistogramBucket {
                upper_bound,
                count: AtomicUsize::new(0),
            })
            .collect();

        Self {
            buckets,
            sum: Arc::new(RwLock::new(0.0)),
            count: AtomicUsize::new(0),
            labels: RwLock::new(Vec::new()),
        }
    }

    /// Observe a value
    pub fn observe(&self, val: f64) {
        // Update sum and count
        *self.sum.write().unwrap() += val;
        self.count.fetch_add(1, Ordering::Relaxed);

        // Update buckets
        for bucket in &self.buckets {
            if val <= bucket.upper_bound {
                bucket.count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Get histogram data
    pub fn get(&self) -> (Vec<(f64, usize)>, f64, usize) {
        let buckets: Vec<(f64, usize)> = self.buckets
            .iter()
            .map(|b| (b.upper_bound, b.count.load(Ordering::Relaxed)))
            .collect();

        let sum = *self.sum.read().unwrap();
        let count = self.count.load(Ordering::Relaxed);

        (buckets, sum, count)
    }

    /// Add label
    pub fn with_label(&self, key: String, value: String) -> &Self {
        self.labels.write().unwrap().push((key, value));
        self
    }

    /// Time a function and observe its duration
    pub fn time<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed().as_secs_f64();
        self.observe(duration);
        result
    }
}

/// Summary metric - similar to histogram but calculates quantiles
pub struct Summary {
    observations: Arc<RwLock<Vec<f64>>>,
    max_age: std::time::Duration,
    labels: RwLock<Vec<(String, String)>>,
}

impl Summary {
    /// Create new summary
    pub fn new() -> Self {
        Self {
            observations: Arc::new(RwLock::new(Vec::new())),
            max_age: std::time::Duration::from_secs(600), // 10 minutes default
            labels: RwLock::new(Vec::new()),
        }
    }

    /// Observe a value
    pub fn observe(&self, val: f64) {
        self.observations.write().unwrap().push(val);

        // TODO: Implement time-based cleanup
    }

    /// Calculate quantile
    pub fn quantile(&self, q: f64) -> Option<f64> {
        let mut observations = self.observations.read().unwrap().clone();
        if observations.is_empty() {
            return None;
        }

        observations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let index = ((observations.len() as f64 - 1.0) * q) as usize;
        Some(observations[index])
    }

    /// Get summary data
    pub fn get(&self) -> (Vec<(f64, f64)>, f64, usize) {
        let observations = self.observations.read().unwrap();
        let count = observations.len();

        if count == 0 {
            return (Vec::new(), 0.0, 0);
        }

        let sum: f64 = observations.iter().sum();

        // Calculate common quantiles
        let quantiles = vec![
            (0.5, self.quantile(0.5).unwrap_or(0.0)),
            (0.9, self.quantile(0.9).unwrap_or(0.0)),
            (0.95, self.quantile(0.95).unwrap_or(0.0)),
            (0.99, self.quantile(0.99).unwrap_or(0.0)),
        ];

        (quantiles, sum, count)
    }

    /// Add label
    pub fn with_label(&self, key: String, value: String) -> &Self {
        self.labels.write().unwrap().push((key, value));
        self
    }
}

/// Metric registry for managing all metrics
pub struct MetricRegistry {
    counters: RwLock<std::collections::HashMap<String, Arc<Counter>>>,
    gauges: RwLock<std::collections::HashMap<String, Arc<Gauge>>>,
    histograms: RwLock<std::collections::HashMap<String, Arc<Histogram>>>,
    summaries: RwLock<std::collections::HashMap<String, Arc<Summary>>>,
}

impl MetricRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(std::collections::HashMap::new()),
            gauges: RwLock::new(std::collections::HashMap::new()),
            histograms: RwLock::new(std::collections::HashMap::new()),
            summaries: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Register or get counter
    pub fn counter(&self, name: &str) -> Arc<Counter> {
        let mut counters = self.counters.write().unwrap();
        counters
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Counter::new()))
            .clone()
    }

    /// Register or get gauge
    pub fn gauge(&self, name: &str) -> Arc<Gauge> {
        let mut gauges = self.gauges.write().unwrap();
        gauges
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Gauge::new()))
            .clone()
    }

    /// Register or get histogram
    pub fn histogram(&self, name: &str) -> Arc<Histogram> {
        let mut histograms = self.histograms.write().unwrap();
        histograms
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Histogram::new()))
            .clone()
    }

    /// Register or get summary
    pub fn summary(&self, name: &str) -> Arc<Summary> {
        let mut summaries = self.summaries.write().unwrap();
        summaries
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Summary::new()))
            .clone()
    }

    /// Export all metrics as text
    pub fn export_text(&self) -> String {
        let mut output = String::new();

        // Export counters
        for (name, counter) in self.counters.read().unwrap().iter() {
            output.push_str(&format!("# TYPE {} counter\n", name));
            output.push_str(&format!("{} {}\n", name, counter.get()));
        }

        // Export gauges
        for (name, gauge) in self.gauges.read().unwrap().iter() {
            output.push_str(&format!("# TYPE {} gauge\n", name));
            output.push_str(&format!("{} {}\n", name, gauge.get()));
        }

        // Export histograms
        for (name, histogram) in self.histograms.read().unwrap().iter() {
            output.push_str(&format!("# TYPE {} histogram\n", name));
            let (buckets, sum, count) = histogram.get();

            for (upper_bound, bucket_count) in buckets {
                output.push_str(&format!("{}_bucket{{le=\"{}\"}} {}\n", name, upper_bound, bucket_count));
            }
            output.push_str(&format!("{}_sum {}\n", name, sum));
            output.push_str(&format!("{}_count {}\n", name, count));
        }

        // Export summaries
        for (name, summary) in self.summaries.read().unwrap().iter() {
            output.push_str(&format!("# TYPE {} summary\n", name));
            let (quantiles, sum, count) = summary.get();

            for (quantile, value) in quantiles {
                output.push_str(&format!("{}{{quantile=\"{}\"}} {}\n", name, quantile, value));
            }
            output.push_str(&format!("{}_sum {}\n", name, sum));
            output.push_str(&format!("{}_count {}\n", name, count));
        }

        output
    }
}

// Global metrics registry
lazy_static::lazy_static! {
    pub static ref METRICS: MetricRegistry = MetricRegistry::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new();
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.inc_by(41);
        assert_eq!(counter.get(), 42);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new();
        assert_eq!(gauge.get(), 0.0);

        gauge.set(3.14);
        assert_eq!(gauge.get(), 3.14);

        gauge.inc();
        assert_eq!(gauge.get(), 4.14);

        gauge.sub(1.14);
        assert_eq!(gauge.get(), 3.0);
    }

    #[test]
    fn test_histogram() {
        let histogram = Histogram::with_buckets(vec![1.0, 2.0, 5.0, 10.0]);

        histogram.observe(0.5);
        histogram.observe(1.5);
        histogram.observe(7.0);

        let (buckets, sum, count) = histogram.get();
        assert_eq!(count, 3);
        assert_eq!(sum, 9.0);
    }

    #[test]
    fn test_registry() {
        let registry = MetricRegistry::new();

        let counter = registry.counter("test_counter");
        counter.inc_by(10);

        let gauge = registry.gauge("test_gauge");
        gauge.set(3.14);

        let export = registry.export_text();
        assert!(export.contains("test_counter 10"));
        assert!(export.contains("test_gauge 3.14"));
    }
}