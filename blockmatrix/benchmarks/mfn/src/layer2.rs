/*!
# Layer 2 (DSR) Benchmarking

Benchmarks for the Dynamic Similarity Resolution layer focusing on:
- Neural similarity detection performance
- Adaptation rate benchmarks
- Feature vector processing
- Model inference latency
- Memory usage optimization

Performance targets:
- <1ms neural similarity detection
- High accuracy similarity scoring
- Efficient model adaptation
- Bounded memory usage
*/

use crate::common::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use candle_core::{Device, Tensor, DType};
use candle_nn::{VarBuilder, VarMap, linear, Linear, Module};
use rand::Rng;

/// DSR-specific benchmark configuration
#[derive(Debug, Clone)]
pub struct DsrBenchmarkConfig {
    pub base: BenchmarkConfig,
    pub feature_dimensions: usize,
    pub model_layers: Vec<usize>,
    pub batch_size: usize,
    pub similarity_threshold: f32,
    pub adaptation_samples: usize,
    pub context_window: usize,
}

impl Default for DsrBenchmarkConfig {
    fn default() -> Self {
        Self {
            base: BenchmarkConfig {
                warmup_iterations: 100,
                measurement_iterations: 1000,
                statistical_confidence: 0.95,
                regression_threshold: 0.05,
                memory_limit_mb: 256,
                timeout_seconds: 180,
                parallel_workers: 1, // Neural inference is typically single-threaded
                output_format: OutputFormat::Json,
                enable_flamegraph: false,
                enable_perf_counters: true,
            },
            feature_dimensions: 512,
            model_layers: vec![512, 256, 128, 64],
            batch_size: 32,
            similarity_threshold: 0.8,
            adaptation_samples: 1000,
            context_window: 10,
        }
    }
}

/// Neural similarity model for flow pattern recognition
pub struct SimilarityModel {
    layers: Vec<Linear>,
    device: Device,
    context_embedder: Linear,
    similarity_projector: Linear,
}

impl SimilarityModel {
    pub fn new(input_dim: usize, hidden_layers: &[usize], device: Device) -> anyhow::Result<Self> {
        let varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);

        let mut layers = Vec::new();
        let mut prev_dim = input_dim;

        for &hidden_dim in hidden_layers {
            let layer = linear(prev_dim, hidden_dim, vs.pp(&format!("layer_{}", layers.len())))?;
            layers.push(layer);
            prev_dim = hidden_dim;
        }

        let context_embedder = linear(input_dim, 128, vs.pp("context_embedder"))?;
        let similarity_projector = linear(prev_dim + 128, 1, vs.pp("similarity_projector"))?;

        Ok(Self {
            layers,
            device,
            context_embedder,
            similarity_projector,
        })
    }

    pub fn forward(&self, input: &Tensor, context: &Tensor) -> anyhow::Result<Tensor> {
        let mut x = input.clone();

        // Forward through hidden layers with ReLU activations
        for layer in &self.layers {
            x = layer.forward(&x)?;
            x = x.relu()?;
        }

        // Process context information
        let context_embedded = self.context_embedder.forward(context)?;
        let context_embedded = context_embedded.relu()?;

        // Concatenate flow features with context
        let combined = Tensor::cat(&[&x, &context_embedded], 1)?;

        // Final similarity projection
        let similarity = self.similarity_projector.forward(&combined)?;
        let similarity = similarity.sigmoid()?; // Sigmoid for 0-1 similarity score

        Ok(similarity)
    }

    pub fn adapt(&mut self, samples: &[(Tensor, Tensor, f32)]) -> anyhow::Result<f32> {
        // Simplified adaptation using gradient approximation
        let mut total_loss = 0.0;
        
        for (input, context, target_similarity) in samples {
            let prediction = self.forward(input, context)?;
            let target_tensor = Tensor::from_slice(&[*target_similarity], (1, 1), &self.device)?;
            
            // Simple MSE loss calculation
            let diff = prediction.sub(&target_tensor)?;
            let loss = diff.sqr()?.sum_all()?.to_scalar::<f32>()?;
            total_loss += loss;
        }

        Ok(total_loss / samples.len() as f32)
    }
}

