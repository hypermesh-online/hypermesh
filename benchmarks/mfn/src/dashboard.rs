/*!
# Performance Dashboard

Real-time performance monitoring dashboard with:
- Live performance metrics
- Historical trend visualization
- Alert system integration
- Interactive charts and graphs
- WebSocket-based real-time updates
*/

use crate::common::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::broadcast;

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub enable_real_time: bool,
    pub update_interval_ms: u64,
    pub history_retention_hours: u32,
    pub max_data_points: usize,
    pub enable_alerts: bool,
    pub alert_thresholds: AlertThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub latency_warning_ms: f64,
    pub latency_critical_ms: f64,
    pub throughput_warning_ops_sec: f64,
    pub memory_warning_mb: f64,
    pub error_rate_warning_percent: f64,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            enable_real_time: true,
            update_interval_ms: 1000, // 1 second updates
            history_retention_hours: 24, // 24 hours of history
            max_data_points: 1440, // 1 minute resolution for 24 hours
            enable_alerts: true,
            alert_thresholds: AlertThresholds {
                latency_warning_ms: 1.0,
                latency_critical_ms: 10.0,
                throughput_warning_ops_sec: 1_000_000.0,
                memory_warning_mb: 500.0,
                error_rate_warning_percent: 5.0,
            },
        }
    }
}

/// Real-time performance dashboard
pub struct PerformanceDashboard {
    config: DashboardConfig,
    metrics_store: Arc<Mutex<MetricsStore>>,
    alert_system: AlertSystem,
    event_broadcaster: broadcast::Sender<DashboardEvent>,
    last_update: Arc<Mutex<Instant>>,
}

#[derive(Debug, Clone)]
pub struct MetricsStore {
    pub layer_metrics: HashMap<MfnLayer, LayerMetricsHistory>,
    pub system_metrics: SystemMetricsHistory,
    pub alert_history: VecDeque<AlertEvent>,
}

#[derive(Debug, Clone)]
pub struct LayerMetricsHistory {
    pub layer: MfnLayer,
    pub latency_history: VecDeque<DataPoint>,
    pub throughput_history: VecDeque<DataPoint>,
    pub error_rate_history: VecDeque<DataPoint>,
    pub memory_usage_history: VecDeque<DataPoint>,
}

