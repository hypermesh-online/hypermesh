//! Metrics Collection and Aggregation
//!
//! Handles collection of metrics from various system components

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

use super::metrics::*;

/// Metrics collector for gathering performance data
pub struct MetricsCollector {
    /// Current metrics
    stoq_metrics: Arc<RwLock<StoqMetrics>>,
    hypermesh_metrics: Arc<RwLock<HyperMeshMetrics>>,
    trustchain_metrics: Arc<RwLock<TrustChainMetrics>>,
    integration_metrics: Arc<RwLock<IntegrationMetrics>>,
    stack_metrics: Arc<RwLock<StackMetrics>>,

    /// Collection state
    collection_interval: Duration,
    last_collection: Instant,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new(collection_interval: Duration) -> Self {
        Self {
            stoq_metrics: Arc::new(RwLock::new(StoqMetrics::default())),
            hypermesh_metrics: Arc::new(RwLock::new(HyperMeshMetrics::default())),
            trustchain_metrics: Arc::new(RwLock::new(TrustChainMetrics::default())),
            integration_metrics: Arc::new(RwLock::new(IntegrationMetrics::default())),
            stack_metrics: Arc::new(RwLock::new(StackMetrics::default())),
            collection_interval,
            last_collection: Instant::now(),
        }
    }

    /// Collect all metrics
    pub async fn collect(&self) -> Result<()> {
        debug!("Collecting performance metrics");

        // Collect from each layer
        self.collect_stoq_metrics().await?;
        self.collect_hypermesh_metrics().await?;
        self.collect_trustchain_metrics().await?;
        self.collect_integration_metrics().await?;

        // Update stack metrics
        self.update_stack_metrics().await?;

        self.last_collection = Instant::now();
        Ok(())
    }

    /// Collect STOQ metrics
    async fn collect_stoq_metrics(&self) -> Result<()> {
        let mut metrics = self.stoq_metrics.write().await;

        // Simulate metric collection
        metrics.current_throughput_gbps = 2.95; // Current bottleneck
        metrics.messages_per_second = 100_000;
        metrics.avg_message_size_bytes = 3686;
        metrics.quic_rtt_ms = 5.2;

        Ok(())
    }

    /// Collect HyperMesh metrics
    async fn collect_hypermesh_metrics(&self) -> Result<()> {
        let mut metrics = self.hypermesh_metrics.write().await;

        // Simulate metric collection
        metrics.consensus_validations_total += 100;
        metrics.avg_consensus_time_ms = 45.2;
        metrics.assets_allocated = 128;
        metrics.cpu_cores_allocated = 64;

        Ok(())
    }

    /// Collect TrustChain metrics
    async fn collect_trustchain_metrics(&self) -> Result<()> {
        let mut metrics = self.trustchain_metrics.write().await;

        // Simulate metric collection
        metrics.certificates_issued += 10;
        metrics.avg_cert_generation_ms = 28.3;
        metrics.avg_cert_verification_ms = 12.5;

        Ok(())
    }

    /// Collect integration metrics
    async fn collect_integration_metrics(&self) -> Result<()> {
        let mut metrics = self.integration_metrics.write().await;

        // Simulate metric collection
        metrics.cross_layer_calls += 1000;
        metrics.avg_coordination_time_ms = 15.6;
        metrics.avg_e2e_latency_ms = 85.3;

        Ok(())
    }

    /// Update stack-wide metrics
    async fn update_stack_metrics(&self) -> Result<()> {
        let stoq = self.stoq_metrics.read().await;
        let hypermesh = self.hypermesh_metrics.read().await;
        let trustchain = self.trustchain_metrics.read().await;
        let integration = self.integration_metrics.read().await;

        let mut stack = self.stack_metrics.write().await;

        // Calculate health scores
        stack.stoq_performance_score = self.calculate_stoq_score(&stoq);
        stack.hypermesh_performance_score = self.calculate_hypermesh_score(&hypermesh);
        stack.trustchain_performance_score = self.calculate_trustchain_score(&trustchain);
        stack.integration_performance_score = self.calculate_integration_score(&integration);

        // Overall health
        stack.overall_health_score = (
            stack.stoq_performance_score +
            stack.hypermesh_performance_score +
            stack.trustchain_performance_score +
            stack.integration_performance_score
        ) / 4.0;

        // Check targets
        stack.meeting_throughput_target = stoq.current_throughput_gbps >= 40.0;
        stack.meeting_consensus_target = hypermesh.avg_consensus_time_ms <= 100.0;
        stack.meeting_certificate_target = trustchain.avg_cert_generation_ms <= 35.0;
        stack.meeting_latency_target = integration.avg_e2e_latency_ms <= 100.0;

        // Update trends
        stack.throughput_trend = self.calculate_trend(stoq.current_throughput_gbps, 40.0);

        stack.last_update = SystemTime::now();

        Ok(())
    }

