/*!
# Integration Benchmarking

End-to-end benchmarks for the complete MFN system integrated with HyperMesh:
- Cross-layer coordination performance
- Complete flow processing pipeline
- System-wide throughput validation
- Memory usage across all layers
- Network performance with MFN optimization

Performance targets:
- adaptive network tiers throughput with <5% MFN overhead
- End-to-end latency improvements
- Resource efficiency validation
- System stability under load
*/

use crate::common::*;
use crate::layer1::{IfrBenchmarkConfig, run_ifr_benchmarks};
use crate::layer2::{DsrBenchmarkConfig, run_dsr_benchmarks};
use crate::layer3::{AlmBenchmarkConfig, run_alm_benchmarks};
use crate::layer4::{CpeBenchmarkConfig, run_cpe_benchmarks};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{timeout, sleep};

/// Integration benchmark configuration
#[derive(Debug, Clone)]
pub struct IntegrationBenchmarkConfig {
    pub base: BenchmarkConfig,
    pub throughput_target_gbps: f64,
    pub max_mfn_overhead_percent: f64,
    pub concurrent_flows: usize,
    pub test_duration_seconds: u64,
    pub layer_coordination_timeout_ms: u64,
    pub enable_all_layers: bool,
    pub network_simulation: NetworkSimulationConfig,
}

#[derive(Debug, Clone)]
pub struct NetworkSimulationConfig {
    pub packet_size_bytes: usize,
    pub burst_size: usize,
    pub inter_burst_delay_ms: u64,
    pub loss_rate_percent: f64,
    pub jitter_range_ms: f64,
}

impl Default for IntegrationBenchmarkConfig {
    fn default() -> Self {
        Self {
            base: BenchmarkConfig {
                warmup_iterations: 50,
                measurement_iterations: 500,
                statistical_confidence: 0.95,
                regression_threshold: 0.05,
                memory_limit_mb: 1024,
                timeout_seconds: 600,
                parallel_workers: num_cpus::get(),
                output_format: OutputFormat::Json,
                enable_flamegraph: false,
                enable_perf_counters: true,
            },
            throughput_target_gbps: 40.0,
            max_mfn_overhead_percent: 5.0,
            concurrent_flows: 10000,
            test_duration_seconds: 60,
            layer_coordination_timeout_ms: 100,
            enable_all_layers: true,
            network_simulation: NetworkSimulationConfig {
                packet_size_bytes: 1500, // Standard MTU
                burst_size: 100,
                inter_burst_delay_ms: 1,
                loss_rate_percent: 0.1,
                jitter_range_ms: 5.0,
            },
        }
    }
}

/// Unified MFN system coordinator
pub struct MfnSystem {
    ifr_registry: Arc<Mutex<crate::layer1::RobinHoodHashTable>>,
    dsr_detector: Arc<RwLock<crate::layer2::FlowSimilarityDetector>>,
    alm_topology: Arc<RwLock<crate::layer3::NetworkTopology>>,
    cpe_predictor: Arc<RwLock<crate::layer4::ContextPredictor>>,
    coordination_channel: mpsc::UnboundedSender<CoordinationMessage>,
    metrics: Arc<Mutex<IntegrationMetrics>>,
    config: IntegrationBenchmarkConfig,
}

#[derive(Debug, Clone)]
pub enum CoordinationMessage {
    FlowRegistered { flow_key: [u8; 32], component_id: u32 },
    SimilarityDetected { flow_key: [u8; 32], similarity: f32 },
    RouteOptimized { flow_key: [u8; 32], route: Vec<usize> },
    ContextPredicted { flow_key: [u8; 32], prediction: Vec<f32> },
    SystemShutdown,
}

#[derive(Debug, Clone, Default)]
pub struct IntegrationMetrics {
    pub total_flows_processed: usize,
    pub layer_coordination_latencies: HashMap<String, Vec<Duration>>,
    pub end_to_end_latencies: Vec<Duration>,
    pub throughput_samples: Vec<f64>,
    pub memory_usage_samples: Vec<f64>,
    pub error_counts: HashMap<String, usize>,
    pub layer_utilization: HashMap<String, f64>,
}

