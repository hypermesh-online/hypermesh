//! Integration layer metrics and monitoring

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use prometheus::{Counter, Histogram, Gauge, Registry, Opts, HistogramOpts};
use tracing::{info, warn, error};

use crate::{IntegrationResult, IntegrationError};
use crate::lifecycle::ComponentMetrics;

/// Integration layer metrics collector
pub struct IntegrationMetrics {
    /// Prometheus metrics registry
    registry: Arc<Registry>,
    /// Component initialization time
    component_init_duration: Arc<Histogram>,
    /// Cross-component communication latency
    communication_latency: Arc<Histogram>,
    /// Component health scores
    component_health_scores: Arc<Gauge>,
    /// Active service count
    active_services: Arc<Gauge>,
    /// Integration errors
    integration_errors: Arc<Counter>,
    /// Platform uptime
    platform_uptime: Arc<Gauge>,
    /// Metrics collection state
    collection_state: Arc<RwLock<MetricsCollectionState>>,
}

/// Metrics collection state
#[derive(Debug)]
struct MetricsCollectionState {
    /// Collection start time
    start_time: Instant,
    /// Last collection time
    last_collection: Option<Instant>,
    /// Component metrics history
    component_history: HashMap<String, Vec<ComponentMetrics>>,
    /// Communication metrics history
    communication_history: Vec<CommunicationMetric>,
}

/// Cross-component communication metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationMetric {
    /// Source component
    pub source: String,
    /// Target component
    pub target: String,
    /// Operation name
    pub operation: String,
    /// Response time in milliseconds
    pub response_time_ms: f64,
    /// Success status
    pub success: bool,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Platform-wide metrics aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMetrics {
    /// Collection timestamp
    pub timestamp: SystemTime,
    /// Platform uptime
    pub uptime: Duration,
    /// Component metrics by name
    pub components: HashMap<String, ComponentMetrics>,
    /// Cross-component communication metrics
    pub communications: Vec<CommunicationMetric>,
    /// Overall health score (0.0 - 1.0)
    pub overall_health_score: f64,
    /// Active services count
    pub active_services_count: u64,
    /// Total integration errors
    pub total_integration_errors: u64,
    /// Resource utilization
    pub resource_utilization: ResourceUtilization,
}

/// Resource utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// Total CPU usage across all components
    pub total_cpu_usage: f64,
    /// Total memory usage across all components
    pub total_memory_usage: u64,
    /// Total network throughput (bytes/sec)
    pub total_network_throughput: f64,
    /// Average response time across all components
    pub avg_response_time_ms: f64,
    /// Total request count
    pub total_request_count: u64,
    /// Total error count
    pub total_error_count: u64,
}

