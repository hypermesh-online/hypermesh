//! Metrics collection for the consensus system

use prometheus::{Counter, Histogram, Gauge, Registry, Opts, HistogramOpts};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Comprehensive metrics for the consensus system
#[derive(Clone)]
pub struct ConsensusMetrics {
    // Leader Election Metrics
    pub leader_elections: Counter,
    pub election_duration: Histogram,
    pub leadership_changes: Counter,
    pub vote_requests_sent: Counter,
    pub vote_requests_received: Counter,
    pub votes_granted: Counter,
    pub votes_denied: Counter,
    
    // Log Replication Metrics
    pub log_entries_replicated: Counter,
    pub replication_latency: Histogram,
    pub append_entries_sent: Counter,
    pub append_entries_received: Counter,
    pub append_entries_failed: Counter,
    pub log_size_bytes: Gauge,
    pub committed_index: Gauge,
    pub last_applied_index: Gauge,
    
    // Byzantine Detection Metrics
    pub byzantine_nodes_detected: Counter,
    pub byzantine_evidence_collected: Counter,
    pub reputation_scores: Gauge,
    pub quarantined_nodes: Gauge,
    pub malicious_messages_blocked: Counter,
    
    // Transaction Metrics
    pub transactions_started: Counter,
    pub transactions_committed: Counter,
    pub transactions_aborted: Counter,
    pub transaction_duration: Histogram,
    pub read_conflicts: Counter,
    pub write_conflicts: Counter,
    pub deadlocks_detected: Counter,
    pub deadlock_resolution_time: Histogram,
    
    // MVCC Storage Metrics
    pub mvcc_versions_created: Counter,
    pub mvcc_versions_garbage_collected: Counter,
    pub version_read_latency: Histogram,
    pub storage_size_bytes: Gauge,
    pub gc_runs: Counter,
    pub gc_duration: Histogram,
    
    // Sharding Metrics
    pub active_shards: Gauge,
    pub shard_splits: Counter,
    pub shard_merges: Counter,
    pub shard_rebalances: Counter,
    pub hot_shards_detected: Counter,
    pub shard_migration_duration: Histogram,
    
    // Performance Metrics
    pub consensus_throughput: Gauge,
    pub network_throughput_bytes: Gauge,
    pub cpu_usage_percent: Gauge,
    pub memory_usage_bytes: Gauge,
    pub disk_usage_bytes: Gauge,
    
    // Error Metrics
    pub consensus_errors: Counter,
    pub network_errors: Counter,
    pub storage_errors: Counter,
    pub timeout_errors: Counter,
    
    registry: Arc<Registry>,
}

impl ConsensusMetrics {
    /// Create a new metrics collection with Prometheus registry
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Arc::new(Registry::new());
        
        // Leader Election Metrics
        let leader_elections = Counter::new(
            "consensus_leader_elections_total",
            "Total number of leader elections"
        )?;
        let election_duration = Histogram::with_opts(
            HistogramOpts::new(
                "consensus_election_duration_seconds",
                "Duration of leader elections in seconds"
            ).buckets(vec![0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0])
        )?;
        let leadership_changes = Counter::new(
            "consensus_leadership_changes_total",
            "Total number of leadership changes"
        )?;
        let vote_requests_sent = Counter::new(
            "consensus_vote_requests_sent_total",
            "Total number of vote requests sent"
        )?;
        let vote_requests_received = Counter::new(
            "consensus_vote_requests_received_total",
            "Total number of vote requests received"
        )?;
        let votes_granted = Counter::new(
            "consensus_votes_granted_total",
            "Total number of votes granted"
        )?;
        let votes_denied = Counter::new(
            "consensus_votes_denied_total",
            "Total number of votes denied"
        )?;
        