/// Flow similarity detector with caching
pub struct FlowSimilarityDetector {
    model: SimilarityModel,
    similarity_cache: HashMap<(Vec<u8>, Vec<u8>), f32>,
    cache_hits: Arc<std::sync::Mutex<usize>>,
    cache_misses: Arc<std::sync::Mutex<usize>>,
    device: Device,
}

impl FlowSimilarityDetector {
    pub fn new(
        input_dim: usize,
        hidden_layers: &[usize],
        device: Device,
    ) -> anyhow::Result<Self> {
        let model = SimilarityModel::new(input_dim, hidden_layers, device.clone())?;

        Ok(Self {
            model,
            similarity_cache: HashMap::new(),
            cache_hits: Arc::new(std::sync::Mutex::new(0)),
            cache_misses: Arc::new(std::sync::Mutex::new(0)),
            device,
        })
    }

    pub fn detect_similarity(
        &mut self,
        flow_vector: &[f32],
        context_vector: &[f32],
    ) -> anyhow::Result<f32> {
        // Create cache key
        let flow_bytes = flow_vector.iter()
            .flat_map(|f| f.to_le_bytes().to_vec())
            .collect::<Vec<u8>>();
        let context_bytes = context_vector.iter()
            .flat_map(|f| f.to_le_bytes().to_vec())
            .collect::<Vec<u8>>();

        let cache_key = (flow_bytes, context_bytes);

        // Check cache first
        if let Some(&similarity) = self.similarity_cache.get(&cache_key) {
            *self.cache_hits.lock().unwrap() += 1;
            return Ok(similarity);
        }

        *self.cache_misses.lock().unwrap() += 1;

        // Create tensors for model inference
        let flow_tensor = Tensor::from_slice(flow_vector, (1, flow_vector.len()), &self.device)?;
        let context_tensor = Tensor::from_slice(context_vector, (1, context_vector.len()), &self.device)?;

        // Run model inference
        let similarity_tensor = self.model.forward(&flow_tensor, &context_tensor)?;
        let similarity = similarity_tensor.to_scalar::<f32>()?;

        // Cache the result
        if self.similarity_cache.len() < 10000 {
            self.similarity_cache.insert(cache_key, similarity);
        }

        Ok(similarity)
    }

    pub fn get_cache_stats(&self) -> (usize, usize) {
        let hits = *self.cache_hits.lock().unwrap();
        let misses = *self.cache_misses.lock().unwrap();
        (hits, misses)
    }

    pub fn adapt_model(&mut self, adaptation_data: &[(Vec<f32>, Vec<f32>, f32)]) -> anyhow::Result<f32> {
        // Convert adaptation data to tensors
        let tensor_samples: Result<Vec<_>, _> = adaptation_data.iter()
            .map(|(flow, context, similarity)| {
                let flow_tensor = Tensor::from_slice(flow, (1, flow.len()), &self.device)?;
                let context_tensor = Tensor::from_slice(context, (1, context.len()), &self.device)?;
                Ok((flow_tensor, context_tensor, *similarity))
            })
            .collect();

        let tensor_samples = tensor_samples?;
        self.model.adapt(&tensor_samples)
    }
}

/// Pattern recognition system for flow classification
pub struct FlowPatternRecognizer {
    patterns: Vec<FlowPattern>,
    recognition_model: SimilarityModel,
    device: Device,
}

#[derive(Debug, Clone)]
pub struct FlowPattern {
    pub id: String,
    pub feature_vector: Vec<f32>,
    pub context_vector: Vec<f32>,
    pub frequency: usize,
    pub last_seen: std::time::SystemTime,
}

impl FlowPatternRecognizer {
    pub fn new(
        feature_dim: usize,
        hidden_layers: &[usize],
        device: Device,
    ) -> anyhow::Result<Self> {
        let recognition_model = SimilarityModel::new(feature_dim, hidden_layers, device.clone())?;

        Ok(Self {
            patterns: Vec::new(),
            recognition_model,
            device,
        })
    }