impl IntegrationMetrics {
    /// Create a new integration metrics collector
    pub fn new() -> Self {
        let registry = Arc::new(Registry::new());
        
        // Initialize Prometheus metrics
        let component_init_duration = Arc::new(
            Histogram::with_opts(
                HistogramOpts::new(
                    "hypermesh_component_init_duration_seconds",
                    "Component initialization duration in seconds"
                )
                .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0])
            ).expect("Failed to create component_init_duration histogram")
        );
        
        let communication_latency = Arc::new(
            Histogram::with_opts(
                HistogramOpts::new(
                    "hypermesh_communication_latency_seconds",
                    "Cross-component communication latency in seconds"
                )
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0])
            ).expect("Failed to create communication_latency histogram")
        );
        
        let component_health_scores = Arc::new(
            Gauge::with_opts(
                Opts::new(
                    "hypermesh_component_health_score",
                    "Component health scores (0.0 - 1.0)"
                )
            ).expect("Failed to create component_health_scores gauge")
        );
        
        let active_services = Arc::new(
            Gauge::with_opts(
                Opts::new(
                    "hypermesh_active_services_count",
                    "Number of active services in the platform"
                )
            ).expect("Failed to create active_services gauge")
        );
        
        let integration_errors = Arc::new(
            Counter::with_opts(
                Opts::new(
                    "hypermesh_integration_errors_total",
                    "Total number of integration errors"
                )
            ).expect("Failed to create integration_errors counter")
        );
        
        let platform_uptime = Arc::new(
            Gauge::with_opts(
                Opts::new(
                    "hypermesh_platform_uptime_seconds",
                    "Platform uptime in seconds"
                )
            ).expect("Failed to create platform_uptime gauge")
        );
        
        // Register metrics
        registry.register(Box::new(component_init_duration.clone())).expect("Failed to register component_init_duration");
        registry.register(Box::new(communication_latency.clone())).expect("Failed to register communication_latency");
        registry.register(Box::new(component_health_scores.clone())).expect("Failed to register component_health_scores");
        registry.register(Box::new(active_services.clone())).expect("Failed to register active_services");
        registry.register(Box::new(integration_errors.clone())).expect("Failed to register integration_errors");
        registry.register(Box::new(platform_uptime.clone())).expect("Failed to register platform_uptime");
        
        Self {
            registry,
            component_init_duration,
            communication_latency,
            component_health_scores,
            active_services,
            integration_errors,
            platform_uptime,
            collection_state: Arc::new(RwLock::new(MetricsCollectionState {
                start_time: Instant::now(),
                last_collection: None,
                component_history: HashMap::new(),
                communication_history: Vec::new(),
            })),
        }
    }
    
    /// Record component initialization time
    pub fn record_component_init(&self, _component_name: &str, duration: Duration) {
        self.component_init_duration.observe(duration.as_secs_f64());
    }
    
    /// Record cross-component communication
    pub async fn record_communication(&self, metric: CommunicationMetric) {
        self.communication_latency.observe(metric.response_time_ms / 1000.0);
        
        if !metric.success {
            self.integration_errors.inc();
        }
        
        let mut state = self.collection_state.write().await;
        state.communication_history.push(metric);
        
        // Keep only recent history (last 1000 entries)
        if state.communication_history.len() > 1000 {
            state.communication_history.drain(0..state.communication_history.len() - 1000);
        }
    }
    
    /// Update component health score
    pub fn update_component_health(&self, _component_name: &str, score: f64) {
        self.component_health_scores.set(score);
    }
    
    /// Update active services count
    pub fn update_active_services(&self, count: u64) {
        self.active_services.set(count as f64);
    }
    
    /// Record integration error
    pub fn record_integration_error(&self) {
        self.integration_errors.inc();
    }
    
    /// Update platform uptime
    pub fn update_platform_uptime(&self, uptime: Duration) {
        self.platform_uptime.set(uptime.as_secs_f64());
    }
    
    /// Collect component metrics
    pub async fn collect_component_metrics(&self, component_name: &str, metrics: ComponentMetrics) {
        let mut state = self.collection_state.write().await;
        
        let history = state.component_history.entry(component_name.to_string()).or_insert_with(Vec::new);
        history.push(metrics);
        
        // Keep only recent history (last 100 entries per component)
        if history.len() > 100 {
            history.drain(0..history.len() - 100);
        }
    }
    
    /// Collect platform-wide metrics
    pub async fn collect_platform_metrics(&self) -> PlatformMetrics {
        let mut state = self.collection_state.write().await;
        let now = SystemTime::now();
        let uptime = state.start_time.elapsed();
        
        // Update collection time
        state.last_collection = Some(Instant::now());
        
        // Aggregate component metrics
        let mut total_cpu = 0.0;
        let mut total_memory = 0u64;
        let mut total_requests = 0u64;
        let mut total_errors = 0u64;
        let mut total_response_time = 0.0;
        let mut component_count = 0;
        
        let mut components = HashMap::new();
        for (name, history) in &state.component_history {
            if let Some(latest_metrics) = history.last() {
                components.insert(name.clone(), latest_metrics.clone());
                
                total_cpu += latest_metrics.cpu_usage;
                total_memory += latest_metrics.memory_usage;
                total_requests += latest_metrics.request_count;
                total_errors += latest_metrics.error_count;
                total_response_time += latest_metrics.avg_response_time_ms;
                component_count += 1;
            }
        }
        
        let avg_response_time = if component_count > 0 {
            total_response_time / component_count as f64
        } else {
            0.0
        };
        
        // Calculate overall health score
        let overall_health_score = if component_count > 0 {
            // Simple health calculation based on error rate
            let error_rate = total_errors as f64 / total_requests.max(1) as f64;
            (1.0 - error_rate).max(0.0)
        } else {
            1.0
        };
        
        // Get recent communication metrics
        let recent_communications = state.communication_history.iter()
            .filter(|comm| {
                now.duration_since(comm.timestamp).unwrap_or(Duration::from_secs(3600)) < Duration::from_secs(300)
            })
            .cloned()
            .collect();
        
        // Calculate network throughput (simplified)
        let total_network_throughput = components.values()
            .map(|m| (m.network_tx_bytes + m.network_rx_bytes) as f64)
            .sum::<f64>() / uptime.as_secs() as f64;
        
        PlatformMetrics {
            timestamp: now,
            uptime,
            components,
            communications: recent_communications,
            overall_health_score,
            active_services_count: self.active_services.get() as u64,
            total_integration_errors: self.integration_errors.get() as u64,
            resource_utilization: ResourceUtilization {
                total_cpu_usage: total_cpu,
                total_memory_usage: total_memory,
                total_network_throughput,
                avg_response_time_ms: avg_response_time,
                total_request_count: total_requests,
                total_error_count: total_errors,
            },
        }
    }
    
    /// Get Prometheus registry for metrics export
    pub fn prometheus_registry(&self) -> &Registry {
        &self.registry
    }
    
    /// Export metrics as JSON
    pub async fn export_json(&self) -> String {
        let metrics = self.collect_platform_metrics().await;
        serde_json::to_string_pretty(&metrics).unwrap_or_else(|_| "{}".to_string())
    }
    
    /// Reset metrics collection
    pub async fn reset(&self) {
        let mut state = self.collection_state.write().await;
        state.component_history.clear();
        state.communication_history.clear();
        state.start_time = Instant::now();
        state.last_collection = None;
        
        info!("Integration metrics collection reset");
    }
}