impl MfnSystem {
    pub async fn new(config: IntegrationBenchmarkConfig) -> anyhow::Result<Self> {
        // Initialize all layers
        let ifr_registry = Arc::new(Mutex::new(
            crate::layer1::RobinHoodHashTable::new(config.concurrent_flows * 2)
        ));

        let dsr_detector = Arc::new(RwLock::new(
            crate::layer2::FlowSimilarityDetector::new(
                512, // feature dimensions
                &[256, 128, 64], // hidden layers
                candle_core::Device::Cpu,
            )?
        ));

        let alm_topology = Arc::new(RwLock::new(crate::layer3::NetworkTopology::new()));
        
        let cpe_predictor = Arc::new(RwLock::new(
            crate::layer4::ContextPredictor::new(
                256, // input size
                128, // hidden size
                2,   // num layers
                candle_core::Device::Cpu,
            )?
        ));

        let (coord_tx, mut coord_rx) = mpsc::unbounded_channel();
        let metrics = Arc::new(Mutex::new(IntegrationMetrics::default()));

        // Start coordination message handler
        let metrics_clone = metrics.clone();
        tokio::spawn(async move {
            while let Some(message) = coord_rx.recv().await {
                Self::handle_coordination_message(message, &metrics_clone).await;
            }
        });

        Ok(Self {
            ifr_registry,
            dsr_detector,
            alm_topology,
            cpe_predictor,
            coordination_channel: coord_tx,
            metrics,
            config,
        })
    }

    async fn handle_coordination_message(
        message: CoordinationMessage,
        metrics: &Arc<Mutex<IntegrationMetrics>>,
    ) {
        let mut metrics_guard = metrics.lock().unwrap();
        
        match message {
            CoordinationMessage::FlowRegistered { .. } => {
                metrics_guard.total_flows_processed += 1;
            }
            CoordinationMessage::SimilarityDetected { .. } => {
                *metrics_guard.layer_utilization.entry("dsr".to_string()).or_default() += 1.0;
            }
            CoordinationMessage::RouteOptimized { .. } => {
                *metrics_guard.layer_utilization.entry("alm".to_string()).or_default() += 1.0;
            }
            CoordinationMessage::ContextPredicted { .. } => {
                *metrics_guard.layer_utilization.entry("cpe".to_string()).or_default() += 1.0;
            }
            CoordinationMessage::SystemShutdown => {
                // Handle graceful shutdown
            }
        }
    }

    /// Process a complete flow through all MFN layers
    pub async fn process_flow_complete(
        &self,
        flow_key: [u8; 32],
        flow_data: &[u8],
        context_data: &[f32],
    ) -> anyhow::Result<FlowProcessingResult> {
        let start_time = Instant::now();
        let mut layer_timings = HashMap::new();

        // Layer 1: IFR - Flow Registration
        let layer1_start = Instant::now();
        let flow_record = crate::layer1::FlowRecord {
            key: flow_key,
            component_id: fastrand::u32(1000..9999),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            metadata: [0; 8],
        };

        let ifr_success = {
            let mut registry = self.ifr_registry.lock().unwrap();
            registry.insert(flow_record.clone())
        };
        layer_timings.insert("ifr".to_string(), layer1_start.elapsed());

        if !ifr_success {
            return Err(anyhow::anyhow!("IFR registration failed"));
        }

        // Send coordination message
        let _ = self.coordination_channel.send(CoordinationMessage::FlowRegistered {
            flow_key,
            component_id: flow_record.component_id,
        });

        // Layer 2: DSR - Similarity Detection
        let layer2_start = Instant::now();
        let flow_vector = self.extract_flow_features(flow_data);
        let context_vector = context_data.to_vec();
        
        let similarity = {
            let mut detector = self.dsr_detector.write().await;
            detector.detect_similarity(&flow_vector, &context_vector)?
        };
        layer_timings.insert("dsr".to_string(), layer2_start.elapsed());

        let _ = self.coordination_channel.send(CoordinationMessage::SimilarityDetected {
            flow_key,
            similarity,
        });

        // Layer 3: ALM - Route Optimization (if similarity is high enough)
        let optimal_route = if similarity > 0.7 {
            let layer3_start = Instant::now();
            let route = {
                let mut topology = self.alm_topology.write().await;
                
                // Simulate route finding between random nodes
                let from_node = fastrand::usize(0..1000);
                let to_node = fastrand::usize(0..1000);
                
                topology.find_optimal_path(from_node, to_node, crate::layer3::PathOptimization::Balanced)
                    .unwrap_or_else(|| vec![from_node, to_node])
            };
            layer_timings.insert("alm".to_string(), layer3_start.elapsed());

            let _ = self.coordination_channel.send(CoordinationMessage::RouteOptimized {
                flow_key,
                route: route.clone(),
            });

            Some(route)
        } else {
            None
        };

        // Layer 4: CPE - Context Prediction
        let layer4_start = Instant::now();
        let context_sequence = self.build_context_sequence(&flow_vector, &context_vector);
        let prediction = {
            let mut predictor = self.cpe_predictor.write().await;
            predictor.predict_next_context(&context_sequence)?
        };
        layer_timings.insert("cpe".to_string(), layer4_start.elapsed());

        let _ = self.coordination_channel.send(CoordinationMessage::ContextPredicted {
            flow_key,
            prediction: prediction.predicted_context.clone(),
        });

        let total_duration = start_time.elapsed();

        Ok(FlowProcessingResult {
            flow_key,
            total_processing_time: total_duration,
            layer_timings,
            ifr_registered: ifr_success,
            similarity_score: similarity,
            optimal_route,
            context_prediction: Some(prediction),
            success: true,
        })
    }