    /// Calculate STOQ performance score
    fn calculate_stoq_score(&self, metrics: &StoqMetrics) -> f64 {
        let throughput_score = (metrics.current_throughput_gbps / 40.0).min(1.0) * 100.0;
        let rtt_score = ((50.0 - metrics.quic_rtt_ms) / 50.0).max(0.0) * 100.0;
        let loss_score = ((1.0 - metrics.packet_loss_rate) * 100.0).max(0.0);

        (throughput_score * 0.5 + rtt_score * 0.3 + loss_score * 0.2).min(100.0)
    }

    /// Calculate HyperMesh performance score
    fn calculate_hypermesh_score(&self, metrics: &HyperMeshMetrics) -> f64 {
        let consensus_score = ((100.0 - metrics.avg_consensus_time_ms) / 100.0).max(0.0) * 100.0;
        let allocation_score = ((50.0 - metrics.avg_allocation_time_ms) / 50.0).max(0.0) * 100.0;
        let failure_rate = metrics.allocation_failures as f64 / metrics.allocation_requests.max(1) as f64;
        let success_score = ((1.0 - failure_rate) * 100.0).max(0.0);

        (consensus_score * 0.4 + allocation_score * 0.3 + success_score * 0.3).min(100.0)
    }

    /// Calculate TrustChain performance score
    fn calculate_trustchain_score(&self, metrics: &TrustChainMetrics) -> f64 {
        let gen_score = ((35.0 - metrics.avg_cert_generation_ms) / 35.0).max(0.0) * 100.0;
        let verify_score = ((20.0 - metrics.avg_cert_verification_ms) / 20.0).max(0.0) * 100.0;
        let cache_hit_rate = metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses).max(1) as f64;
        let cache_score = cache_hit_rate * 100.0;

        (gen_score * 0.4 + verify_score * 0.3 + cache_score * 0.3).min(100.0)
    }

    /// Calculate integration performance score
    fn calculate_integration_score(&self, metrics: &IntegrationMetrics) -> f64 {
        let latency_score = ((100.0 - metrics.avg_e2e_latency_ms) / 100.0).max(0.0) * 100.0;
        let success_rate = metrics.e2e_requests_successful as f64 / metrics.e2e_requests_total.max(1) as f64;
        let success_score = success_rate * 100.0;

        (latency_score * 0.5 + success_score * 0.5).min(100.0)
    }

    /// Calculate trend direction
    fn calculate_trend(&self, current: f64, target: f64) -> TrendDirection {
        let ratio = current / target;
        if ratio >= 0.95 {
            TrendDirection::Stable
        } else if ratio >= 0.8 {
            TrendDirection::Degrading
        } else {
            TrendDirection::Degrading
        }
    }

    /// Get current metrics snapshot
    pub async fn get_snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            stoq: self.stoq_metrics.read().await.clone(),
            hypermesh: self.hypermesh_metrics.read().await.clone(),
            trustchain: self.trustchain_metrics.read().await.clone(),
            integration: self.integration_metrics.read().await.clone(),
            stack: self.stack_metrics.read().await.clone(),
            timestamp: SystemTime::now(),
        }
    }
}

/// Metrics snapshot for reporting
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub stoq: StoqMetrics,
    pub hypermesh: HyperMeshMetrics,
    pub trustchain: TrustChainMetrics,
    pub integration: IntegrationMetrics,
    pub stack: StackMetrics,
    pub timestamp: SystemTime,
}

/// Metrics aggregator for statistics
pub struct MetricsAggregator {
    /// Historical snapshots
    snapshots: Vec<MetricsSnapshot>,

    /// Maximum snapshots to retain
    max_snapshots: usize,
}

impl MetricsAggregator {
    /// Create new aggregator
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: Vec::new(),
            max_snapshots,
        }
    }

    /// Add snapshot
    pub fn add_snapshot(&mut self, snapshot: MetricsSnapshot) {
        self.snapshots.push(snapshot);

        // Limit size
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.remove(0);
        }
    }

    /// Calculate statistics over time window
    pub fn calculate_statistics(&self, window: Duration) -> StackStatistics {
        let now = SystemTime::now();
        let cutoff = now - window;

        let recent_snapshots: Vec<_> = self.snapshots.iter()
            .filter(|s| s.timestamp >= cutoff)
            .collect();

        if recent_snapshots.is_empty() {
            return StackStatistics::default();
        }

        // Calculate averages
        let avg_throughput = recent_snapshots.iter()
            .map(|s| s.stoq.current_throughput_gbps)
            .sum::<f64>() / recent_snapshots.len() as f64;

        let avg_latency = recent_snapshots.iter()
            .map(|s| s.integration.avg_e2e_latency_ms)
            .sum::<f64>() / recent_snapshots.len() as f64;

        StackStatistics {
            period_start: recent_snapshots.first().unwrap().timestamp,
            period_end: recent_snapshots.last().unwrap().timestamp,
            avg_throughput_gbps: avg_throughput,
            peak_throughput_gbps: recent_snapshots.iter()
                .map(|s| s.stoq.current_throughput_gbps)
                .fold(0.0, f64::max),
            min_throughput_gbps: recent_snapshots.iter()
                .map(|s| s.stoq.current_throughput_gbps)
                .fold(f64::MAX, f64::min),
            avg_latency_ms: avg_latency,
            ..Default::default()
        }
    }
}