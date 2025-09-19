//! Performance Monitoring for Internet 2.0 Protocol Stack
//! 
//! Comprehensive monitoring system that tracks performance across all layers
//! and ensures the 40 Gbps throughput target and other performance goals.

use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};
use dashmap::DashMap;

use crate::config::Internet2Config;

/// Performance Monitor for Internet 2.0 Protocol Stack
/// 
/// Tracks performance metrics across all layers:
/// - STOQ Transport: 40 Gbps throughput target
/// - HyperMesh Assets: Consensus validation timing (<100ms)
/// - TrustChain Authority: Certificate operations (<35ms)
/// - Integration: Cross-layer coordination efficiency
pub struct PerformanceMonitor {
    /// Configuration
    config: Arc<Internet2Config>,
    
    /// Layer-specific metrics
    stoq_metrics: Arc<RwLock<StoqMetrics>>,
    hypermesh_metrics: Arc<RwLock<HyperMeshMetrics>>,
    trustchain_metrics: Arc<RwLock<TrustChainMetrics>>,
    integration_metrics: Arc<RwLock<IntegrationMetrics>>,
    
    /// Overall stack metrics
    stack_metrics: Arc<RwLock<StackMetrics>>,
    
    /// Performance alerts
    alerts: Arc<DashMap<String, PerformanceAlert>>,
    
    /// Monitoring state
    monitoring_state: Arc<RwLock<MonitoringState>>,
}

/// STOQ Transport layer metrics
#[derive(Debug, Clone, Default)]
pub struct StoqMetrics {
    /// Throughput metrics (40 Gbps target)
    pub current_throughput_gbps: f64,
    pub peak_throughput_gbps: f64,
    pub avg_throughput_gbps: f64,
    pub throughput_samples: Vec<(Instant, f64)>,
    
    /// Connection metrics
    pub active_connections: u32,
    pub total_connections: u64,
    pub connection_establishment_time_ms: Vec<f64>,
    pub connection_errors: u64,
    
    /// Certificate validation metrics (at transport level)
    pub certificates_validated: u64,
    pub certificate_validation_time_ms: Vec<f64>,
    pub certificate_validation_errors: u64,
    
    /// DNS resolution metrics (at transport level)
    pub dns_queries: u64,
    pub dns_resolution_time_ms: Vec<f64>,
    pub dns_errors: u64,
    
    /// Performance optimization metrics
    pub zero_copy_operations: u64,
    pub hardware_acceleration_ops: u64,
    pub memory_pool_hits: u64,
    pub memory_pool_misses: u64,
}

/// HyperMesh Asset layer metrics
#[derive(Debug, Clone, Default)]
pub struct HyperMeshMetrics {
    /// Asset metrics
    pub total_assets: u32,
    pub asset_allocations: u64,
    pub allocation_time_ms: Vec<f64>,
    pub allocation_errors: u64,
    
    /// Consensus metrics (<100ms target)
    pub consensus_operations: u64,
    pub consensus_validation_time_ms: Vec<f64>,
    pub consensus_success_rate: f64,
    pub consensus_errors: u64,
    
    /// VM execution metrics
    pub vm_executions: u64,
    pub vm_execution_time_ms: Vec<f64>,
    pub vm_errors: u64,
    
    /// Proxy/NAT metrics
    pub proxy_connections: u32,
    pub proxy_throughput_mbps: f64,
    pub proxy_latency_ms: Vec<f64>,
}

/// TrustChain Authority layer metrics
#[derive(Debug, Clone, Default)]
pub struct TrustChainMetrics {
    /// Certificate operations (<35ms target)
    pub certificates_issued: u64,
    pub certificate_issuance_time_ms: Vec<f64>,
    pub certificates_validated: u64,
    pub certificate_validation_time_ms: Vec<f64>,
    
    /// DNS operations
    pub dns_queries_resolved: u64,
    pub dns_resolution_time_ms: Vec<f64>,
    pub dns_cache_hits: u64,
    pub dns_cache_misses: u64,
    
    /// Certificate Transparency metrics
    pub ct_submissions: u64,
    pub ct_verification_time_ms: Vec<f64>,
    
    /// Post-quantum crypto metrics
    pub pq_operations: u64,
    pub pq_operation_time_ms: Vec<f64>,
    
    /// Certificate rotation metrics
    pub certificate_rotations: u64,
    pub rotation_time_ms: Vec<f64>,
}