impl Default for IntegrationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ResourceUtilization {
    fn default() -> Self {
        Self {
            total_cpu_usage: 0.0,
            total_memory_usage: 0,
            total_network_throughput: 0.0,
            avg_response_time_ms: 0.0,
            total_request_count: 0,
            total_error_count: 0,
        }
    }
}

/// Metrics helper functions
impl IntegrationMetrics {
    /// Start metrics collection background task
    pub async fn start_collection_task(&self, interval: Duration) -> IntegrationResult<()> {
        let metrics = self.clone_for_task();
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                
                // Update platform uptime
                let state = metrics.collection_state.read().await;
                let uptime = state.start_time.elapsed();
                drop(state);
                
                metrics.update_platform_uptime(uptime);
                
                // Log metrics summary
                let platform_metrics = metrics.collect_platform_metrics().await;
                info!(
                    "Platform metrics - Health: {:.2}, Components: {}, Errors: {}, Uptime: {:?}",
                    platform_metrics.overall_health_score,
                    platform_metrics.components.len(),
                    platform_metrics.total_integration_errors,
                    platform_metrics.uptime
                );
            }
        });
        
        info!("Started metrics collection task with interval {:?}", interval);
        Ok(())
    }
    
    /// Helper method to clone for background tasks
    fn clone_for_task(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            component_init_duration: self.component_init_duration.clone(),
            communication_latency: self.communication_latency.clone(),
            component_health_scores: self.component_health_scores.clone(),
            active_services: self.active_services.clone(),
            integration_errors: self.integration_errors.clone(),
            platform_uptime: self.platform_uptime.clone(),
            collection_state: self.collection_state.clone(),
        }
    }
}