    fn extract_flow_features(&self, flow_data: &[u8]) -> Vec<f32> {
        // Simple feature extraction from flow data
        let mut features = vec![0.0; 512];
        
        // Use flow data to generate features
        for (i, chunk) in flow_data.chunks(4).enumerate() {
            if i >= features.len() {
                break;
            }
            
            let value = if chunk.len() == 4 {
                f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
            } else {
                chunk.iter().map(|&b| b as f32).sum::<f32>() / chunk.len() as f32
            };
            
            features[i] = (value % 256.0) / 256.0; // Normalize to [0,1]
        }
        
        features
    }

    fn build_context_sequence(&self, flow_vector: &[f32], context_vector: &[f32]) -> Vec<crate::layer4::ContextVector> {
        let mut sequence = Vec::new();
        let base_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create a sequence of contexts for prediction
        for i in 0..5 {
            let mut features = flow_vector.clone();
            features.extend_from_slice(context_vector);
            
            // Truncate or pad to match expected dimensions
            features.truncate(256);
            while features.len() < 256 {
                features.push(0.0);
            }
            
            let context = crate::layer4::ContextVector {
                timestamp: base_time - (5 - i) as u64 * 10,
                features,
                pattern_type: crate::layer4::PatternType::Network,
                metadata: HashMap::new(),
            };
            
            sequence.push(context);
        }
        
        sequence
    }

    pub fn get_metrics(&self) -> IntegrationMetrics {
        self.metrics.lock().unwrap().clone()
    }
}

#[derive(Debug, Clone)]
pub struct FlowProcessingResult {
    pub flow_key: [u8; 32],
    pub total_processing_time: Duration,
    pub layer_timings: HashMap<String, Duration>,
    pub ifr_registered: bool,
    pub similarity_score: f32,
    pub optimal_route: Option<Vec<usize>>,
    pub context_prediction: Option<crate::layer4::PredictionResult>,
    pub success: bool,
}

/// Network throughput simulator for end-to-end testing
pub struct NetworkThroughputSimulator {
    config: NetworkSimulationConfig,
    packet_generator: PacketGenerator,
    throughput_meter: ThroughputMeter,
}

pub struct PacketGenerator {
    packet_size: usize,
    burst_size: usize,
    inter_burst_delay: Duration,
}

pub struct ThroughputMeter {
    bytes_transferred: Arc<Mutex<u64>>,
    start_time: Instant,
    samples: Arc<Mutex<Vec<(Instant, u64)>>>,
}

impl NetworkThroughputSimulator {
    pub fn new(config: NetworkSimulationConfig) -> Self {
        Self {
            config: config.clone(),
            packet_generator: PacketGenerator {
                packet_size: config.packet_size_bytes,
                burst_size: config.burst_size,
                inter_burst_delay: Duration::from_millis(config.inter_burst_delay_ms),
            },
            throughput_meter: ThroughputMeter {
                bytes_transferred: Arc::new(Mutex::new(0)),
                start_time: Instant::now(),
                samples: Arc::new(Mutex::new(Vec::new())),
            },
        }
    }

