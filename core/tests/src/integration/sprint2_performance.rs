//! Sprint 2 Performance Benchmark Tests
//!
//! Comprehensive performance validation for Byzantine fault-tolerant container orchestration
//! This suite measures and validates all Sprint 2 performance requirements.

use nexus_runtime::{
    ConsensusContainerOrchestrator, ContainerSpec, ContainerConsensusOperation,
    ImageSpec, RuntimeConfig, Runtime, ContainerStateManager,
    networking::{P2PNetworkManager, QuicTransportConfig},
    health::{HealthMonitor, HealthMetrics},
};
use nexus_consensus::pbft::{PbftNode, PbftConfig};
use nexus_consensus::byzantine::{ByzantineGuard, FaultDetectionConfig};
use nexus_shared::{NodeId, ResourceId};

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{mpsc, Mutex, RwLock as AsyncRwLock};
use tempfile::TempDir;
use tracing::{info, warn};
use criterion::{Criterion, BenchmarkId};

/// Sprint 2 Performance Requirements
#[derive(Debug, Clone)]
pub struct PerformanceRequirements {
    /// Maximum consensus coordination overhead
    pub max_consensus_overhead_ms: u64,  // Target: 50ms
    
    /// Maximum container startup time with consensus
    pub max_container_startup_ms: u64,   // Target: 100ms
    
    /// Maximum network setup time per container
    pub max_network_setup_ms: u64,       // Target: 10ms
    
    /// Maximum P2P connectivity establishment time
    pub max_p2p_connectivity_ms: u64,    // Target: 5ms
    
    /// Maximum monitoring overhead (CPU %)
    pub max_monitoring_overhead_percent: f64, // Target: 1%
}

impl Default for PerformanceRequirements {
    fn default() -> Self {
        Self {
            max_consensus_overhead_ms: 50,
            max_container_startup_ms: 100,
            max_network_setup_ms: 10,
            max_p2p_connectivity_ms: 5,
            max_monitoring_overhead_percent: 1.0,
        }
    }
}

/// Performance test results
#[derive(Debug, Default)]
pub struct PerformanceResults {
    /// Consensus coordination measurements
    pub consensus_overhead_ms: Vec<u64>,
    
    /// Container startup measurements
    pub container_startup_ms: Vec<u64>,
    
    /// Network setup measurements
    pub network_setup_ms: Vec<u64>,
    
    /// P2P connectivity measurements
    pub p2p_connectivity_ms: Vec<u64>,
    
    /// CPU monitoring overhead measurements
    pub monitoring_overhead_percent: Vec<f64>,
    
    /// Test violations
    pub violations: Vec<String>,
}

impl PerformanceResults {
    pub fn add_consensus_measurement(&mut self, ms: u64) {
        self.consensus_overhead_ms.push(ms);
    }
    
    pub fn add_startup_measurement(&mut self, ms: u64) {
        self.container_startup_ms.push(ms);
    }
    
    pub fn add_network_measurement(&mut self, ms: u64) {
        self.network_setup_ms.push(ms);
    }
    
    pub fn add_p2p_measurement(&mut self, ms: u64) {
        self.p2p_connectivity_ms.push(ms);
    }
    
    pub fn add_monitoring_overhead(&mut self, percent: f64) {
        self.monitoring_overhead_percent.push(percent);
    }
    
    pub fn add_violation(&mut self, violation: String) {
        self.violations.push(violation);
    }
    
    pub fn avg_consensus_overhead(&self) -> f64 {
        if self.consensus_overhead_ms.is_empty() {
            0.0
        } else {
            self.consensus_overhead_ms.iter().sum::<u64>() as f64 
                / self.consensus_overhead_ms.len() as f64
        }
    }
    
    pub fn avg_startup_time(&self) -> f64 {
        if self.container_startup_ms.is_empty() {
            0.0
        } else {
            self.container_startup_ms.iter().sum::<u64>() as f64
                / self.container_startup_ms.len() as f64
        }
    }
    
    pub fn avg_network_setup(&self) -> f64 {
        if self.network_setup_ms.is_empty() {
            0.0
        } else {
            self.network_setup_ms.iter().sum::<u64>() as f64
                / self.network_setup_ms.len() as f64
        }
    }
    
    pub fn avg_p2p_connectivity(&self) -> f64 {
        if self.p2p_connectivity_ms.is_empty() {
            0.0
        } else {
            self.p2p_connectivity_ms.iter().sum::<u64>() as f64
                / self.p2p_connectivity_ms.len() as f64
        }
    }
    
