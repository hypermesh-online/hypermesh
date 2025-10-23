//! Chaos engineering and fault injection testing
//! Tests system resilience under various failure scenarios

pub mod network_chaos;
pub mod resource_chaos;
pub mod timing_chaos;
pub mod byzantine_failures;

use crate::{TestResult, init_test_logging};
use tracing::{info, warn, error};
use std::time::{Duration, Instant};
use rand::{Rng, thread_rng};

/// Chaos test configuration
#[derive(Debug, Clone)]
pub struct ChaosConfig {
    pub duration_seconds: u64,
    pub failure_rate: f64,        // Percentage of operations that should fail (0.0-1.0)
    pub failure_duration_ms: u64, // How long failures should last
    pub recovery_time_ms: u64,    // Time to wait for recovery after failure
    pub max_concurrent_failures: usize,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            duration_seconds: 60,
            failure_rate: 0.1,        // 10% failure rate
            failure_duration_ms: 1000, // 1 second failures
            recovery_time_ms: 2000,    // 2 seconds recovery
            max_concurrent_failures: 3,
        }
    }
}

/// Types of chaos that can be injected
#[derive(Debug, Clone)]
pub enum ChaosType {
    NetworkPartition,
    NetworkLatency(Duration),
    NetworkPacketLoss(f64),
    ResourceExhaustion,
    ProcessCrash,
    DiskFull,
    MemoryExhaustion,
    TimingSkew(Duration),
    ByzantineFailure,
    RandomFailure,
}

/// Chaos test results
#[derive(Debug, Clone)]
pub struct ChaosTestResults {
    pub total_chaos_events: usize,
    pub successful_recoveries: usize,
    pub failed_recoveries: usize,
    pub average_recovery_time_ms: f64,
    pub max_recovery_time_ms: u64,
    pub system_availability: f64, // Percentage of time system was available
}

impl ChaosTestResults {
    pub fn recovery_rate(&self) -> f64 {
        if self.total_chaos_events == 0 {
            1.0
        } else {
            self.successful_recoveries as f64 / self.total_chaos_events as f64
        }
    }
    
    pub fn print_summary(&self) {
        println!("\n🌪️  Chaos Test Results:");
        println!("========================");
        println!("Total Chaos Events: {}", self.total_chaos_events);
        println!("Successful Recoveries: {}", self.successful_recoveries);
        println!("Failed Recoveries: {}", self.failed_recoveries);
        println!("Recovery Rate: {:.1}%", self.recovery_rate() * 100.0);
        println!("Average Recovery Time: {:.1} ms", self.average_recovery_time_ms);
        println!("Max Recovery Time: {} ms", self.max_recovery_time_ms);
        println!("System Availability: {:.1}%", self.system_availability * 100.0);
    }
}

/// Run all chaos engineering tests
pub async fn run_all_chaos_tests() -> TestResult {
    init_test_logging();
    info!("🌪️  Starting chaos engineering test suite");
    
    // Skip if not enabled
    if std::env::var("NEXUS_CHAOS_TESTS").is_err() {
        info!("Skipping chaos tests (set NEXUS_CHAOS_TESTS=1 to enable)");
        return Ok(());
    }
    
    let mut failed_tests = Vec::new();
    
    let test_suites = vec![
        ("network_chaos", network_chaos::run_network_chaos_tests),
        ("resource_chaos", resource_chaos::run_resource_chaos_tests),
        ("timing_chaos", timing_chaos::run_timing_chaos_tests),
        ("byzantine_failures", byzantine_failures::run_byzantine_failure_tests),
    ];
    
    for (test_name, test_fn) in test_suites {
        info!("Running {} chaos tests", test_name);
        
        match test_fn().await {
            Ok(()) => {
                info!("✅ {} chaos tests passed", test_name);
            }
            Err(e) => {
                error!("❌ {} chaos tests failed: {}", test_name, e);
                failed_tests.push(test_name);
            }
        }
    }
    
    if failed_tests.is_empty() {
        info!("🎉 All chaos tests completed!");
        Ok(())
    } else {
        error!("Some chaos tests failed: {}", failed_tests.join(", "));
        Err(format!("Chaos tests failed: {}", failed_tests.join(", ")).into())
    }
}

/// Chaos test runner that injects failures during system operation
pub struct ChaosTestRunner {
    config: ChaosConfig,
    active_failures: Vec<ChaosEvent>,
    recovery_times: Vec<Duration>,
    availability_samples: Vec<bool>,
}