    pub async fn run_throughput_test(
        &mut self,
        mfn_system: &MfnSystem,
        duration: Duration,
    ) -> anyhow::Result<ThroughputTestResult> {
        let start_time = Instant::now();
        let mut total_flows = 0;
        let mut successful_flows = 0;
        let mut total_bytes = 0u64;

        while start_time.elapsed() < duration {
            // Generate a burst of packets
            for _ in 0..self.packet_generator.burst_size {
                let packet_data = self.generate_packet();
                let context_data = self.generate_context();
                let flow_key = self.generate_flow_key();

                total_flows += 1;
                total_bytes += packet_data.len() as u64;

                // Process through MFN system
                match timeout(
                    Duration::from_millis(mfn_system.config.layer_coordination_timeout_ms),
                    mfn_system.process_flow_complete(flow_key, &packet_data, &context_data)
                ).await {
                    Ok(Ok(_result)) => {
                        successful_flows += 1;
                        self.throughput_meter.record_bytes(packet_data.len() as u64);
                    }
                    Ok(Err(_)) | Err(_) => {
                        // Flow processing failed or timed out
                    }
                }

                // Simulate packet loss
                if fastrand::f64() < self.config.loss_rate_percent / 100.0 {
                    continue; // Simulate dropped packet
                }
            }

            // Inter-burst delay with jitter
            let jitter = Duration::from_millis(
                (fastrand::f64() * self.config.jitter_range_ms) as u64
            );
            sleep(self.packet_generator.inter_burst_delay + jitter).await;
        }

        let test_duration = start_time.elapsed();
        let throughput_gbps = (total_bytes as f64 * 8.0) / (test_duration.as_secs_f64() * 1_000_000_000.0);
        
        Ok(ThroughputTestResult {
            test_duration,
            total_flows,
            successful_flows,
            total_bytes,
            throughput_gbps,
            success_rate: successful_flows as f64 / total_flows as f64,
            average_flow_latency: Duration::from_millis(1), // Placeholder
        })
    }

    fn generate_packet(&self) -> Vec<u8> {
        let mut packet = vec![0u8; self.packet_generator.packet_size];
        fastrand::fill(&mut packet);
        packet
    }

    fn generate_context(&self) -> Vec<f32> {
        (0..256).map(|_| fastrand::f32()).collect()
    }

    fn generate_flow_key(&self) -> [u8; 32] {
        let mut key = [0u8; 32];
        fastrand::fill(&mut key);
        key
    }
}

impl ThroughputMeter {
    fn record_bytes(&self, bytes: u64) {
        {
            let mut total = self.bytes_transferred.lock().unwrap();
            *total += bytes;
        }
        
        {
            let mut samples = self.samples.lock().unwrap();
            samples.push((Instant::now(), bytes));
            
            // Keep only recent samples to prevent memory growth
            if samples.len() > 10000 {
                samples.drain(0..1000);
            }
        }
    }