        // Log Replication Metrics
        let log_entries_replicated = Counter::new(
            "consensus_log_entries_replicated_total",
            "Total number of log entries replicated"
        )?;
        let replication_latency = Histogram::with_opts(
            HistogramOpts::new(
                "consensus_replication_latency_seconds",
                "Latency of log replication in seconds"
            ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5])
        )?;
        let append_entries_sent = Counter::new(
            "consensus_append_entries_sent_total",
            "Total number of append entries requests sent"
        )?;
        let append_entries_received = Counter::new(
            "consensus_append_entries_received_total",
            "Total number of append entries requests received"
        )?;
        let append_entries_failed = Counter::new(
            "consensus_append_entries_failed_total",
            "Total number of failed append entries requests"
        )?;
        let log_size_bytes = Gauge::new(
            "consensus_log_size_bytes",
            "Current size of the replicated log in bytes"
        )?;
        let committed_index = Gauge::new(
            "consensus_committed_index",
            "Current committed log index"
        )?;
        let last_applied_index = Gauge::new(
            "consensus_last_applied_index",
            "Last applied log index"
        )?;
        
        // Byzantine Detection Metrics
        let byzantine_nodes_detected = Counter::new(
            "consensus_byzantine_nodes_detected_total",
            "Total number of Byzantine nodes detected"
        )?;
        let byzantine_evidence_collected = Counter::new(
            "consensus_byzantine_evidence_collected_total",
            "Total amount of Byzantine evidence collected"
        )?;
        let reputation_scores = Gauge::new(
            "consensus_node_reputation_score",
            "Current reputation score of nodes"
        )?;
        let quarantined_nodes = Gauge::new(
            "consensus_quarantined_nodes",
            "Number of nodes currently quarantined"
        )?;
        let malicious_messages_blocked = Counter::new(
            "consensus_malicious_messages_blocked_total",
            "Total number of malicious messages blocked"
        )?;
        
        // Transaction Metrics
        let transactions_started = Counter::new(
            "consensus_transactions_started_total",
            "Total number of transactions started"
        )?;
        let transactions_committed = Counter::new(
            "consensus_transactions_committed_total",
            "Total number of transactions committed"
        )?;
        let transactions_aborted = Counter::new(
            "consensus_transactions_aborted_total",
            "Total number of transactions aborted"
        )?;
        let transaction_duration = Histogram::with_opts(
            HistogramOpts::new(
                "consensus_transaction_duration_seconds",
                "Duration of transactions in seconds"
            ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0])
        )?;
        let read_conflicts = Counter::new(
            "consensus_read_conflicts_total",
            "Total number of read conflicts detected"
        )?;
        let write_conflicts = Counter::new(
            "consensus_write_conflicts_total",
            "Total number of write conflicts detected"
        )?;
        let deadlocks_detected = Counter::new(
            "consensus_deadlocks_detected_total",
            "Total number of deadlocks detected"
        )?;
        let deadlock_resolution_time = Histogram::with_opts(
            HistogramOpts::new(
                "consensus_deadlock_resolution_seconds",
                "Time to resolve deadlocks in seconds"
            ).buckets(vec![0.001, 0.01, 0.1, 0.5, 1.0, 5.0])
        )?;
        
        // MVCC Storage Metrics
        let mvcc_versions_created = Counter::new(
            "consensus_mvcc_versions_created_total",
            "Total number of MVCC versions created"
        )?;
        let mvcc_versions_garbage_collected = Counter::new(
            "consensus_mvcc_versions_gc_total",
            "Total number of MVCC versions garbage collected"
        )?;
        let version_read_latency = Histogram::with_opts(
            HistogramOpts::new(
                "consensus_version_read_latency_seconds",
                "Latency of reading versioned data in seconds"
            ).buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1])
        )?;
        let storage_size_bytes = Gauge::new(
            "consensus_storage_size_bytes",
            "Current storage size in bytes"
        )?;
        let gc_runs = Counter::new(
            "consensus_gc_runs_total",
            "Total number of garbage collection runs"
        )?;
        let gc_duration = Histogram::with_opts(
            HistogramOpts::new(
                "consensus_gc_duration_seconds",
                "Duration of garbage collection runs in seconds"
            ).buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0])
        )?;
        
        // Sharding Metrics
        let active_shards = Gauge::new(
            "consensus_active_shards",
            "Current number of active shards"
        )?;
        let shard_splits = Counter::new(
            "consensus_shard_splits_total",
            "Total number of shard splits performed"
        )?;
        let shard_merges = Counter::new(
            "consensus_shard_merges_total",
            "Total number of shard merges performed"
        )?;
        let shard_rebalances = Counter::new(
            "consensus_shard_rebalances_total",
            "Total number of shard rebalances performed"
        )?;
        let hot_shards_detected = Counter::new(
            "consensus_hot_shards_detected_total",
            "Total number of hot shards detected"
        )?;
        let shard_migration_duration = Histogram::with_opts(
            HistogramOpts::new(
                "consensus_shard_migration_duration_seconds",
                "Duration of shard migrations in seconds"
            ).buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0])
        )?;
        
        // Performance Metrics
        let consensus_throughput = Gauge::new(
            "consensus_throughput_ops_per_second",
            "Current consensus throughput in operations per second"
        )?;
        let network_throughput_bytes = Gauge::new(
            "consensus_network_throughput_bytes_per_second",
            "Current network throughput in bytes per second"
        )?;
        let cpu_usage_percent = Gauge::new(
            "consensus_cpu_usage_percent",
            "Current CPU usage percentage"
        )?;
        let memory_usage_bytes = Gauge::new(
            "consensus_memory_usage_bytes",
            "Current memory usage in bytes"
        )?;
        let disk_usage_bytes = Gauge::new(
            "consensus_disk_usage_bytes",
            "Current disk usage in bytes"
        )?;
        
        // Error Metrics
        let consensus_errors = Counter::new(
            "consensus_errors_total",
            "Total number of consensus errors"
        )?;
        let network_errors = Counter::new(
            "consensus_network_errors_total",
            "Total number of network errors"
        )?;
        let storage_errors = Counter::new(
            "consensus_storage_errors_total",
            "Total number of storage errors"
        )?;
        let timeout_errors = Counter::new(
            "consensus_timeout_errors_total",
            "Total number of timeout errors"
        )?;
        
        // Register all metrics
        registry.register(Box::new(leader_elections.clone()))?;
        registry.register(Box::new(election_duration.clone()))?;
        registry.register(Box::new(leadership_changes.clone()))?;
        registry.register(Box::new(vote_requests_sent.clone()))?;
        registry.register(Box::new(vote_requests_received.clone()))?;
        registry.register(Box::new(votes_granted.clone()))?;
        registry.register(Box::new(votes_denied.clone()))?;
        
        registry.register(Box::new(log_entries_replicated.clone()))?;
        registry.register(Box::new(replication_latency.clone()))?;
        registry.register(Box::new(append_entries_sent.clone()))?;
        registry.register(Box::new(append_entries_received.clone()))?;
        registry.register(Box::new(append_entries_failed.clone()))?;
        registry.register(Box::new(log_size_bytes.clone()))?;
        registry.register(Box::new(committed_index.clone()))?;
        registry.register(Box::new(last_applied_index.clone()))?;
        
        registry.register(Box::new(byzantine_nodes_detected.clone()))?;
        registry.register(Box::new(byzantine_evidence_collected.clone()))?;
        registry.register(Box::new(reputation_scores.clone()))?;
        registry.register(Box::new(quarantined_nodes.clone()))?;
        registry.register(Box::new(malicious_messages_blocked.clone()))?;
        
        registry.register(Box::new(transactions_started.clone()))?;
        registry.register(Box::new(transactions_committed.clone()))?;
        registry.register(Box::new(transactions_aborted.clone()))?;
        registry.register(Box::new(transaction_duration.clone()))?;
        registry.register(Box::new(read_conflicts.clone()))?;
        registry.register(Box::new(write_conflicts.clone()))?;
        registry.register(Box::new(deadlocks_detected.clone()))?;
        registry.register(Box::new(deadlock_resolution_time.clone()))?;
        
        registry.register(Box::new(mvcc_versions_created.clone()))?;
        registry.register(Box::new(mvcc_versions_garbage_collected.clone()))?;
        registry.register(Box::new(version_read_latency.clone()))?;
        registry.register(Box::new(storage_size_bytes.clone()))?;
        registry.register(Box::new(gc_runs.clone()))?;
        registry.register(Box::new(gc_duration.clone()))?;
        
        registry.register(Box::new(active_shards.clone()))?;
        registry.register(Box::new(shard_splits.clone()))?;
        registry.register(Box::new(shard_merges.clone()))?;
        registry.register(Box::new(shard_rebalances.clone()))?;
        registry.register(Box::new(hot_shards_detected.clone()))?;
        registry.register(Box::new(shard_migration_duration.clone()))?;
        
        registry.register(Box::new(consensus_throughput.clone()))?;
        registry.register(Box::new(network_throughput_bytes.clone()))?;
        registry.register(Box::new(cpu_usage_percent.clone()))?;
        registry.register(Box::new(memory_usage_bytes.clone()))?;
        registry.register(Box::new(disk_usage_bytes.clone()))?;
        
        registry.register(Box::new(consensus_errors.clone()))?;
        registry.register(Box::new(network_errors.clone()))?;
        registry.register(Box::new(storage_errors.clone()))?;
        registry.register(Box::new(timeout_errors.clone()))?;
        
        Ok(Self {
            leader_elections,
            election_duration,
            leadership_changes,
            vote_requests_sent,
            vote_requests_received,
            votes_granted,
            votes_denied,
            
            log_entries_replicated,
            replication_latency,
            append_entries_sent,
            append_entries_received,
            append_entries_failed,
            log_size_bytes,
            committed_index,
            last_applied_index,
            
            byzantine_nodes_detected,
            byzantine_evidence_collected,
            reputation_scores,
            quarantined_nodes,
            malicious_messages_blocked,
            
            transactions_started,
            transactions_committed,
            transactions_aborted,
            transaction_duration,
            read_conflicts,
            write_conflicts,
            deadlocks_detected,
            deadlock_resolution_time,
            
            mvcc_versions_created,
            mvcc_versions_garbage_collected,
            version_read_latency,
            storage_size_bytes,
            gc_runs,
            gc_duration,
            
            active_shards,
            shard_splits,
            shard_merges,
            shard_rebalances,
            hot_shards_detected,
            shard_migration_duration,
            
            consensus_throughput,
            network_throughput_bytes,
            cpu_usage_percent,
            memory_usage_bytes,
            disk_usage_bytes,
            
            consensus_errors,
            network_errors,
            storage_errors,
            timeout_errors,
            
            registry,
        })
    }
    
    /// Get the Prometheus registry for metrics export
    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }
    
    /// Record a timed operation
    pub fn time_operation<F, R>(&self, histogram: &Histogram, operation: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = operation();
        histogram.observe(start.elapsed().as_secs_f64());
        result
    }
    
    /// Record an async timed operation
    pub async fn time_async_operation<F, R>(&self, histogram: &Histogram, operation: F) -> R
    where
        F: std::future::Future<Output = R>,
    {
        let start = Instant::now();
        let result = operation.await;
        histogram.observe(start.elapsed().as_secs_f64());
        result
    }
    
    /// Update resource usage metrics
    pub fn update_resource_usage(&self, cpu: f64, memory: u64, disk: u64) {
        self.cpu_usage_percent.set(cpu);
        self.memory_usage_bytes.set(memory as f64);
        self.disk_usage_bytes.set(disk as f64);
    }
    
    /// Update throughput metrics
    pub fn update_throughput(&self, consensus_ops: f64, network_bytes: f64) {
        self.consensus_throughput.set(consensus_ops);
        self.network_throughput_bytes.set(network_bytes);
    }
    
    /// Record leader election
    pub fn record_leader_election(&self, duration: Duration) {
        self.leader_elections.inc();
        self.election_duration.observe(duration.as_secs_f64());
        info!("Leader election completed in {:?}", duration);
    }
    
    /// Record leadership change
    pub fn record_leadership_change(&self, old_leader: &str, new_leader: &str) {
        self.leadership_changes.inc();
        info!("Leadership changed from {} to {}", old_leader, new_leader);
    }
    
    /// Record Byzantine detection
    pub fn record_byzantine_detection(&self, node_id: &str, evidence_type: &str) {
        self.byzantine_nodes_detected.inc();
        self.byzantine_evidence_collected.inc();
        warn!("Byzantine behavior detected from node {}: {}", node_id, evidence_type);
    }
    
    /// Record transaction metrics
    pub fn record_transaction_commit(&self, duration: Duration) {
        self.transactions_committed.inc();
        self.transaction_duration.observe(duration.as_secs_f64());
    }
    
    /// Record transaction abort
    pub fn record_transaction_abort(&self, duration: Duration, reason: &str) {
        self.transactions_aborted.inc();
        self.transaction_duration.observe(duration.as_secs_f64());
        info!("Transaction aborted after {:?}: {}", duration, reason);
    }
    
    /// Record shard operation
    pub fn record_shard_split(&self, shard_id: &str, duration: Duration) {
        self.shard_splits.inc();
        self.shard_migration_duration.observe(duration.as_secs_f64());
        info!("Shard {} split completed in {:?}", shard_id, duration);
    }
    
    /// Record hot shard detection
    pub fn record_hot_shard_detection(&self, shard_id: &str, metric: &str, value: f64) {
        self.hot_shards_detected.inc();
        warn!("Hot shard detected: {} - {} = {}", shard_id, metric, value);
    }
    
    /// Print current metrics summary
    pub fn print_summary(&self) {
        info!("=== Consensus Metrics Summary ===");
        info!("Leader Elections: {}", self.leader_elections.get());
        info!("Log Entries Replicated: {}", self.log_entries_replicated.get());
        info!("Transactions Committed: {}", self.transactions_committed.get());
        info!("Transactions Aborted: {}", self.transactions_aborted.get());
        info!("Byzantine Nodes Detected: {}", self.byzantine_nodes_detected.get());
        info!("Active Shards: {}", self.active_shards.get());
        info!("Consensus Throughput: {:.2} ops/sec", self.consensus_throughput.get());
        info!("=== End Metrics Summary ===");
    }
}