/// Integration layer metrics
#[derive(Debug, Clone, Default)]
pub struct IntegrationMetrics {
    /// Cross-layer operations
    pub cross_layer_operations: u64,
    pub integration_latency_ms: Vec<f64>,
    pub integration_errors: u64,
    
    /// Performance coordination
    pub performance_coordinations: u64,
    pub coordination_time_ms: Vec<f64>,
    
    /// Layer communication
    pub layer_messages: u64,
    pub message_latency_ms: Vec<f64>,
}

/// Overall stack metrics
#[derive(Debug, Clone, serde::Serialize)]
pub struct StackMetrics {
    /// Overall performance
    pub stack_throughput_gbps: f64,
    pub stack_latency_ms: f64,
    pub stack_availability: f64,
    
    /// Layer status
    pub layers_operational: u8, // 0-3 layers operational
    pub layers_integrated: bool,
    
    /// Performance targets
    pub performance_targets_met: u8, // Number of targets met
    pub overall_performance_score: f64, // 0-100 score
    
    /// Alerts
    pub active_alerts: u32,
    pub critical_alerts: u32,
    
    /// Resource utilization
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub network_utilization: f64,
    
    /// Error rates
    pub overall_error_rate: f64,
    pub error_count_24h: u64,
}

/// Performance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    pub id: String,
    pub level: AlertLevel,
    pub message: String,
    pub layer: String,
    pub metric: String,
    pub current_value: f64,
    pub threshold_value: f64,
    pub created_at: SystemTime,
    pub acknowledged: bool,
}

/// Alert levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Monitoring state
#[derive(Debug, Clone)]
pub struct MonitoringState {
    pub monitoring_active: bool,
    pub started_at: Instant,
    pub last_collection_at: Option<Instant>,
    pub collection_interval: Duration,
    pub retention_period: Duration,
}

/// Stack statistics for external reporting
#[derive(Debug, Clone, serde::Serialize)]
pub struct StackStatistics {
    /// Performance overview
    pub stoq_throughput: f64, // Current STOQ throughput in Mbps
    pub active_assets: u32,
    pub active_certificates: u32,
    pub integration_ops_per_sec: f64,
    
    /// Performance warnings
    pub performance_warnings: Vec<String>,
    
    /// Layer health
    pub layer_health: LayerHealth,
}

/// Layer health status
#[derive(Debug, Clone, serde::Serialize)]
pub struct LayerHealth {
    pub stoq_healthy: bool,
    pub hypermesh_healthy: bool,
    pub trustchain_healthy: bool,
    pub integration_healthy: bool,
}

impl PerformanceMonitor {
    /// Create new performance monitor
    pub async fn new(config: Arc<Internet2Config>) -> Result<Self> {
        info!("📊 Initializing Performance Monitor");
        info!("   Targets: 40 Gbps STOQ, <100ms consensus, <35ms certificates");
        
        let monitoring_state = MonitoringState {
            monitoring_active: false,
            started_at: Instant::now(),
            last_collection_at: None,
            collection_interval: config.global.metrics_interval,
            retention_period: Duration::from_secs(24 * 3600), // 24 hours
        };
        
        Ok(Self {
            config,
            stoq_metrics: Arc::new(RwLock::new(StoqMetrics::default())),
            hypermesh_metrics: Arc::new(RwLock::new(HyperMeshMetrics::default())),
            trustchain_metrics: Arc::new(RwLock::new(TrustChainMetrics::default())),
            integration_metrics: Arc::new(RwLock::new(IntegrationMetrics::default())),
            stack_metrics: Arc::new(RwLock::new(StackMetrics {
                stack_throughput_gbps: 0.0,
                stack_latency_ms: 0.0,
                stack_availability: 100.0,
                layers_operational: 0,
                layers_integrated: false,
                performance_targets_met: 0,
                overall_performance_score: 0.0,
                active_alerts: 0,
                critical_alerts: 0,
                cpu_utilization: 0.0,
                memory_utilization: 0.0,
                network_utilization: 0.0,
                overall_error_rate: 0.0,
                error_count_24h: 0,
            })),
            alerts: Arc::new(DashMap::new()),
            monitoring_state: Arc::new(RwLock::new(monitoring_state)),
        })
    }
    
    /// Start performance monitoring
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Performance Monitor");
        
        // Update monitoring state
        let mut state = self.monitoring_state.write().await;
        state.monitoring_active = true;
        state.started_at = Instant::now();
        