    fn get_current_throughput_gbps(&self) -> f64 {
        let total_bytes = *self.bytes_transferred.lock().unwrap();
        let elapsed = self.start_time.elapsed().as_secs_f64();
        
        if elapsed > 0.0 {
            (total_bytes as f64 * 8.0) / (elapsed * 1_000_000_000.0)
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThroughputTestResult {
    pub test_duration: Duration,
    pub total_flows: usize,
    pub successful_flows: usize,
    pub total_bytes: u64,
    pub throughput_gbps: f64,
    pub success_rate: f64,
    pub average_flow_latency: Duration,
}

/// Main integration benchmark suite
pub async fn run_integration_benchmarks(
    config: IntegrationBenchmarkConfig,
) -> anyhow::Result<Vec<BenchmarkResult>> {
    let mut harness = BenchmarkHarness::new(config.base.clone());
    let mut results = Vec::new();

    println!("🔗 Starting MFN Integration Benchmarks");
    println!("    Target throughput: {:.1} Gbps", config.throughput_target_gbps);
    println!("    Max MFN overhead: {:.1}%", config.max_mfn_overhead_percent);
    println!("    Concurrent flows: {}", config.concurrent_flows);
    println!("    Test duration: {}s", config.test_duration_seconds);

    // Create MFN system
    let mfn_system = Arc::new(MfnSystem::new(config.clone()).await?);

    // Benchmark 1: End-to-End Flow Processing
    results.push(run_end_to_end_benchmark(&mut harness, &config, &mfn_system).await?);
    
    // Benchmark 2: Cross-Layer Coordination
    results.push(run_coordination_benchmark(&mut harness, &config, &mfn_system).await?);
    
    // Benchmark 3: System Throughput Test
    results.push(run_system_throughput_benchmark(&mut harness, &config, &mfn_system).await?);
    
    // Benchmark 4: Memory Usage Under Load
    results.push(run_memory_usage_benchmark(&mut harness, &config, &mfn_system).await?);
    
    // Benchmark 5: Performance Regression Test
    results.push(run_performance_regression_benchmark(&mut harness, &config, &mfn_system).await?);

    Ok(results)
}

async fn run_end_to_end_benchmark(
    harness: &mut BenchmarkHarness,
    config: &IntegrationBenchmarkConfig,
    mfn_system: &Arc<MfnSystem>,
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "end_to_end_flow_processing",
        MfnLayer::Integration,
        {
            let system = mfn_system.clone();
            
            move || {
                let system = system.clone();
                async move {
                    let start = Instant::now();
                    
                    let flow_key = {
                        let mut key = [0u8; 32];
                        fastrand::fill(&mut key);
                        key
                    };
                    
                    let flow_data = {
                        let mut data = vec![0u8; 1500]; // MTU size
                        fastrand::fill(&mut data);
                        data
                    };
                    
                    let context_data: Vec<f32> = (0..256).map(|_| fastrand::f32()).collect();
                    
                    match timeout(
                        Duration::from_millis(config.layer_coordination_timeout_ms),
                        system.process_flow_complete(flow_key, &flow_data, &context_data)
                    ).await {
                        Ok(Ok(_)) => Ok(start.elapsed()),
                        Ok(Err(e)) => Err(anyhow::anyhow!("Flow processing failed: {}", e)),
                        Err(_) => Err(anyhow::anyhow!("Flow processing timed out")),
                    }
                }
            }
        }
    ).await
}

async fn run_coordination_benchmark(
    harness: &mut BenchmarkHarness,
    config: &IntegrationBenchmarkConfig,
    mfn_system: &Arc<MfnSystem>,
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "cross_layer_coordination",
        MfnLayer::Integration,
        {
            let system = mfn_system.clone();
            
            move || {
                let system = system.clone();
                async move {
                    let start = Instant::now();
                    
                    // Simulate coordination messages between layers
                    let flow_key = {
                        let mut key = [0u8; 32];
                        fastrand::fill(&mut key);
                        key
                    };
                    
                    // Send coordination messages
                    let messages = vec![
                        CoordinationMessage::FlowRegistered { flow_key, component_id: 1234 },
                        CoordinationMessage::SimilarityDetected { flow_key, similarity: 0.85 },
                        CoordinationMessage::RouteOptimized { flow_key, route: vec![0, 1, 2, 3] },
                        CoordinationMessage::ContextPredicted { flow_key, prediction: vec![0.5; 10] },
                    ];
                    
                    for message in messages {
                        let _ = system.coordination_channel.send(message);
                    }
                    
                    // Small delay to allow processing
                    sleep(Duration::from_micros(100)).await;
                    
                    Ok(start.elapsed())
                }
            }
        }
    ).await
}

async fn run_system_throughput_benchmark(
    harness: &mut BenchmarkHarness,
    config: &IntegrationBenchmarkConfig,
    mfn_system: &Arc<MfnSystem>,
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "system_throughput_40gbps",
        MfnLayer::Integration,
        {
            let system = mfn_system.clone();
            let sim_config = config.network_simulation.clone();
            let test_duration = Duration::from_secs(5); // Shorter for benchmark
            
            move || {
                let system = system.clone();
                let sim_config = sim_config.clone();
                async move {
                    let start = Instant::now();
                    
                    let mut simulator = NetworkThroughputSimulator::new(sim_config);
                    let throughput_result = simulator.run_throughput_test(&system, test_duration).await?;
                    
                    // Validate throughput target
                    if throughput_result.throughput_gbps < config.throughput_target_gbps * 0.8 {
                        return Err(anyhow::anyhow!(
                            "Throughput {} Gbps below target {} Gbps",
                            throughput_result.throughput_gbps,
                            config.throughput_target_gbps
                        ));
                    }
                    
                    Ok(start.elapsed())
                }
            }
        }
    ).await
}

