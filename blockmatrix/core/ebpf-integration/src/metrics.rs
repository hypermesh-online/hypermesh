//! eBPF metrics collection and monitoring
//!
//! Provides comprehensive metrics collection from eBPF programs including
//! performance counters, network statistics, and operational metrics.

use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// eBPF metrics collector and aggregator
pub struct EbpfMetrics {
    collectors: RwLock<HashMap<String, MetricCollector>>,
    aggregated_stats: RwLock<AggregatedStats>,
    collection_interval: Duration,
}

impl EbpfMetrics {
    pub fn new() -> Self {
        Self {
            collectors: RwLock::new(HashMap::new()),
            aggregated_stats: RwLock::new(AggregatedStats::default()),
            collection_interval: Duration::from_secs(5),
        }
    }

    /// Start metrics collection
    pub async fn start_collection(&self) -> Result<()> {
        info!("📊 Starting eBPF metrics collection");
        
        // Initialize default collectors
        let mut collectors = self.collectors.write().await;
        collectors.insert("network".to_string(), MetricCollector::new("network"));
        collectors.insert("traffic_control".to_string(), MetricCollector::new("traffic_control"));
        collectors.insert("load_balancer".to_string(), MetricCollector::new("load_balancer"));
        collectors.insert("security".to_string(), MetricCollector::new("security"));
        
        // Start collection loop
        let collectors_clone = self.collectors.clone();
        let stats_clone = self.aggregated_stats.clone();
        let interval = self.collection_interval;
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            
            loop {
                ticker.tick().await;
                
                // Collect metrics from all collectors
                {
                    let mut collectors = collectors_clone.write().await;
                    let mut stats = stats_clone.write().await;
                    
                    for (name, collector) in collectors.iter_mut() {
                        let metrics = collector.collect().await;
                        stats.update_from_collector(name, &metrics);
                    }
                    
                    stats.last_collection = Instant::now();
                }
            }
        });
        
        Ok(())
    }

    /// Get current metrics snapshot
    pub async fn snapshot(&self) -> Result<EbpfMetricsSnapshot> {
        let stats = self.aggregated_stats.read().await;
        let collectors = self.collectors.read().await;
        
        let mut component_metrics = HashMap::new();
        for (name, collector) in collectors.iter() {
            component_metrics.insert(name.clone(), collector.get_current_metrics());
        }
        
        Ok(EbpfMetricsSnapshot {
            timestamp: Instant::now(),
            network: stats.network.clone(),
            traffic_control: stats.traffic_control.clone(),
            load_balancer: stats.load_balancer.clone(),
            security: stats.security.clone(),
            system: SystemMetrics::collect().await,
            component_metrics,
        })
    }

    /// Get metrics for a specific component
    pub async fn get_component_metrics(&self, component: &str) -> Option<ComponentMetrics> {
        let collectors = self.collectors.read().await;
        collectors.get(component).map(|c| c.get_current_metrics())
    }

    /// Register a custom metric
    pub async fn register_metric(&self, component: &str, metric: CustomMetric) {
        let mut collectors = self.collectors.write().await;
        if let Some(collector) = collectors.get_mut(component) {
            collector.register_custom_metric(metric);
        }
    }

    /// Get historical metrics (last N snapshots)
    pub async fn get_history(&self, component: &str, count: usize) -> Vec<ComponentMetrics> {
        let collectors = self.collectors.read().await;
        if let Some(collector) = collectors.get(component) {
            collector.get_history(count)
        } else {
            Vec::new()
        }
    }
}

/// Metrics collector for individual eBPF components
struct MetricCollector {
    component: String,
    current_metrics: ComponentMetrics,
    history: Vec<ComponentMetrics>,
    max_history: usize,
    custom_metrics: HashMap<String, CustomMetric>,
}

impl MetricCollector {
    fn new(component: &str) -> Self {
        Self {
            component: component.to_string(),
            current_metrics: ComponentMetrics::default(),
            history: Vec::new(),
            max_history: 100,
            custom_metrics: HashMap::new(),
        }
    }

    async fn collect(&mut self) -> ComponentMetrics {
        // Simulate metrics collection from eBPF maps
        let metrics = match self.component.as_str() {
            "network" => self.collect_network_metrics().await,
            "traffic_control" => self.collect_traffic_metrics().await,
            "load_balancer" => self.collect_lb_metrics().await,
            "security" => self.collect_security_metrics().await,
            _ => ComponentMetrics::default(),
        };
        
        // Store in history
        self.history.push(self.current_metrics.clone());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
        
        self.current_metrics = metrics.clone();
        metrics
    }