    pub fn avg_monitoring_overhead(&self) -> f64 {
        if self.monitoring_overhead_percent.is_empty() {
            0.0
        } else {
            self.monitoring_overhead_percent.iter().sum::<f64>()
                / self.monitoring_overhead_percent.len() as f64
        }
    }
    
    pub fn validate(&self, requirements: &PerformanceRequirements) -> bool {
        let mut valid = true;
        
        if self.avg_consensus_overhead() > requirements.max_consensus_overhead_ms as f64 {
            valid = false;
        }
        
        if self.avg_startup_time() > requirements.max_container_startup_ms as f64 {
            valid = false;
        }
        
        if self.avg_network_setup() > requirements.max_network_setup_ms as f64 {
            valid = false;
        }
        
        if self.avg_p2p_connectivity() > requirements.max_p2p_connectivity_ms as f64 {
            valid = false;
        }
        
        if self.avg_monitoring_overhead() > requirements.max_monitoring_overhead_percent {
            valid = false;
        }
        
        valid && self.violations.is_empty()
    }
}

/// Performance benchmark harness
pub struct PerformanceBenchmark {
    test_dir: TempDir,
    requirements: PerformanceRequirements,
    results: PerformanceResults,
}

impl PerformanceBenchmark {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            test_dir: TempDir::new()?,
            requirements: PerformanceRequirements::default(),
            results: PerformanceResults::default(),
        })
    }
    
    /// Benchmark consensus coordination overhead
    pub async fn benchmark_consensus_overhead(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Benchmarking consensus coordination overhead");
        
        let node_id = NodeId::random();
        let orchestrator = self.create_test_orchestrator(node_id).await?;
        
        // Warm up
        for _ in 0..5 {
            let spec = self.create_test_container_spec();
            let _ = orchestrator.submit_container_operation(
                ContainerConsensusOperation::Create(spec)
            ).await?;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        // Measure consensus overhead for multiple operations
        for i in 0..20 {
            let spec = self.create_test_container_spec();
            
            let start = Instant::now();
            let op_id = orchestrator.submit_container_operation(
                ContainerConsensusOperation::Create(spec.clone())
            ).await?;
            
            // Wait for consensus completion
            tokio::time::sleep(Duration::from_millis(20)).await;
            
            let consensus_time = start.elapsed().as_millis() as u64;
            self.results.add_consensus_measurement(consensus_time);
            
            if consensus_time > self.requirements.max_consensus_overhead_ms {
                self.results.add_violation(format!(
                    "Consensus overhead {}ms exceeds target {}ms (operation {})",
                    consensus_time, self.requirements.max_consensus_overhead_ms, i
                ));
            }
        }
        
        info!("Average consensus overhead: {:.2}ms", self.results.avg_consensus_overhead());
        Ok(())
    }
    
    /// Benchmark container startup with consensus
    pub async fn benchmark_container_startup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Benchmarking container startup with consensus");
        
        let node_id = NodeId::random();
        let orchestrator = self.create_test_orchestrator(node_id).await?;
        
        for i in 0..10 {
            let spec = self.create_test_container_spec();
            
            // First create the container
            let _ = orchestrator.submit_container_operation(
                ContainerConsensusOperation::Create(spec.clone())
            ).await?;
            tokio::time::sleep(Duration::from_millis(20)).await;
            
            // Measure startup time
            let start = Instant::now();
            let _ = orchestrator.submit_container_operation(
                ContainerConsensusOperation::Start(spec.id)
            ).await?;
            
            // Wait for container to start
            tokio::time::sleep(Duration::from_millis(50)).await;
            
            let startup_time = start.elapsed().as_millis() as u64;
            self.results.add_startup_measurement(startup_time);
            
            if startup_time > self.requirements.max_container_startup_ms {
                self.results.add_violation(format!(
                    "Container startup {}ms exceeds target {}ms (container {})",
                    startup_time, self.requirements.max_container_startup_ms, i
                ));
            }
        }
        
        info!("Average container startup: {:.2}ms", self.results.avg_startup_time());
        Ok(())
    }
    
    /// Benchmark P2P network setup
    pub async fn benchmark_p2p_networking(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Benchmarking P2P networking performance");
        
        let runtime_config = self.create_runtime_config();
        let runtime = Arc::new(Runtime::new(runtime_config).await?);
        
        // Test network setup for multiple containers
        for i in 0..15 {
            let container_id = ResourceId::random();
            
            let start = Instant::now();
            runtime.setup_container_network(&container_id).await?;
            let network_time = start.elapsed().as_millis() as u64;
            
            self.results.add_network_measurement(network_time);
            
            if network_time > self.requirements.max_network_setup_ms {
                self.results.add_violation(format!(
                    "Network setup {}ms exceeds target {}ms (container {})",
                    network_time, self.requirements.max_network_setup_ms, i
                ));
            }
        }
        
        // Test P2P connectivity establishment
        for i in 0..10 {
            let peer_addr = format!("127.0.0.1:{}", 9300 + i);
            
            let start = Instant::now();
            let _ = runtime.establish_p2p_connection(&peer_addr).await;
            let conn_time = start.elapsed().as_millis() as u64;
            
            self.results.add_p2p_measurement(conn_time);
            
            if conn_time > self.requirements.max_p2p_connectivity_ms {
                self.results.add_violation(format!(
                    "P2P connectivity {}ms exceeds target {}ms (peer {})",
                    conn_time, self.requirements.max_p2p_connectivity_ms, i
                ));
            }
        }
        
        info!("Average network setup: {:.2}ms", self.results.avg_network_setup());
        info!("Average P2P connectivity: {:.2}ms", self.results.avg_p2p_connectivity());
        Ok(())
    }
    
    /// Benchmark monitoring overhead
    pub async fn benchmark_monitoring_overhead(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Benchmarking monitoring overhead");
        
        let runtime_config = self.create_runtime_config();
        let runtime = Arc::new(Runtime::new(runtime_config).await?);
        
        // Get baseline CPU usage
        let baseline_cpu = self.measure_cpu_usage().await?;
        
        // Start health monitoring
        let health_monitor = HealthMonitor::new(runtime.clone());
        health_monitor.start_monitoring().await?;
        
        // Let it run for a bit
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // Measure CPU with monitoring
        let monitoring_cpu = self.measure_cpu_usage().await?;
        
        let overhead = monitoring_cpu - baseline_cpu;
        self.results.add_monitoring_overhead(overhead);
        
        if overhead > self.requirements.max_monitoring_overhead_percent {
            self.results.add_violation(format!(
                "Monitoring overhead {:.2}% exceeds target {:.2}%",
                overhead, self.requirements.max_monitoring_overhead_percent
            ));
        }
        
        info!("Monitoring CPU overhead: {:.2}%", overhead);
        Ok(())
    }
    
    /// Helper to create test orchestrator
    async fn create_test_orchestrator(&self, node_id: NodeId) -> Result<ConsensusContainerOrchestrator, Box<dyn std::error::Error>> {
        let runtime_config = self.create_runtime_config();
        let runtime = Arc::new(Runtime::new(runtime_config).await?);
        
        let pbft_config = PbftConfig {
            node_id,
            node_ids: vec![node_id],
            batch_size: 10,
            checkpoint_interval: 100,
            view_timeout: Duration::from_secs(1),
            ..Default::default()
        };
        
        let consensus_node = Arc::new(Mutex::new(PbftNode::new(pbft_config)?));
        let byzantine_guard = Arc::new(AsyncRwLock::new(
            ByzantineGuard::new(node_id, FaultDetectionConfig::default())
        ));
        let state_manager = Arc::new(ContainerStateManager::new(node_id));
        let (tx, _rx) = mpsc::unbounded_channel();
        
        Ok(ConsensusContainerOrchestrator::new(
            node_id,
            runtime,
            consensus_node,
            byzantine_guard,
            state_manager,
            tx,
        ).await?)
    }
    
    fn create_runtime_config(&self) -> RuntimeConfig {
        let mut config = RuntimeConfig::default();
        config.storage.data_dir = format!("{}/runtime", self.test_dir.path().display());
        config.networking.enable_p2p = true;
        config.networking.p2p_port = 9292;
        config.networking.quic_port = 9293;
        config
    }
    
    fn create_test_container_spec(&self) -> ContainerSpec {
        ContainerSpec {
            id: ResourceId::random(),
            name: format!("perf-test-{}", uuid::Uuid::new_v4()),
            image: ImageSpec {
                name: "alpine".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            env: vec![],
            mounts: vec![],
            labels: Default::default(),
            resources: Default::default(),
            network_config: Default::default(),
        }
    }
    
    async fn measure_cpu_usage(&self) -> Result<f64, Box<dyn std::error::Error>> {
        // Simplified CPU measurement - in production would use proper system metrics
        use sysinfo::{System, SystemExt, ProcessExt};
        
        let mut system = System::new_all();
        system.refresh_all();
        
        let pid = std::process::id();
        if let Some(process) = system.process(pid.into()) {
            Ok(process.cpu_usage() as f64)
        } else {
            Ok(0.0)
        }
    }
    
    /// Generate performance report
    pub fn generate_report(&self) -> String {
        let mut report = String::from("=== Sprint 2 Performance Report ===\n\n");
        
        report.push_str("Performance Metrics:\n");
        report.push_str(&format!("  Consensus Overhead: {:.2}ms (target: {}ms)\n", 
            self.results.avg_consensus_overhead(),
            self.requirements.max_consensus_overhead_ms
        ));
        report.push_str(&format!("  Container Startup: {:.2}ms (target: {}ms)\n",
            self.results.avg_startup_time(),
            self.requirements.max_container_startup_ms
        ));
        report.push_str(&format!("  Network Setup: {:.2}ms (target: {}ms)\n",
            self.results.avg_network_setup(),
            self.requirements.max_network_setup_ms
        ));
        report.push_str(&format!("  P2P Connectivity: {:.2}ms (target: {}ms)\n",
            self.results.avg_p2p_connectivity(),
            self.requirements.max_p2p_connectivity_ms
        ));
        report.push_str(&format!("  Monitoring Overhead: {:.2}% (target: {:.2}%)\n\n",
            self.results.avg_monitoring_overhead(),
            self.requirements.max_monitoring_overhead_percent
        ));
        
        if self.results.violations.is_empty() {
            report.push_str("✅ All performance targets met!\n");
        } else {
            report.push_str("⚠️ Performance Violations:\n");
            for violation in &self.results.violations {
                report.push_str(&format!("  - {}\n", violation));
            }
        }
        
        report
    }
}