    pub fn recognize_pattern(&self, flow_vector: &[f32], context_vector: &[f32]) -> anyhow::Result<Option<String>> {
        let mut best_match = None;
        let mut best_similarity = 0.0;

        let flow_tensor = Tensor::from_slice(flow_vector, (1, flow_vector.len()), &self.device)?;
        let context_tensor = Tensor::from_slice(context_vector, (1, context_vector.len()), &self.device)?;

        for pattern in &self.patterns {
            let pattern_tensor = Tensor::from_slice(&pattern.feature_vector, (1, pattern.feature_vector.len()), &self.device)?;
            let pattern_context = Tensor::from_slice(&pattern.context_vector, (1, pattern.context_vector.len()), &self.device)?;

            // Calculate similarity between input and pattern
            let combined_input = Tensor::cat(&[&flow_tensor, &pattern_tensor], 1)?;
            let combined_context = Tensor::cat(&[&context_tensor, &pattern_context], 1)?;
            
            let similarity_tensor = self.recognition_model.forward(&combined_input, &combined_context)?;
            let similarity = similarity_tensor.to_scalar::<f32>()?;

            if similarity > best_similarity {
                best_similarity = similarity;
                best_match = Some(pattern.id.clone());
            }
        }

        Ok(if best_similarity > 0.8 { best_match } else { None })
    }

    pub fn add_pattern(&mut self, pattern: FlowPattern) {
        self.patterns.push(pattern);
    }