impl Default for ConsensusMetrics {
    fn default() -> Self {
        Self::new().expect("Failed to create default metrics")
    }
}

/// Timer helper for measuring operation duration
pub struct Timer {
    start: Instant,
    histogram: Histogram,
}

impl Timer {
    /// Create a new timer
    pub fn new(histogram: Histogram) -> Self {
        Self {
            start: Instant::now(),
            histogram,
        }
    }
    
    /// Record the elapsed time and consume the timer
    pub fn record(self) {
        self.histogram.observe(self.start.elapsed().as_secs_f64());
    }
    
    /// Get elapsed time without recording
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.histogram.observe(self.start.elapsed().as_secs_f64());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_metrics_creation() {
        let metrics = ConsensusMetrics::new().unwrap();
        assert_eq!(metrics.leader_elections.get(), 0.0);
    }
    
    #[test]
    fn test_metrics_recording() {
        let metrics = ConsensusMetrics::new().unwrap();
        
        metrics.leader_elections.inc();
        assert_eq!(metrics.leader_elections.get(), 1.0);
        
        metrics.record_leader_election(Duration::from_millis(500));
        assert_eq!(metrics.leader_elections.get(), 2.0);
    }
    
    #[test]
    fn test_timer() {
        let metrics = ConsensusMetrics::new().unwrap();
        let timer = Timer::new(metrics.election_duration.clone());
        
        std::thread::sleep(Duration::from_millis(1));
        timer.record();
        
        // Should have recorded some time > 0
        assert!(metrics.election_duration.get_sample_count() > 0);
    }
}