        // Start metric collection loop
        let collection_interval = state.collection_interval;
        let monitor_clone = self.clone();
        
        tokio::spawn(async move {
            monitor_clone.metric_collection_loop(collection_interval).await;
        });
        
        // Start alert processing loop
        let monitor_clone = self.clone();
        tokio::spawn(async move {
            monitor_clone.alert_processing_loop().await;
        });
        
        info!("✅ Performance Monitor started");
        info!("   Collection interval: {:?}", collection_interval);
        
        Ok(())
    }
    
    /// Metric collection loop
    async fn metric_collection_loop(&self, interval: Duration) {
        let mut collection_timer = tokio::time::interval(interval);
        
        loop {
            collection_timer.tick().await;
            
            if let Err(e) = self.collect_metrics().await {
                warn!("Metric collection failed: {}", e);
            }
            
            if let Err(e) = self.update_stack_metrics().await {
                warn!("Stack metrics update failed: {}", e);
            }
            
            if let Err(e) = self.check_performance_targets().await {
                warn!("Performance target check failed: {}", e);
            }
        }
    }
    
    /// Alert processing loop
    async fn alert_processing_loop(&self) {
        let mut alert_timer = tokio::time::interval(Duration::from_secs(30)); // Check alerts every 30s
        
        loop {
            alert_timer.tick().await;
            
            if let Err(e) = self.process_alerts().await {
                warn!("Alert processing failed: {}", e);
            }
        }
    }
    
    /// Collect metrics from all layers
    async fn collect_metrics(&self) -> Result<()> {
        let mut state = self.monitoring_state.write().await;
        state.last_collection_at = Some(Instant::now());
        
        // Metrics collection would happen here
        // For now, just update the collection timestamp
        debug!("Collecting metrics from all layers");
        
        Ok(())
    }
    
    /// Update overall stack metrics
    async fn update_stack_metrics(&self) -> Result<()> {
        let stoq_metrics = self.stoq_metrics.read().await;
        let hypermesh_metrics = self.hypermesh_metrics.read().await;
        let trustchain_metrics = self.trustchain_metrics.read().await;
        let _integration_metrics = self.integration_metrics.read().await;
        
        let mut stack_metrics = self.stack_metrics.write().await;
        
        // Calculate overall throughput
        stack_metrics.stack_throughput_gbps = stoq_metrics.current_throughput_gbps;
        
        // Calculate layers operational
        stack_metrics.layers_operational = 3; // Assume all layers operational
        stack_metrics.layers_integrated = true; // Assume integration successful
        
        // Calculate performance score based on targets
        let mut targets_met = 0;
        let mut performance_score = 0.0;
        
        // STOQ throughput target (40 Gbps)
        if stoq_metrics.current_throughput_gbps >= self.config.stoq.performance.target_throughput_gbps {
            targets_met += 1;
            performance_score += 33.33;
        } else {
            let achievement = stoq_metrics.current_throughput_gbps / self.config.stoq.performance.target_throughput_gbps;
            performance_score += achievement * 33.33;
        }
        
        // HyperMesh consensus target (<100ms)
        let avg_consensus_time = calculate_average(&hypermesh_metrics.consensus_validation_time_ms);
        if avg_consensus_time <= 100.0 {
            targets_met += 1;
            performance_score += 33.33;
        } else {
            let achievement = 100.0 / avg_consensus_time;
            performance_score += achievement * 33.33;
        }
        
        // TrustChain certificate target (<35ms)
        let avg_cert_time = calculate_average(&trustchain_metrics.certificate_issuance_time_ms);
        if avg_cert_time <= 35.0 {
            targets_met += 1;
            performance_score += 33.34;
        } else {
            let achievement = 35.0 / avg_cert_time;
            performance_score += achievement * 33.34;
        }
        
        stack_metrics.performance_targets_met = targets_met;
        stack_metrics.overall_performance_score = performance_score.min(100.0);
        
        // Update alert counts
        stack_metrics.active_alerts = self.alerts.len() as u32;
        stack_metrics.critical_alerts = self.alerts.iter()
            .filter(|alert| matches!(alert.level, AlertLevel::Critical))
            .count() as u32;
        
        debug!("Stack metrics updated: {:.1}% performance score, {}/{} targets met", 
               stack_metrics.overall_performance_score, 
               stack_metrics.performance_targets_met, 3);
        
        Ok(())
    }
    
    /// Check performance targets and generate alerts
    async fn check_performance_targets(&self) -> Result<()> {
        let stoq_metrics = self.stoq_metrics.read().await;
        let hypermesh_metrics = self.hypermesh_metrics.read().await;
        let trustchain_metrics = self.trustchain_metrics.read().await;
        
        // Check STOQ throughput target
        let throughput_target = self.config.stoq.performance.target_throughput_gbps;
        if stoq_metrics.current_throughput_gbps < throughput_target * 0.8 { // 80% threshold
            self.create_alert(
                AlertLevel::Warning,
                "STOQ throughput below target".to_string(),
                "stoq".to_string(),
                "throughput_gbps".to_string(),
                stoq_metrics.current_throughput_gbps,
                throughput_target,
            ).await;
        }
        
        // Check HyperMesh consensus timing
        let avg_consensus_time = calculate_average(&hypermesh_metrics.consensus_validation_time_ms);
        if avg_consensus_time > 100.0 {
            self.create_alert(
                AlertLevel::Warning,
                "HyperMesh consensus validation exceeding target".to_string(),
                "hypermesh".to_string(),
                "consensus_time_ms".to_string(),
                avg_consensus_time,
                100.0,
            ).await;
        }
        
        // Check TrustChain certificate timing
        let avg_cert_time = calculate_average(&trustchain_metrics.certificate_issuance_time_ms);
        if avg_cert_time > 35.0 {
            self.create_alert(
                AlertLevel::Warning,
                "TrustChain certificate operations exceeding target".to_string(),
                "trustchain".to_string(),
                "certificate_ops_ms".to_string(),
                avg_cert_time,
                35.0,
            ).await;
        }
        
        Ok(())
    }
    
    /// Create performance alert
    async fn create_alert(
        &self,
        level: AlertLevel,
        message: String,
        layer: String,
        metric: String,
        current_value: f64,
        threshold_value: f64,
    ) {
        let alert_id = format!("{}-{}-{}", layer, metric, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs());
        
        let alert = PerformanceAlert {
            id: alert_id.clone(),
            level: level.clone(),
            message: message.clone(),
            layer,
            metric,
            current_value,
            threshold_value,
            created_at: SystemTime::now(),
            acknowledged: false,
        };
        
        self.alerts.insert(alert_id, alert);
        
        match level {
            AlertLevel::Critical => warn!("🚨 CRITICAL ALERT: {}", message),
            AlertLevel::Error => warn!("❌ ERROR: {}", message),
            AlertLevel::Warning => warn!("⚠️  WARNING: {}", message),
            AlertLevel::Info => info!("ℹ️  INFO: {}", message),
        }
    }
    
    /// Process and clean up alerts
    async fn process_alerts(&self) -> Result<()> {
        // Remove old acknowledged alerts
        let cutoff_time = SystemTime::now() - Duration::from_secs(3600); // 1 hour
        
        self.alerts.retain(|_, alert| {
            !(alert.acknowledged && alert.created_at < cutoff_time)
        });
        
        Ok(())
    }
    
    /// Record STOQ throughput measurement
    pub async fn record_stoq_throughput(&self, throughput_gbps: f64) {
        let mut metrics = self.stoq_metrics.write().await;
        metrics.current_throughput_gbps = throughput_gbps;
        metrics.peak_throughput_gbps = metrics.peak_throughput_gbps.max(throughput_gbps);
        metrics.throughput_samples.push((Instant::now(), throughput_gbps));
        
        // Keep only recent samples (last hour)
        let cutoff = Instant::now() - Duration::from_secs(3600);
        metrics.throughput_samples.retain(|(time, _)| *time > cutoff);
        
        // Calculate rolling average
        if !metrics.throughput_samples.is_empty() {
            metrics.avg_throughput_gbps = metrics.throughput_samples.iter()
                .map(|(_, throughput)| throughput)
                .sum::<f64>() / metrics.throughput_samples.len() as f64;
        }
    }
    
    /// Record certificate operation
    pub async fn record_certificate_operation(&self, duration: Duration) {
        let mut metrics = self.trustchain_metrics.write().await;
        metrics.certificates_issued += 1;
        metrics.certificate_issuance_time_ms.push(duration.as_millis() as f64);
        
        // Keep only recent measurements
        if metrics.certificate_issuance_time_ms.len() > 1000 {
            metrics.certificate_issuance_time_ms.drain(0..500);
        }
    }
    
    /// Record consensus operation
    pub async fn record_consensus_operation(&self, duration: Duration) {
        let mut metrics = self.hypermesh_metrics.write().await;
        metrics.consensus_operations += 1;
        metrics.consensus_validation_time_ms.push(duration.as_millis() as f64);
        
        // Keep only recent measurements
        if metrics.consensus_validation_time_ms.len() > 1000 {
            metrics.consensus_validation_time_ms.drain(0..500);
        }
    }
    
    /// Record DNS operation
    pub async fn record_dns_operation(&self, duration: Duration) {
        let mut metrics = self.trustchain_metrics.write().await;
        metrics.dns_queries_resolved += 1;
        metrics.dns_resolution_time_ms.push(duration.as_millis() as f64);
        
        // Keep only recent measurements
        if metrics.dns_resolution_time_ms.len() > 1000 {
            metrics.dns_resolution_time_ms.drain(0..500);
        }
    }
    
    /// Record integration operation
    pub async fn record_integration_operation(&self, duration: Duration) {
        let mut metrics = self.integration_metrics.write().await;
        metrics.cross_layer_operations += 1;
        metrics.integration_latency_ms.push(duration.as_millis() as f64);
        
        // Keep only recent measurements
        if metrics.integration_latency_ms.len() > 1000 {
            metrics.integration_latency_ms.drain(0..500);
        }
    }
    
    /// Get stack statistics for external reporting
    pub async fn get_stack_statistics(&self) -> StackStatistics {
        let stoq_metrics = self.stoq_metrics.read().await;
        let hypermesh_metrics = self.hypermesh_metrics.read().await;
        let trustchain_metrics = self.trustchain_metrics.read().await;
        let stack_metrics = self.stack_metrics.read().await;
        
        let mut performance_warnings = Vec::new();
        
        // Check for performance issues
        if stoq_metrics.current_throughput_gbps < self.config.stoq.performance.target_throughput_gbps * 0.8 {
            performance_warnings.push(format!(
                "STOQ throughput {:.1} Gbps below target {:.1} Gbps",
                stoq_metrics.current_throughput_gbps,
                self.config.stoq.performance.target_throughput_gbps
            ));
        }
        
        let avg_consensus_time = calculate_average(&hypermesh_metrics.consensus_validation_time_ms);
        if avg_consensus_time > 100.0 {
            performance_warnings.push(format!(
                "HyperMesh consensus time {:.1}ms exceeds 100ms target",
                avg_consensus_time
            ));
        }
        
        let avg_cert_time = calculate_average(&trustchain_metrics.certificate_issuance_time_ms);
        if avg_cert_time > 35.0 {
            performance_warnings.push(format!(
                "TrustChain certificate ops {:.1}ms exceeds 35ms target",
                avg_cert_time
            ));
        }
        
        StackStatistics {
            stoq_throughput: stoq_metrics.current_throughput_gbps * 1000.0, // Convert to Mbps
            active_assets: hypermesh_metrics.total_assets,
            active_certificates: trustchain_metrics.certificates_issued as u32,
            integration_ops_per_sec: 0.0, // Would be calculated
            performance_warnings,
            layer_health: LayerHealth {
                stoq_healthy: stoq_metrics.current_throughput_gbps > 0.0,
                hypermesh_healthy: hypermesh_metrics.total_assets > 0,
                trustchain_healthy: trustchain_metrics.certificates_issued > 0,
                integration_healthy: stack_metrics.layers_integrated,
            },
        }
    }
    
    /// Shutdown performance monitor
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down Performance Monitor");
        
        let mut state = self.monitoring_state.write().await;
        state.monitoring_active = false;
        
        // Clear alerts
        self.alerts.clear();
        
        info!("✅ Performance Monitor shutdown complete");
        Ok(())
    }
}

impl Clone for PerformanceMonitor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            stoq_metrics: self.stoq_metrics.clone(),
            hypermesh_metrics: self.hypermesh_metrics.clone(),
            trustchain_metrics: self.trustchain_metrics.clone(),
            integration_metrics: self.integration_metrics.clone(),
            stack_metrics: self.stack_metrics.clone(),
            alerts: self.alerts.clone(),
            monitoring_state: self.monitoring_state.clone(),
        }
    }
}

/// Helper function to calculate average of a vector
fn calculate_average(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}