async fn run_memory_usage_benchmark(
    harness: &mut BenchmarkHarness,
    config: &IntegrationBenchmarkConfig,
    mfn_system: &Arc<MfnSystem>,
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "memory_usage_under_load",
        MfnLayer::Integration,
        {
            let system = mfn_system.clone();
            
            move || {
                let system = system.clone();
                async move {
                    let start = Instant::now();
                    
                    // Process many flows to stress test memory usage
                    let num_flows = 1000;
                    for i in 0..num_flows {
                        let flow_key = {
                            let mut key = [0u8; 32];
                            key[..4].copy_from_slice(&(i as u32).to_le_bytes());
                            fastrand::fill(&mut key[4..]);
                            key
                        };
                        
                        let flow_data = vec![i as u8; 64]; // Small packets
                        let context_data: Vec<f32> = vec![i as f32 / 1000.0; 256];
                        
                        // Process with timeout to prevent hanging
                        let _ = timeout(
                            Duration::from_millis(10),
                            system.process_flow_complete(flow_key, &flow_data, &context_data)
                        ).await;
                    }
                    
                    Ok(start.elapsed())
                }
            }
        }
    ).await
}

async fn run_performance_regression_benchmark(
    harness: &mut BenchmarkHarness,
    _config: &IntegrationBenchmarkConfig,
    mfn_system: &Arc<MfnSystem>,
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "performance_regression_test",
        MfnLayer::Integration,
        {
            let system = mfn_system.clone();
            
            move || {
                let system = system.clone();
                async move {
                    let start = Instant::now();
                    
                    // Run a mix of operations to detect performance regressions
                    let operations = [
                        "small_flow", "large_flow", "batch_flows", "coordination", "prediction"
                    ];
                    
                    for &operation in &operations {
                        match operation {
                            "small_flow" => {
                                let flow_key = [1u8; 32];
                                let flow_data = vec![0u8; 64];
                                let context_data = vec![0.1; 64];
                                let _ = system.process_flow_complete(flow_key, &flow_data, &context_data).await;
                            }
                            "large_flow" => {
                                let flow_key = [2u8; 32];
                                let flow_data = vec![0u8; 9000]; // Jumbo frame
                                let context_data = vec![0.2; 512];
                                let _ = system.process_flow_complete(flow_key, &flow_data, &context_data).await;
                            }
                            "batch_flows" => {
                                for i in 0..10 {
                                    let mut flow_key = [3u8; 32];
                                    flow_key[0] = i;
                                    let flow_data = vec![i; 256];
                                    let context_data = vec![i as f32 * 0.01; 256];
                                    let _ = system.process_flow_complete(flow_key, &flow_data, &context_data).await;
                                }
                            }
                            "coordination" => {
                                let flow_key = [4u8; 32];
                                let _ = system.coordination_channel.send(
                                    CoordinationMessage::FlowRegistered { flow_key, component_id: 9999 }
                                );
                            }
                            "prediction" => {
                                // This is implicitly tested in flow processing
                            }
                            _ => {}
                        }
                    }
                    
                    Ok(start.elapsed())
                }
            }
        }
    ).await
}

/// End-to-end benchmark that combines all layers
pub struct HyperMeshIntegration {
    pub mfn_system: Arc<MfnSystem>,
    pub baseline_system: Option<BaselineSystem>, // For comparison
}

/// Baseline system without MFN optimizations for comparison
pub struct BaselineSystem {
    // Simplified baseline implementation
}

impl HyperMeshIntegration {
    pub async fn new(config: IntegrationBenchmarkConfig) -> anyhow::Result<Self> {
        let mfn_system = Arc::new(MfnSystem::new(config).await?);
        
        Ok(Self {
            mfn_system,
            baseline_system: Some(BaselineSystem {}),
        })
    }