/// Main performance test suite
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_sprint2_performance_suite() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("=== Sprint 2 Performance Test Suite ===");
    
    let mut benchmark = PerformanceBenchmark::new().await?;
    
    // Run all benchmarks
    benchmark.benchmark_consensus_overhead().await?;
    benchmark.benchmark_container_startup().await?;
    benchmark.benchmark_p2p_networking().await?;
    benchmark.benchmark_monitoring_overhead().await?;
    
    // Generate and display report
    let report = benchmark.generate_report();
    info!("\n{}", report);
    
    // Validate all requirements met
    assert!(
        benchmark.results.validate(&benchmark.requirements),
        "Performance requirements not met"
    );
    
    info!("=== Performance Test Suite PASSED ===");
    Ok(())
}

/// Load test for sustained operations
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_performance() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("=== Load Performance Test ===");
    
    let test_dir = TempDir::new()?;
    let node_id = NodeId::random();
    
    let mut runtime_config = RuntimeConfig::default();
    runtime_config.storage.data_dir = format!("{}/load", test_dir.path().display());
    std::fs::create_dir_all(&runtime_config.storage.data_dir)?;
    
    let runtime = Arc::new(Runtime::new(runtime_config).await?);
    
    let pbft_config = PbftConfig {
        node_id,
        node_ids: vec![node_id],
        batch_size: 20,
        checkpoint_interval: 100,
        view_timeout: Duration::from_secs(1),
        ..Default::default()
    };
    
    let consensus_node = Arc::new(Mutex::new(PbftNode::new(pbft_config)?));
    let byzantine_guard = Arc::new(AsyncRwLock::new(
        ByzantineGuard::new(node_id, FaultDetectionConfig::default())
    ));
    let state_manager = Arc::new(ContainerStateManager::new(node_id));
    let (tx, _rx) = mpsc::unbounded_channel();
    
    let orchestrator = ConsensusContainerOrchestrator::new(
        node_id,
        runtime,
        consensus_node,
        byzantine_guard,
        state_manager,
        tx,
    ).await?;
    
    // Submit many operations concurrently
    let num_operations = 100;
    let mut handles = Vec::new();
    
    let orchestrator = Arc::new(orchestrator);
    
    for i in 0..num_operations {
        let orchestrator = orchestrator.clone();
        let handle = tokio::spawn(async move {
            let spec = ContainerSpec {
                id: ResourceId::random(),
                name: format!("load-test-{}", i),
                image: ImageSpec {
                    name: "alpine".to_string(),
                    tag: "latest".to_string(),
                    digest: None,
                },
                env: vec![],
                mounts: vec![],
                labels: Default::default(),
                resources: Default::default(),
                network_config: Default::default(),
            };
            
            orchestrator.submit_container_operation(
                ContainerConsensusOperation::Create(spec)
            ).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations
    let start = Instant::now();
    for handle in handles {
        handle.await??;
    }
    let total_time = start.elapsed();
    
    let ops_per_second = num_operations as f64 / total_time.as_secs_f64();
    info!("Processed {} operations in {:?}", num_operations, total_time);
    info!("Throughput: {:.2} operations/second", ops_per_second);
    
    assert!(
        ops_per_second > 10.0,
        "Throughput {:.2} ops/s below minimum 10 ops/s",
        ops_per_second
    );
    
    info!("=== Load Performance Test PASSED ===");
    Ok(())
}