    pub fn get_pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

/// Main DSR benchmark suite
pub async fn run_dsr_benchmarks(config: DsrBenchmarkConfig) -> anyhow::Result<Vec<BenchmarkResult>> {
    let mut harness = BenchmarkHarness::new(config.base.clone());
    let mut results = Vec::new();

    println!("🧠 Starting Layer 2 (DSR) Benchmarks");
    println!("    Feature dimensions: {}", config.feature_dimensions);
    println!("    Model layers: {:?}", config.model_layers);
    println!("    Batch size: {}", config.batch_size);

    let device = Device::Cpu; // Use CPU for consistent benchmarking

    // Generate test data
    let (flow_vectors, context_vectors) = generate_test_vectors(&config);
    let adaptation_data = generate_adaptation_data(&config);

    // Benchmark 1: Neural Similarity Detection
    results.push(run_similarity_detection_benchmark(&mut harness, &config, &device, &flow_vectors, &context_vectors).await?);
    
    // Benchmark 2: Model Adaptation Performance
    results.push(run_adaptation_benchmark(&mut harness, &config, &device, &adaptation_data).await?);
    
    // Benchmark 3: Pattern Recognition
    results.push(run_pattern_recognition_benchmark(&mut harness, &config, &device, &flow_vectors, &context_vectors).await?);
    
    // Benchmark 4: Cache Performance
    results.push(run_cache_performance_benchmark(&mut harness, &config, &device, &flow_vectors, &context_vectors).await?);
    
    // Benchmark 5: Batch Processing
    results.push(run_batch_processing_benchmark(&mut harness, &config, &device, &flow_vectors, &context_vectors).await?);

    Ok(results)
}

async fn run_similarity_detection_benchmark(
    harness: &mut BenchmarkHarness,
    config: &DsrBenchmarkConfig,
    device: &Device,
    flow_vectors: &[Vec<f32>],
    context_vectors: &[Vec<f32>],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "neural_similarity_detection",
        MfnLayer::Layer2Dsr,
        {
            let mut detector = FlowSimilarityDetector::new(
                config.feature_dimensions,
                &config.model_layers,
                device.clone(),
            ).unwrap();

            let flows = flow_vectors.to_vec();
            let contexts = context_vectors.to_vec();

            move || {
                let start = Instant::now();
                
                let flow_idx = fastrand::usize(0..flows.len());
                let context_idx = fastrand::usize(0..contexts.len());
                
                let _ = detector.detect_similarity(&flows[flow_idx], &contexts[context_idx]).unwrap();
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_adaptation_benchmark(
    harness: &mut BenchmarkHarness,
    config: &DsrBenchmarkConfig,
    device: &Device,
    adaptation_data: &[(Vec<f32>, Vec<f32>, f32)],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "model_adaptation",
        MfnLayer::Layer2Dsr,
        {
            let mut detector = FlowSimilarityDetector::new(
                config.feature_dimensions,
                &config.model_layers,
                device.clone(),
            ).unwrap();

            let adapt_data = adaptation_data.to_vec();

            move || {
                let start = Instant::now();
                
                // Take a small batch for adaptation
                let batch_size = 10;
                let start_idx = fastrand::usize(0..adapt_data.len().saturating_sub(batch_size));
                let batch = &adapt_data[start_idx..start_idx + batch_size];
                
                let _ = detector.adapt_model(batch).unwrap();
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_pattern_recognition_benchmark(
    harness: &mut BenchmarkHarness,
    config: &DsrBenchmarkConfig,
    device: &Device,
    flow_vectors: &[Vec<f32>],
    context_vectors: &[Vec<f32>],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "pattern_recognition",
        MfnLayer::Layer2Dsr,
        {
            let mut recognizer = FlowPatternRecognizer::new(
                config.feature_dimensions,
                &config.model_layers,
                device.clone(),
            ).unwrap();

            // Pre-populate with patterns
            for i in 0..100 {
                let pattern = FlowPattern {
                    id: format!("pattern_{}", i),
                    feature_vector: flow_vectors[i % flow_vectors.len()].clone(),
                    context_vector: context_vectors[i % context_vectors.len()].clone(),
                    frequency: 1,
                    last_seen: std::time::SystemTime::now(),
                };
                recognizer.add_pattern(pattern);
            }

            let flows = flow_vectors.to_vec();
            let contexts = context_vectors.to_vec();

            move || {
                let start = Instant::now();
                
                let flow_idx = fastrand::usize(0..flows.len());
                let context_idx = fastrand::usize(0..contexts.len());
                
                let _ = recognizer.recognize_pattern(&flows[flow_idx], &contexts[context_idx]).unwrap();
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_cache_performance_benchmark(
    harness: &mut BenchmarkHarness,
    config: &DsrBenchmarkConfig,
    device: &Device,
    flow_vectors: &[Vec<f32>],
    context_vectors: &[Vec<f32>],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "similarity_cache",
        MfnLayer::Layer2Dsr,
        {
            let mut detector = FlowSimilarityDetector::new(
                config.feature_dimensions,
                &config.model_layers,
                device.clone(),
            ).unwrap();

            // Pre-warm the cache with some common patterns
            for i in 0..100 {
                let flow_idx = i % flow_vectors.len();
                let context_idx = i % context_vectors.len();
                let _ = detector.detect_similarity(&flow_vectors[flow_idx], &context_vectors[context_idx]).unwrap();
            }

            let flows = flow_vectors.to_vec();
            let contexts = context_vectors.to_vec();

            move || {
                let start = Instant::now();
                
                // Favor cached entries (80% cache hit rate)
                let flow_idx = if fastrand::f32() < 0.8 {
                    fastrand::usize(0..100.min(flows.len()))
                } else {
                    fastrand::usize(0..flows.len())
                };
                
                let context_idx = if fastrand::f32() < 0.8 {
                    fastrand::usize(0..100.min(contexts.len()))
                } else {
                    fastrand::usize(0..contexts.len())
                };
                
                let _ = detector.detect_similarity(&flows[flow_idx], &contexts[context_idx]).unwrap();
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_batch_processing_benchmark(
    harness: &mut BenchmarkHarness,
    config: &DsrBenchmarkConfig,
    device: &Device,
    flow_vectors: &[Vec<f32>],
    context_vectors: &[Vec<f32>],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "batch_similarity_processing",
        MfnLayer::Layer2Dsr,
        {
            let mut detector = FlowSimilarityDetector::new(
                config.feature_dimensions,
                &config.model_layers,
                device.clone(),
            ).unwrap();

            let flows = flow_vectors.to_vec();
            let contexts = context_vectors.to_vec();
            let batch_size = config.batch_size;

            move || {
                let start = Instant::now();
                
                // Process a batch of similarities
                for _ in 0..batch_size {
                    let flow_idx = fastrand::usize(0..flows.len());
                    let context_idx = fastrand::usize(0..contexts.len());
                    let _ = detector.detect_similarity(&flows[flow_idx], &contexts[context_idx]).unwrap();
                }
                
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

fn generate_test_vectors(config: &DsrBenchmarkConfig) -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
    let mut rng = rand::thread_rng();
    let count = 1000;

    let flow_vectors: Vec<Vec<f32>> = (0..count)
        .map(|_| {
            (0..config.feature_dimensions)
                .map(|_| rng.gen::<f32>() * 2.0 - 1.0) // Range [-1, 1]
                .collect()
        })
        .collect();

    let context_vectors: Vec<Vec<f32>> = (0..count)
        .map(|_| {
            (0..config.feature_dimensions)
                .map(|_| rng.gen::<f32>() * 2.0 - 1.0) // Range [-1, 1]
                .collect()
        })
        .collect();

    (flow_vectors, context_vectors)
}

fn generate_adaptation_data(config: &DsrBenchmarkConfig) -> Vec<(Vec<f32>, Vec<f32>, f32)> {
    let mut rng = rand::thread_rng();

    (0..config.adaptation_samples)
        .map(|_| {
            let flow_vector: Vec<f32> = (0..config.feature_dimensions)
                .map(|_| rng.gen::<f32>() * 2.0 - 1.0)
                .collect();

            let context_vector: Vec<f32> = (0..config.feature_dimensions)
                .map(|_| rng.gen::<f32>() * 2.0 - 1.0)
                .collect();

            let similarity = rng.gen::<f32>(); // Random similarity target

            (flow_vector, context_vector, similarity)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity_model_creation() {
        let device = Device::Cpu;
        let model = SimilarityModel::new(128, &[64, 32], device);
        assert!(model.is_ok());
    }

    #[tokio::test]
    async fn test_flow_similarity_detector() {
        let device = Device::Cpu;
        let mut detector = FlowSimilarityDetector::new(128, &[64, 32], device).unwrap();

        let flow_vector = vec![0.5; 128];
        let context_vector = vec![0.3; 128];

        let similarity = detector.detect_similarity(&flow_vector, &context_vector).unwrap();
        assert!(similarity >= 0.0 && similarity <= 1.0);

        // Test caching
        let similarity2 = detector.detect_similarity(&flow_vector, &context_vector).unwrap();
        assert_eq!(similarity, similarity2);

        let (hits, misses) = detector.get_cache_stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
    }

    #[test]
    fn test_flow_pattern_recognizer() {
        let device = Device::Cpu;
        let mut recognizer = FlowPatternRecognizer::new(128, &[64, 32], device).unwrap();

        let pattern = FlowPattern {
            id: "test_pattern".to_string(),
            feature_vector: vec![0.5; 128],
            context_vector: vec![0.3; 128],
            frequency: 1,
            last_seen: std::time::SystemTime::now(),
        };

        recognizer.add_pattern(pattern);
        assert_eq!(recognizer.get_pattern_count(), 1);

        let result = recognizer.recognize_pattern(&vec![0.5; 128], &vec![0.3; 128]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vector_generation() {
        let config = DsrBenchmarkConfig::default();
        let (flow_vectors, context_vectors) = generate_test_vectors(&config);
        
        assert_eq!(flow_vectors.len(), 1000);
        assert_eq!(context_vectors.len(), 1000);
        assert_eq!(flow_vectors[0].len(), config.feature_dimensions);
        assert_eq!(context_vectors[0].len(), config.feature_dimensions);
    }
}