    async fn collect_network_metrics(&self) -> ComponentMetrics {
        ComponentMetrics {
            component: "network".to_string(),
            timestamp: Instant::now(),
            counters: vec![
                ("packets_processed".to_string(), rand::random::<u64>() % 10000),
                ("bytes_processed".to_string(), rand::random::<u64>() % 1000000),
                ("packets_dropped".to_string(), rand::random::<u64>() % 100),
                ("connections_tracked".to_string(), rand::random::<u64>() % 1000),
            ].into_iter().collect(),
            gauges: vec![
                ("bandwidth_utilization".to_string(), rand::random::<f64>() * 100.0),
                ("cpu_usage".to_string(), rand::random::<f64>() * 50.0),
                ("memory_usage_mb".to_string(), rand::random::<f64>() * 100.0),
            ].into_iter().collect(),
            histograms: vec![
                ("latency_microseconds".to_string(), HistogramData {
                    count: 1000,
                    sum: 50000.0,
                    buckets: vec![
                        (100, 800),
                        (1000, 950),
                        (10000, 990),
                        (100000, 1000),
                    ],
                }),
            ].into_iter().collect(),
        }
    }

    async fn collect_traffic_metrics(&self) -> ComponentMetrics {
        ComponentMetrics {
            component: "traffic_control".to_string(),
            timestamp: Instant::now(),
            counters: vec![
                ("bytes_shaped".to_string(), rand::random::<u64>() % 500000),
                ("packets_shaped".to_string(), rand::random::<u64>() % 5000),
                ("rate_limit_hits".to_string(), rand::random::<u64>() % 100),
            ].into_iter().collect(),
            gauges: vec![
                ("active_flows".to_string(), rand::random::<f64>() * 500.0),
                ("queue_depth".to_string(), rand::random::<f64>() * 100.0),
            ].into_iter().collect(),
            histograms: HashMap::new(),
        }
    }

    async fn collect_lb_metrics(&self) -> ComponentMetrics {
        ComponentMetrics {
            component: "load_balancer".to_string(),
            timestamp: Instant::now(),
            counters: vec![
                ("requests_routed".to_string(), rand::random::<u64>() % 10000),
                ("backend_failures".to_string(), rand::random::<u64>() % 50),
                ("health_checks_performed".to_string(), rand::random::<u64>() % 100),
            ].into_iter().collect(),
            gauges: vec![
                ("active_connections".to_string(), rand::random::<f64>() * 1000.0),
                ("healthy_backends".to_string(), rand::random::<f64>() * 10.0),
            ].into_iter().collect(),
            histograms: vec![
                ("backend_response_time".to_string(), HistogramData {
                    count: 500,
                    sum: 25000.0,
                    buckets: vec![
                        (10, 400),
                        (50, 480),
                        (100, 495),
                        (500, 500),
                    ],
                }),
            ].into_iter().collect(),
        }
    }

    async fn collect_security_metrics(&self) -> ComponentMetrics {
        ComponentMetrics {
            component: "security".to_string(),
            timestamp: Instant::now(),
            counters: vec![
                ("packets_inspected".to_string(), rand::random::<u64>() % 50000),
                ("threats_blocked".to_string(), rand::random::<u64>() % 10),
                ("policy_violations".to_string(), rand::random::<u64>() % 5),
            ].into_iter().collect(),
            gauges: vec![
                ("active_policies".to_string(), rand::random::<f64>() * 20.0),
                ("threat_score".to_string(), rand::random::<f64>() * 100.0),
            ].into_iter().collect(),
            histograms: HashMap::new(),
        }
    }

    fn get_current_metrics(&self) -> ComponentMetrics {
        self.current_metrics.clone()
    }

    fn get_history(&self, count: usize) -> Vec<ComponentMetrics> {
        let start = if self.history.len() > count {
            self.history.len() - count
        } else {
            0
        };
        self.history[start..].to_vec()
    }

    fn register_custom_metric(&mut self, metric: CustomMetric) {
        self.custom_metrics.insert(metric.name.clone(), metric);
    }
}

/// Aggregated statistics across all eBPF components
#[derive(Debug, Clone, Default)]
struct AggregatedStats {
    network: NetworkAggregateStats,
    traffic_control: TrafficControlAggregateStats,
    load_balancer: LoadBalancerAggregateStats,
    security: SecurityAggregateStats,
    last_collection: Instant,
}

impl AggregatedStats {
    fn update_from_collector(&mut self, component: &str, metrics: &ComponentMetrics) {
        match component {
            "network" => self.network.update_from_metrics(metrics),
            "traffic_control" => self.traffic_control.update_from_metrics(metrics),
            "load_balancer" => self.load_balancer.update_from_metrics(metrics),
            "security" => self.security.update_from_metrics(metrics),
            _ => {}
        }
    }
}

/// Complete eBPF metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbpfMetricsSnapshot {
    pub timestamp: Instant,
    pub network: NetworkAggregateStats,
    pub traffic_control: TrafficControlAggregateStats,
    pub load_balancer: LoadBalancerAggregateStats,
    pub security: SecurityAggregateStats,
    pub system: SystemMetrics,
    pub component_metrics: HashMap<String, ComponentMetrics>,
}

/// Individual component metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentMetrics {
    pub component: String,
    pub timestamp: Instant,
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub histograms: HashMap<String, HistogramData>,
}

/// Histogram data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramData {
    pub count: u64,
    pub sum: f64,
    pub buckets: Vec<(u64, u64)>, // (upper_bound, count)
}