#[derive(Debug, Clone)]
pub struct SystemMetricsHistory {
    pub cpu_usage_history: VecDeque<DataPoint>,
    pub memory_usage_history: VecDeque<DataPoint>,
    pub network_io_history: VecDeque<DataPoint>,
    pub disk_io_history: VecDeque<DataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: SystemTime,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DashboardEvent {
    MetricsUpdate(MetricsSnapshot),
    AlertTriggered(AlertEvent),
    BenchmarkCompleted(BenchmarkResult),
    SystemStatusChange(SystemStatus),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: SystemTime,
    pub layer_metrics: HashMap<MfnLayer, LayerSnapshot>,
    pub system_metrics: SystemSnapshot,
    pub active_alerts: Vec<AlertEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerSnapshot {
    pub layer: MfnLayer,
    pub current_latency_ms: f64,
    pub current_throughput_ops_sec: f64,
    pub current_error_rate_percent: f64,
    pub current_memory_mb: f64,
    pub target_status: TargetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub memory_usage_percent: f64,
    pub network_rx_mbps: f64,
    pub network_tx_mbps: f64,
    pub disk_read_mbps: f64,
    pub disk_write_mbps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetStatus {
    Optimal,   // All targets met, performing well
    Warning,   // Some targets at risk
    Critical,  // Targets not met
    Unknown,   // Insufficient data
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemStatus {
    Healthy,
    Degraded,
    Critical,
    Offline,
}

/// Alert system for monitoring performance thresholds
pub struct AlertSystem {
    config: DashboardConfig,
    active_alerts: Arc<Mutex<HashMap<String, AlertEvent>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub layer: Option<MfnLayer>,
    pub metric_name: String,
    pub current_value: f64,
    pub threshold_value: f64,
    pub message: String,
    pub triggered_at: SystemTime,
    pub resolved_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    LatencyThreshold,
    ThroughputThreshold,
    MemoryThreshold,
    ErrorRateThreshold,
    SystemResource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl PerformanceDashboard {
    pub fn new(config: DashboardConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        
        let metrics_store = Arc::new(Mutex::new(MetricsStore {
            layer_metrics: HashMap::new(),
            system_metrics: SystemMetricsHistory {
                cpu_usage_history: VecDeque::new(),
                memory_usage_history: VecDeque::new(),
                network_io_history: VecDeque::new(),
                disk_io_history: VecDeque::new(),
            },
            alert_history: VecDeque::new(),
        }));

        let alert_system = AlertSystem {
            config: config.clone(),
            active_alerts: Arc::new(Mutex::new(HashMap::new())),
        };

        Self {
            config,
            metrics_store,
            alert_system,
            event_broadcaster: event_tx,
            last_update: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Start the dashboard with real-time updates
    pub async fn start(&self) -> anyhow::Result<()> {
        if !self.config.enable_real_time {
            return Ok(());
        }

        println!("📊 Starting performance dashboard with real-time updates");

        let metrics_store = self.metrics_store.clone();
        let alert_system = self.alert_system.clone();
        let event_broadcaster = self.event_broadcaster.clone();
        let config = self.config.clone();
        let last_update = self.last_update.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(config.update_interval_ms));
            
            loop {
                interval.tick().await;
                
                // Update metrics
                if let Ok(snapshot) = Self::collect_current_metrics(&config).await {
                    Self::update_metrics_store(&metrics_store, &snapshot, &config).await;
                    
                    // Check for alerts
                    let alerts = alert_system.check_thresholds(&snapshot).await;
                    
                    // Broadcast update event
                    let event = DashboardEvent::MetricsUpdate(snapshot);
                    let _ = event_broadcaster.send(event);
                    
                    // Broadcast alert events
                    for alert in alerts {
                        let event = DashboardEvent::AlertTriggered(alert);
                        let _ = event_broadcaster.send(event);
                    }
                    
                    *last_update.lock().unwrap() = Instant::now();
                }
            }
        });

        Ok(())
    }

    /// Record benchmark result in dashboard
    pub async fn record_benchmark_result(&self, result: &BenchmarkResult) -> anyhow::Result<()> {
        let event = DashboardEvent::BenchmarkCompleted(result.clone());
        let _ = self.event_broadcaster.send(event);

        // Update layer metrics
        {
            let mut store = self.metrics_store.lock().unwrap();
            let layer_metrics = store.layer_metrics.entry(result.layer).or_insert_with(|| LayerMetricsHistory {
                layer: result.layer,
                latency_history: VecDeque::new(),
                throughput_history: VecDeque::new(),
                error_rate_history: VecDeque::new(),
                memory_usage_history: VecDeque::new(),
            });

            let now = SystemTime::now();
            
            Self::add_data_point(&mut layer_metrics.latency_history, 
                                 DataPoint { timestamp: now, value: result.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0 },
                                 self.config.max_data_points);
            
            Self::add_data_point(&mut layer_metrics.throughput_history,
                                 DataPoint { timestamp: now, value: result.metrics.throughput_ops_per_sec },
                                 self.config.max_data_points);
            
            Self::add_data_point(&mut layer_metrics.error_rate_history,
                                 DataPoint { timestamp: now, value: result.metrics.error_rate },
                                 self.config.max_data_points);
            
            Self::add_data_point(&mut layer_metrics.memory_usage_history,
                                 DataPoint { timestamp: now, value: result.metrics.memory_usage_mb },
                                 self.config.max_data_points);
        }

        Ok(())
    }

    fn add_data_point(history: &mut VecDeque<DataPoint>, point: DataPoint, max_points: usize) {
        history.push_back(point);
        while history.len() > max_points {
            history.pop_front();
        }
    }

    async fn collect_current_metrics(config: &DashboardConfig) -> anyhow::Result<MetricsSnapshot> {
        let system_metrics = Self::collect_system_metrics().await?;
        let now = SystemTime::now();

        // In a real implementation, this would collect from actual running MFN components
        // For now, we'll create a basic snapshot structure
        Ok(MetricsSnapshot {
            timestamp: now,
            layer_metrics: HashMap::new(), // Would be populated from actual MFN components
            system_metrics,
            active_alerts: Vec::new(),
        })
    }

    async fn collect_system_metrics() -> anyhow::Result<SystemSnapshot> {
        use sysinfo::{System, SystemExt, CpuExt, NetworkExt, DiskExt};
        
        let mut system = System::new_all();
        system.refresh_all();

        let cpu_usage = system.cpus().iter()
            .map(|cpu| cpu.cpu_usage() as f64)
            .sum::<f64>() / system.cpus().len() as f64;

        let total_memory = system.total_memory() as f64 / 1024.0 / 1024.0; // Convert to MB
        let used_memory = system.used_memory() as f64 / 1024.0 / 1024.0;
        let memory_usage_percent = (used_memory / total_memory) * 100.0;

        // Network I/O (simplified)
        let mut network_rx_total = 0u64;
        let mut network_tx_total = 0u64;
        
        for (_, network) in system.networks() {
            network_rx_total += network.received();
            network_tx_total += network.transmitted();
        }
        
        let network_rx_mbps = network_rx_total as f64 / 1024.0 / 1024.0 * 8.0; // Convert to Mbps
        let network_tx_mbps = network_tx_total as f64 / 1024.0 / 1024.0 * 8.0;

        // Disk I/O (simplified)
        let disks = system.disks();
        let disk_read_mbps = 0.0; // Would need to track deltas
        let disk_write_mbps = 0.0;

        Ok(SystemSnapshot {
            cpu_usage_percent: cpu_usage,
            memory_usage_mb: used_memory,
            memory_usage_percent,
            network_rx_mbps,
            network_tx_mbps,
            disk_read_mbps,
            disk_write_mbps,
        })
    }

    async fn update_metrics_store(
        metrics_store: &Arc<Mutex<MetricsStore>>,
        snapshot: &MetricsSnapshot,
        config: &DashboardConfig,
    ) {
        let mut store = metrics_store.lock().unwrap();
        let now = SystemTime::now();

        // Update system metrics
        Self::add_data_point(&mut store.system_metrics.cpu_usage_history,
                             DataPoint { timestamp: now, value: snapshot.system_metrics.cpu_usage_percent },
                             config.max_data_points);
        
        Self::add_data_point(&mut store.system_metrics.memory_usage_history,
                             DataPoint { timestamp: now, value: snapshot.system_metrics.memory_usage_mb },
                             config.max_data_points);

        // Clean up old data
        Self::cleanup_old_data(&mut store, config);
    }

    fn cleanup_old_data(store: &mut MetricsStore, config: &DashboardConfig) {
        let cutoff_time = SystemTime::now() - Duration::from_secs(config.history_retention_hours as u64 * 3600);
        
        // Clean up system metrics
        store.system_metrics.cpu_usage_history.retain(|dp| dp.timestamp >= cutoff_time);
        store.system_metrics.memory_usage_history.retain(|dp| dp.timestamp >= cutoff_time);
        store.system_metrics.network_io_history.retain(|dp| dp.timestamp >= cutoff_time);
        store.system_metrics.disk_io_history.retain(|dp| dp.timestamp >= cutoff_time);
        
        // Clean up layer metrics
        for layer_metrics in store.layer_metrics.values_mut() {
            layer_metrics.latency_history.retain(|dp| dp.timestamp >= cutoff_time);
            layer_metrics.throughput_history.retain(|dp| dp.timestamp >= cutoff_time);
            layer_metrics.error_rate_history.retain(|dp| dp.timestamp >= cutoff_time);
            layer_metrics.memory_usage_history.retain(|dp| dp.timestamp >= cutoff_time);
        }
        
        // Clean up alert history
        store.alert_history.retain(|alert| alert.triggered_at >= cutoff_time);
    }

    /// Get current dashboard state
    pub fn get_dashboard_state(&self) -> DashboardState {
        let store = self.metrics_store.lock().unwrap();
        let active_alerts = self.alert_system.active_alerts.lock().unwrap();
        let last_update = *self.last_update.lock().unwrap();

        DashboardState {
            last_update,
            layer_count: store.layer_metrics.len(),
            active_alert_count: active_alerts.len(),
            data_point_count: store.layer_metrics.values()
                .map(|metrics| metrics.latency_history.len())
                .sum(),
            system_status: if last_update.elapsed() < Duration::from_secs(10) {
                SystemStatus::Healthy
            } else {
                SystemStatus::Degraded
            },
        }
    }

    /// Subscribe to dashboard events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<DashboardEvent> {
        self.event_broadcaster.subscribe()
    }

    /// Generate dashboard summary report
    pub fn generate_summary_report(&self) -> DashboardSummary {
        let store = self.metrics_store.lock().unwrap();
        let active_alerts = self.alert_system.active_alerts.lock().unwrap();

        let mut layer_summaries = HashMap::new();
        
        for (layer, metrics) in &store.layer_metrics {
            let latest_latency = metrics.latency_history.back().map(|dp| dp.value).unwrap_or(0.0);
            let latest_throughput = metrics.throughput_history.back().map(|dp| dp.value).unwrap_or(0.0);
            let latest_error_rate = metrics.error_rate_history.back().map(|dp| dp.value).unwrap_or(0.0);
            
            // Determine target status
            let target_status = match layer {
                MfnLayer::Layer1Ifr => {
                    if latest_latency < 0.1 { TargetStatus::Optimal }
                    else if latest_latency < 1.0 { TargetStatus::Warning }
                    else { TargetStatus::Critical }
                }
                MfnLayer::Layer2Dsr => {
                    if latest_latency < 1.0 { TargetStatus::Optimal }
                    else if latest_latency < 5.0 { TargetStatus::Warning }
                    else { TargetStatus::Critical }
                }
                MfnLayer::Layer4Cpe => {
                    if latest_latency < 2.0 { TargetStatus::Optimal }
                    else if latest_latency < 10.0 { TargetStatus::Warning }
                    else { TargetStatus::Critical }
                }
                _ => TargetStatus::Unknown,
            };

            layer_summaries.insert(*layer, LayerSummary {
                layer: *layer,
                current_latency_ms: latest_latency,
                current_throughput_ops_sec: latest_throughput,
                current_error_rate_percent: latest_error_rate,
                target_status,
                data_points: metrics.latency_history.len(),
            });
        }

        let system_summary = SystemSummary {
            cpu_usage_percent: store.system_metrics.cpu_usage_history.back().map(|dp| dp.value).unwrap_or(0.0),
            memory_usage_mb: store.system_metrics.memory_usage_history.back().map(|dp| dp.value).unwrap_or(0.0),
            active_alerts: active_alerts.len(),
            total_data_retention_hours: self.config.history_retention_hours,
        };

        DashboardSummary {
            timestamp: SystemTime::now(),
            layer_summaries,
            system_summary,
            active_alert_count: active_alerts.len(),
            total_data_points: store.layer_metrics.values()
                .map(|m| m.latency_history.len())
                .sum(),
        }
    }
}

impl AlertSystem {
    async fn check_thresholds(&self, snapshot: &MetricsSnapshot) -> Vec<AlertEvent> {
        let mut new_alerts = Vec::new();
        let mut active_alerts = self.active_alerts.lock().unwrap();

        // Check system-level thresholds
        if snapshot.system_metrics.memory_usage_mb > self.config.alert_thresholds.memory_warning_mb {
            let alert_id = "system_memory_high".to_string();
            
            if !active_alerts.contains_key(&alert_id) {
                let alert = AlertEvent {
                    id: alert_id.clone(),
                    alert_type: AlertType::SystemResource,
                    severity: AlertSeverity::Warning,
                    layer: None,
                    metric_name: "system_memory_usage".to_string(),
                    current_value: snapshot.system_metrics.memory_usage_mb,
                    threshold_value: self.config.alert_thresholds.memory_warning_mb,
                    message: format!("System memory usage ({:.1} MB) exceeds warning threshold ({:.1} MB)",
                                   snapshot.system_metrics.memory_usage_mb,
                                   self.config.alert_thresholds.memory_warning_mb),
                    triggered_at: SystemTime::now(),
                    resolved_at: None,
                };
                
                active_alerts.insert(alert_id, alert.clone());
                new_alerts.push(alert);
            }
        }

        // Check layer-specific thresholds
        for (layer, layer_snapshot) in &snapshot.layer_metrics {
            // Latency alerts
            if layer_snapshot.current_latency_ms > self.config.alert_thresholds.latency_critical_ms {
                let alert_id = format!("{}_latency_critical", layer);
                
                if !active_alerts.contains_key(&alert_id) {
                    let alert = AlertEvent {
                        id: alert_id.clone(),
                        alert_type: AlertType::LatencyThreshold,
                        severity: AlertSeverity::Critical,
                        layer: Some(*layer),
                        metric_name: "latency".to_string(),
                        current_value: layer_snapshot.current_latency_ms,
                        threshold_value: self.config.alert_thresholds.latency_critical_ms,
                        message: format!("{} latency ({:.3} ms) exceeds critical threshold ({:.3} ms)",
                                       layer, layer_snapshot.current_latency_ms,
                                       self.config.alert_thresholds.latency_critical_ms),
                        triggered_at: SystemTime::now(),
                        resolved_at: None,
                    };
                    
                    active_alerts.insert(alert_id, alert.clone());
                    new_alerts.push(alert);
                }
            }
            
            // Error rate alerts
            if layer_snapshot.current_error_rate_percent > self.config.alert_thresholds.error_rate_warning_percent {
                let alert_id = format!("{}_error_rate_high", layer);
                
                if !active_alerts.contains_key(&alert_id) {
                    let alert = AlertEvent {
                        id: alert_id.clone(),
                        alert_type: AlertType::ErrorRateThreshold,
                        severity: AlertSeverity::Warning,
                        layer: Some(*layer),
                        metric_name: "error_rate".to_string(),
                        current_value: layer_snapshot.current_error_rate_percent,
                        threshold_value: self.config.alert_thresholds.error_rate_warning_percent,
                        message: format!("{} error rate ({:.1}%) exceeds warning threshold ({:.1}%)",
                                       layer, layer_snapshot.current_error_rate_percent,
                                       self.config.alert_thresholds.error_rate_warning_percent),
                        triggered_at: SystemTime::now(),
                        resolved_at: None,
                    };
                    
                    active_alerts.insert(alert_id, alert.clone());
                    new_alerts.push(alert);
                }
            }
        }

        new_alerts
    }
}

// Data structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardState {
    pub last_update: Instant,
    pub layer_count: usize,
    pub active_alert_count: usize,
    pub data_point_count: usize,
    pub system_status: SystemStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub timestamp: SystemTime,
    pub layer_summaries: HashMap<MfnLayer, LayerSummary>,
    pub system_summary: SystemSummary,
    pub active_alert_count: usize,
    pub total_data_points: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerSummary {
    pub layer: MfnLayer,
    pub current_latency_ms: f64,
    pub current_throughput_ops_sec: f64,
    pub current_error_rate_percent: f64,
    pub target_status: TargetStatus,
    pub data_points: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSummary {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub active_alerts: usize,
    pub total_data_retention_hours: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_creation() {
        let config = DashboardConfig::default();
        let dashboard = PerformanceDashboard::new(config);
        
        let state = dashboard.get_dashboard_state();
        assert_eq!(state.layer_count, 0);
        assert_eq!(state.active_alert_count, 0);
    }

    #[tokio::test]
    async fn test_benchmark_result_recording() {
        let config = DashboardConfig::default();
        let dashboard = PerformanceDashboard::new(config);
        
        let test_result = create_test_benchmark_result();
        let result = dashboard.record_benchmark_result(&test_result).await;
        
        assert!(result.is_ok());
        
        let state = dashboard.get_dashboard_state();
        assert_eq!(state.layer_count, 1);
    }

    #[test]
    fn test_alert_system() {
        let config = DashboardConfig::default();
        let alert_system = AlertSystem {
            config: config.clone(),
            active_alerts: Arc::new(Mutex::new(HashMap::new())),
        };
        
        // Test that alert system can be created
        assert_eq!(alert_system.active_alerts.lock().unwrap().len(), 0);
    }

    fn create_test_benchmark_result() -> BenchmarkResult {
        BenchmarkResult {
            id: "test".to_string(),
            name: "test_benchmark".to_string(),
            layer: MfnLayer::Layer1Ifr,
            config: BenchmarkConfig {
                warmup_iterations: 10,
                measurement_iterations: 100,
                statistical_confidence: 0.95,
                regression_threshold: 0.05,
                memory_limit_mb: 128,
                timeout_seconds: 60,
                parallel_workers: 1,
                output_format: OutputFormat::Json,
                enable_flamegraph: false,
                enable_perf_counters: false,
            },
            metrics: PerformanceMetrics {
                benchmark_id: "test".to_string(),
                layer: MfnLayer::Layer1Ifr,
                timestamp: chrono::Utc::now(),
                duration: Duration::from_secs(1),
                throughput_ops_per_sec: 1000000.0,
                latency_percentiles: LatencyPercentiles {
                    p50: Duration::from_micros(50),
                    p75: Duration::from_micros(75),
                    p90: Duration::from_micros(90),
                    p95: Duration::from_micros(95),
                    p99: Duration::from_micros(99),
                    p999: Duration::from_micros(100),
                    max: Duration::from_micros(200),
                    min: Duration::from_micros(10),
                    mean: Duration::from_micros(60),
                    stddev: Duration::from_micros(5),
                },
                memory_usage_mb: 8.0,
                cpu_utilization: 25.0,
                error_rate: 0.0,
                custom_metrics: HashMap::new(),
            },
            target_validation: TargetValidation {
                latency_target_met: true,
                throughput_target_met: true,
                memory_target_met: true,
                improvement_target_met: true,
                overall_success: true,
                target_details: HashMap::new(),
            },
            baseline_comparison: None,
            success: true,
            error_message: None,
        }
    }
}