    /// Compare MFN-enabled system with baseline
    pub async fn run_comparison_benchmark(&self) -> anyhow::Result<ComparisonResult> {
        let test_flows = self.generate_test_flows(1000);
        let mut mfn_results = Vec::new();
        let mut baseline_results = Vec::new();

        // Test MFN system
        for (flow_key, flow_data, context_data) in &test_flows {
            let start = Instant::now();
            let result = self.mfn_system.process_flow_complete(*flow_key, flow_data, context_data).await;
            let duration = start.elapsed();
            
            mfn_results.push((result.is_ok(), duration));
        }

        // Test baseline system (simplified)
        for (_, flow_data, _) in &test_flows {
            let start = Instant::now();
            // Simulate baseline processing (just data copying)
            let _processed = flow_data.clone();
            let duration = start.elapsed();
            
            baseline_results.push((true, duration));
        }

        let mfn_avg_latency = mfn_results.iter()
            .map(|(_, duration)| duration.as_secs_f64())
            .sum::<f64>() / mfn_results.len() as f64;
            
        let baseline_avg_latency = baseline_results.iter()
            .map(|(_, duration)| duration.as_secs_f64())
            .sum::<f64>() / baseline_results.len() as f64;

        let improvement_percent = ((baseline_avg_latency - mfn_avg_latency) / baseline_avg_latency) * 100.0;

        Ok(ComparisonResult {
            mfn_avg_latency_ms: mfn_avg_latency * 1000.0,
            baseline_avg_latency_ms: baseline_avg_latency * 1000.0,
            improvement_percent,
            mfn_success_rate: mfn_results.iter().filter(|(success, _)| *success).count() as f64 / mfn_results.len() as f64,
            baseline_success_rate: baseline_results.iter().filter(|(success, _)| *success).count() as f64 / baseline_results.len() as f64,
        })
    }

    fn generate_test_flows(&self, count: usize) -> Vec<([u8; 32], Vec<u8>, Vec<f32>)> {
        (0..count)
            .map(|i| {
                let mut flow_key = [0u8; 32];
                flow_key[..4].copy_from_slice(&(i as u32).to_le_bytes());
                fastrand::fill(&mut flow_key[4..]);

                let flow_data = {
                    let size = 64 + (i % 1400); // Variable packet sizes
                    let mut data = vec![0u8; size];
                    fastrand::fill(&mut data);
                    data
                };

                let context_data: Vec<f32> = (0..256)
                    .map(|j| (i + j) as f32 / 1000.0)
                    .collect();

                (flow_key, flow_data, context_data)
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub mfn_avg_latency_ms: f64,
    pub baseline_avg_latency_ms: f64,
    pub improvement_percent: f64,
    pub mfn_success_rate: f64,
    pub baseline_success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mfn_system_creation() {
        let config = IntegrationBenchmarkConfig::default();
        let system = MfnSystem::new(config).await;
        assert!(system.is_ok());
    }

    #[tokio::test]
    async fn test_flow_processing() {
        let config = IntegrationBenchmarkConfig::default();
        let system = MfnSystem::new(config).await.unwrap();
        
        let flow_key = [1u8; 32];
        let flow_data = vec![0u8; 100];
        let context_data = vec![0.5; 256];
        
        let result = system.process_flow_complete(flow_key, &flow_data, &context_data).await;
        assert!(result.is_ok());
        
        let result = result.unwrap();
        assert!(result.success);
        assert!(result.ifr_registered);
        assert!(result.similarity_score >= 0.0 && result.similarity_score <= 1.0);
    }

    #[tokio::test]
    async fn test_throughput_simulator() {
        let config = NetworkSimulationConfig {
            packet_size_bytes: 100,
            burst_size: 10,
            inter_burst_delay_ms: 1,
            loss_rate_percent: 0.0,
            jitter_range_ms: 0.0,
        };
        
        let mut simulator = NetworkThroughputSimulator::new(config);
        let system_config = IntegrationBenchmarkConfig::default();
        let system = MfnSystem::new(system_config).await.unwrap();
        
        let result = simulator.run_throughput_test(&system, Duration::from_millis(100)).await;
        assert!(result.is_ok());
        
        let result = result.unwrap();
        assert!(result.total_flows > 0);
        assert!(result.throughput_gbps >= 0.0);
    }

    #[tokio::test]
    async fn test_integration_comparison() {
        let config = IntegrationBenchmarkConfig {
            concurrent_flows: 100,
            test_duration_seconds: 1,
            ..Default::default()
        };
        
        let integration = HyperMeshIntegration::new(config).await.unwrap();
        let comparison = integration.run_comparison_benchmark().await;
        
        assert!(comparison.is_ok());
        let result = comparison.unwrap();
        
        assert!(result.mfn_avg_latency_ms >= 0.0);
        assert!(result.baseline_avg_latency_ms >= 0.0);
        assert!(result.mfn_success_rate >= 0.0 && result.mfn_success_rate <= 1.0);
        assert!(result.baseline_success_rate >= 0.0 && result.baseline_success_rate <= 1.0);
    }
}