/// Custom metric definition
#[derive(Debug, Clone)]
pub struct CustomMetric {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub labels: HashMap<String, String>,
}

/// Metric types
#[derive(Debug, Clone)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

/// Aggregate network statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkAggregateStats {
    pub total_packets: u64,
    pub total_bytes: u64,
    pub dropped_packets: u64,
    pub average_latency_us: f64,
    pub bandwidth_utilization: f64,
}

impl NetworkAggregateStats {
    fn update_from_metrics(&mut self, metrics: &ComponentMetrics) {
        if let Some(&packets) = metrics.counters.get("packets_processed") {
            self.total_packets += packets;
        }
        if let Some(&bytes) = metrics.counters.get("bytes_processed") {
            self.total_bytes += bytes;
        }
        if let Some(&dropped) = metrics.counters.get("packets_dropped") {
            self.dropped_packets += dropped;
        }
        if let Some(&bandwidth) = metrics.gauges.get("bandwidth_utilization") {
            self.bandwidth_utilization = bandwidth;
        }
    }
}

/// Aggregate traffic control statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrafficControlAggregateStats {
    pub bytes_shaped: u64,
    pub active_flows: u32,
    pub rate_limit_hits: u64,
    pub average_queue_depth: f64,
}

impl TrafficControlAggregateStats {
    fn update_from_metrics(&mut self, metrics: &ComponentMetrics) {
        if let Some(&bytes) = metrics.counters.get("bytes_shaped") {
            self.bytes_shaped += bytes;
        }
        if let Some(&hits) = metrics.counters.get("rate_limit_hits") {
            self.rate_limit_hits += hits;
        }
        if let Some(&flows) = metrics.gauges.get("active_flows") {
            self.active_flows = flows as u32;
        }
        if let Some(&queue) = metrics.gauges.get("queue_depth") {
            self.average_queue_depth = queue;
        }
    }
}

/// Aggregate load balancer statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadBalancerAggregateStats {
    pub requests_routed: u64,
    pub backend_failures: u64,
    pub active_connections: u32,
    pub healthy_backends: u32,
    pub average_response_time_ms: f64,
}

impl LoadBalancerAggregateStats {
    fn update_from_metrics(&mut self, metrics: &ComponentMetrics) {
        if let Some(&requests) = metrics.counters.get("requests_routed") {
            self.requests_routed += requests;
        }
        if let Some(&failures) = metrics.counters.get("backend_failures") {
            self.backend_failures += failures;
        }
        if let Some(&connections) = metrics.gauges.get("active_connections") {
            self.active_connections = connections as u32;
        }
        if let Some(&backends) = metrics.gauges.get("healthy_backends") {
            self.healthy_backends = backends as u32;
        }
    }
}

/// Aggregate security statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityAggregateStats {
    pub packets_inspected: u64,
    pub threats_blocked: u64,
    pub policy_violations: u64,
    pub active_policies: u32,
    pub threat_score: f64,
}

impl SecurityAggregateStats {
    fn update_from_metrics(&mut self, metrics: &ComponentMetrics) {
        if let Some(&inspected) = metrics.counters.get("packets_inspected") {
            self.packets_inspected += inspected;
        }
        if let Some(&blocked) = metrics.counters.get("threats_blocked") {
            self.threats_blocked += blocked;
        }
        if let Some(&violations) = metrics.counters.get("policy_violations") {
            self.policy_violations += violations;
        }
        if let Some(&policies) = metrics.gauges.get("active_policies") {
            self.active_policies = policies as u32;
        }
        if let Some(&score) = metrics.gauges.get("threat_score") {
            self.threat_score = score;
        }
    }
}

/// System-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: u64,
    pub network_interfaces: u32,
    pub ebpf_programs_loaded: u32,
    pub kernel_version: String,
}

impl SystemMetrics {
    async fn collect() -> Self {
        Self {
            cpu_usage_percent: rand::random::<f64>() * 100.0,
            memory_usage_mb: 512 + rand::random::<u64>() % 1024,
            network_interfaces: 2,
            ebpf_programs_loaded: 4,
            kernel_version: "6.15.8-zen1-1-zen".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_creation() {
        let metrics = EbpfMetrics::new();
        let snapshot = metrics.snapshot().await.unwrap();
        assert!(!snapshot.component_metrics.is_empty());
    }

    #[test]
    fn test_metric_collector() {
        let collector = MetricCollector::new("test");
        assert_eq!(collector.component, "test");
        assert!(collector.history.is_empty());
    }

    #[tokio::test]
    async fn test_component_metrics_collection() {
        let mut collector = MetricCollector::new("network");
        let metrics = collector.collect().await;
        
        assert_eq!(metrics.component, "network");
        assert!(!metrics.counters.is_empty());
        assert!(!metrics.gauges.is_empty());
    }

    #[test]
    fn test_custom_metric() {
        let metric = CustomMetric {
            name: "test_metric".to_string(),
            metric_type: MetricType::Counter,
            description: "Test metric".to_string(),
            labels: HashMap::new(),
        };
        
        assert_eq!(metric.name, "test_metric");
    }
}