#[derive(Debug)]
struct ChaosEvent {
    chaos_type: ChaosType,
    started_at: Instant,
    duration: Duration,
}

impl ChaosTestRunner {
    pub fn new(config: ChaosConfig) -> Self {
        Self {
            config,
            active_failures: Vec::new(),
            recovery_times: Vec::new(),
            availability_samples: Vec::new(),
        }
    }
    
    fn random_chaos_type() -> ChaosType {
        let chaos_types = vec![
            ChaosType::NetworkPartition,
            ChaosType::NetworkLatency(Duration::from_millis(500)),
            ChaosType::NetworkPacketLoss(0.1),
            ChaosType::ResourceExhaustion,
            ChaosType::ProcessCrash,
            ChaosType::TimingSkew(Duration::from_millis(100)),
            ChaosType::RandomFailure,
        ];
        
        chaos_types[thread_rng().gen_range(0..chaos_types.len())].clone()
    }
    
    async fn inject_chaos(chaos_type: &ChaosType) {
        // In a real implementation, this would actually inject the failure
        // For testing, we simulate the injection
        match chaos_type {
            ChaosType::NetworkPartition => {
                info!("🌐 Simulating network partition");
                // Would use iptables or similar to block network traffic
            }
            ChaosType::NetworkLatency(delay) => {
                info!("🐌 Simulating network latency: {:?}", delay);
                // Would use traffic control (tc) to add latency
            }
            ChaosType::NetworkPacketLoss(rate) => {
                info!("📉 Simulating packet loss: {:.1}%", rate * 100.0);
                // Would use tc to drop packets
            }
            ChaosType::ResourceExhaustion => {
                info!("💾 Simulating resource exhaustion");
                // Would consume CPU/memory/disk space
            }
            ChaosType::ProcessCrash => {
                info!("💥 Simulating process crash");
                // Would kill processes
            }
            ChaosType::DiskFull => {
                info!("💿 Simulating disk full");
                // Would fill up disk space
            }
            ChaosType::MemoryExhaustion => {
                info!("🧠 Simulating memory exhaustion");
                // Would allocate large amounts of memory
            }
            ChaosType::TimingSkew(skew) => {
                info!("⏰ Simulating timing skew: {:?}", skew);
                // Would adjust system clock
            }
            ChaosType::ByzantineFailure => {
                info!("🎭 Simulating Byzantine failure");
                // Would make nodes send conflicting messages
            }
            ChaosType::RandomFailure => {
                info!("🎲 Simulating random system failure");
                // Would trigger various random failures
            }
        }
        
        // Simulate injection time
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    async fn stop_chaos(chaos_type: &ChaosType) {
        match chaos_type {
            ChaosType::NetworkPartition => {
                info!("🔧 Restoring network connectivity");
            }
            ChaosType::NetworkLatency(_) => {
                info!("🔧 Removing network latency");
            }
            ChaosType::NetworkPacketLoss(_) => {
                info!("🔧 Stopping packet loss");
            }
            ChaosType::ResourceExhaustion => {
                info!("🔧 Releasing resources");
            }
            ChaosType::ProcessCrash => {
                info!("🔧 Restarting processes");
            }
            ChaosType::DiskFull => {
                info!("🔧 Freeing disk space");
            }
            ChaosType::MemoryExhaustion => {
                info!("🔧 Freeing memory");
            }
            ChaosType::TimingSkew(_) => {
                info!("🔧 Restoring correct time");
            }
            ChaosType::ByzantineFailure => {
                info!("🔧 Fixing Byzantine behavior");
            }
            ChaosType::RandomFailure => {
                info!("🔧 Resolving random failure");
            }
        }
        
        // Simulate recovery time
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_chaos_runner_creation() {
        let config = ChaosConfig::default();
        let runner = ChaosTestRunner::new(config);
        assert_eq!(runner.active_failures.len(), 0);
    }
    
    #[test]
    fn test_chaos_type_generation() {
        let chaos_type = ChaosTestRunner::random_chaos_type();
        // Should generate a valid chaos type
        match chaos_type {
            ChaosType::NetworkPartition | 
            ChaosType::NetworkLatency(_) |
            ChaosType::NetworkPacketLoss(_) |
            ChaosType::ResourceExhaustion |
            ChaosType::ProcessCrash |
            ChaosType::TimingSkew(_) |
            ChaosType::RandomFailure => {},
            _ => panic!("Invalid chaos type generated")
        }
    }
}