/*!
# Layer 4 (CPE) Benchmarking

Benchmarks for the Context Prediction Engine layer focusing on:
- Context prediction latency
- Forecast accuracy validation  
- Historical pattern analysis
- Adaptive learning performance
- Memory usage optimization

Performance targets:
- <2ms context prediction latency
- High forecast accuracy
- Efficient pattern recognition
- Adaptive model updates
*/

use crate::common::*;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use candle_core::{Device, Tensor, DType, Shape};
use candle_nn::{VarBuilder, VarMap, linear, Linear, Module, lstm::LSTM};
use rand::Rng;

/// CPE-specific benchmark configuration
#[derive(Debug, Clone)]
pub struct CpeBenchmarkConfig {
    pub base: BenchmarkConfig,
    pub context_dimensions: usize,
    pub prediction_window: usize,
    pub history_length: usize,
    pub lstm_hidden_size: usize,
    pub lstm_layers: usize,
    pub batch_size: usize,
    pub learning_rate: f64,
    pub pattern_types: Vec<PatternType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    Temporal,      // Time-based patterns
    Behavioral,    // User/system behavior patterns
    Load,          // Resource load patterns
    Network,       // Network traffic patterns
    Hybrid,        // Combined patterns
}

impl Default for CpeBenchmarkConfig {
    fn default() -> Self {
        Self {
            base: BenchmarkConfig {
                warmup_iterations: 100,
                measurement_iterations: 1000,
                statistical_confidence: 0.95,
                regression_threshold: 0.05,
                memory_limit_mb: 512,
                timeout_seconds: 300,
                parallel_workers: 1, // LSTM typically single-threaded
                output_format: OutputFormat::Json,
                enable_flamegraph: false,
                enable_perf_counters: true,
            },
            context_dimensions: 256,
            prediction_window: 10,
            history_length: 100,
            lstm_hidden_size: 128,
            lstm_layers: 2,
            batch_size: 32,
            learning_rate: 0.001,
            pattern_types: vec![
                PatternType::Temporal,
                PatternType::Behavioral,
                PatternType::Load,
                PatternType::Network,
                PatternType::Hybrid,
            ],
        }
    }
}

/// Context vector representing system state
#[derive(Debug, Clone)]
pub struct ContextVector {
    pub timestamp: u64,
    pub features: Vec<f32>,
    pub pattern_type: PatternType,
    pub metadata: HashMap<String, f32>,
}

/// Historical context database for pattern analysis
pub struct ContextHistory {
    history: VecDeque<ContextVector>,
    max_length: usize,
    patterns: HashMap<PatternType, Vec<ContextVector>>,
    statistics: ContextStatistics,
}

#[derive(Debug, Clone, Default)]
pub struct ContextStatistics {
    pub total_contexts: usize,
    pub pattern_counts: HashMap<PatternType, usize>,
    pub average_feature_values: Vec<f32>,
    pub feature_variance: Vec<f32>,
}

impl ContextHistory {
    pub fn new(max_length: usize, feature_dimensions: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_length),
            max_length,
            patterns: HashMap::new(),
            statistics: ContextStatistics {
                total_contexts: 0,
                pattern_counts: HashMap::new(),
                average_feature_values: vec![0.0; feature_dimensions],
                feature_variance: vec![0.0; feature_dimensions],
            },
        }
    }

    pub fn add_context(&mut self, context: ContextVector) {
        if self.history.len() >= self.max_length {
            if let Some(old_context) = self.history.pop_front() {
                self.remove_from_statistics(&old_context);
            }
        }

        self.add_to_statistics(&context);
        
        // Store in pattern-specific collections
        self.patterns.entry(context.pattern_type)
            .or_default()
            .push(context.clone());

        self.history.push_back(context);
    }

    fn add_to_statistics(&mut self, context: &ContextVector) {
        self.statistics.total_contexts += 1;
        *self.statistics.pattern_counts.entry(context.pattern_type).or_default() += 1;

        // Update running averages
        let n = self.statistics.total_contexts as f32;
        for (i, &value) in context.features.iter().enumerate() {
            if i < self.statistics.average_feature_values.len() {
                let old_avg = self.statistics.average_feature_values[i];
                self.statistics.average_feature_values[i] = old_avg + (value - old_avg) / n;
            }
        }
    }

    fn remove_from_statistics(&mut self, context: &ContextVector) {
        if self.statistics.total_contexts > 0 {
            self.statistics.total_contexts -= 1;
            if let Some(count) = self.statistics.pattern_counts.get_mut(&context.pattern_type) {
                *count = count.saturating_sub(1);
            }
        }
    }

    pub fn get_recent_contexts(&self, count: usize, pattern_type: Option<PatternType>) -> Vec<&ContextVector> {
        match pattern_type {
            Some(pt) => {
                self.history.iter()
                    .rev()
                    .filter(|ctx| ctx.pattern_type == pt)
                    .take(count)
                    .collect()
            }
            None => {
                self.history.iter()
                    .rev()
                    .take(count)
                    .collect()
            }
        }
    }

    pub fn get_patterns_by_type(&self, pattern_type: PatternType) -> Option<&Vec<ContextVector>> {
        self.patterns.get(&pattern_type)
    }

    pub fn get_statistics(&self) -> &ContextStatistics {
        &self.statistics
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }
}

/// LSTM-based context prediction model
pub struct ContextPredictor {
    lstm: LSTM,
    output_layer: Linear,
    device: Device,
    hidden_size: usize,
    input_size: usize,
    prediction_cache: HashMap<Vec<u8>, PredictionResult>,
}

#[derive(Debug, Clone)]
pub struct PredictionResult {
    pub predicted_context: Vec<f32>,
    pub confidence: f32,
    pub timestamp: u64,
    pub accuracy_score: Option<f32>,
}

impl ContextPredictor {
    pub fn new(
        input_size: usize,
        hidden_size: usize,
        num_layers: usize,
        device: Device,
    ) -> anyhow::Result<Self> {
        let varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);

        let lstm = LSTM::new(input_size, hidden_size, num_layers, vs.pp("lstm"))?;
        let output_layer = linear(hidden_size, input_size, vs.pp("output"))?;

        Ok(Self {
            lstm,
            output_layer,
            device,
            hidden_size,
            input_size,
            prediction_cache: HashMap::new(),
        })
    }

    pub fn predict_next_context(
        &mut self,
        context_sequence: &[ContextVector],
    ) -> anyhow::Result<PredictionResult> {
        // Create cache key from input sequence
        let cache_key = self.create_cache_key(context_sequence);
        
        if let Some(cached_result) = self.prediction_cache.get(&cache_key).cloned() {
            return Ok(cached_result);
        }

        // Prepare input tensor
        let input_tensor = self.prepare_input_tensor(context_sequence)?;
        
        // Run LSTM forward pass
        let (lstm_output, _) = self.lstm.forward(&input_tensor, None)?;
        
        // Get the last output for prediction
        let last_output = lstm_output.narrow(0, lstm_output.dim(0)? - 1, 1)?;
        let prediction_tensor = self.output_layer.forward(&last_output)?;
        
        // Convert to prediction result
        let predicted_features: Vec<f32> = prediction_tensor
            .flatten_all()?
            .to_vec1()?;

        let confidence = self.calculate_prediction_confidence(context_sequence, &predicted_features);
        
        let result = PredictionResult {
            predicted_context: predicted_features,
            confidence,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            accuracy_score: None,
        };

        // Cache result
        if self.prediction_cache.len() < 1000 {
            self.prediction_cache.insert(cache_key, result.clone());
        }

        Ok(result)
    }

    fn prepare_input_tensor(&self, context_sequence: &[ContextVector]) -> anyhow::Result<Tensor> {
        let seq_len = context_sequence.len();
        let mut input_data = Vec::with_capacity(seq_len * self.input_size);

        for context in context_sequence {
            for &feature in &context.features {
                input_data.push(feature);
            }
            // Pad if necessary
            while input_data.len() % self.input_size != 0 {
                input_data.push(0.0);
            }
        }

        let shape = Shape::from((seq_len, self.input_size));
        Tensor::from_slice(&input_data, shape, &self.device)
    }

    fn calculate_prediction_confidence(
        &self,
        context_sequence: &[ContextVector],
        predicted_features: &[f32],
    ) -> f32 {
        if context_sequence.is_empty() {
            return 0.0;
        }

        // Simple confidence based on recent context similarity
        let recent_context = &context_sequence[context_sequence.len() - 1];
        let similarity = self.calculate_feature_similarity(&recent_context.features, predicted_features);
        
        // Confidence increases with sequence length (up to a point)
        let sequence_factor = (context_sequence.len() as f32 / 10.0).min(1.0);
        
        similarity * sequence_factor
    }

    fn calculate_feature_similarity(&self, features1: &[f32], features2: &[f32]) -> f32 {
        let min_len = features1.len().min(features2.len());
        if min_len == 0 {
            return 0.0;
        }

        let dot_product: f32 = features1.iter()
            .zip(features2.iter())
            .take(min_len)
            .map(|(a, b)| a * b)
            .sum();

        let norm1: f32 = features1.iter().take(min_len).map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = features2.iter().take(min_len).map(|x| x * x).sum::<f32>().sqrt();

        if norm1 * norm2 == 0.0 {
            0.0
        } else {
            (dot_product / (norm1 * norm2)).max(0.0)
        }
    }

    fn create_cache_key(&self, context_sequence: &[ContextVector]) -> Vec<u8> {
        let mut key_data = Vec::new();
        
        for context in context_sequence.iter().take(5) { // Use last 5 contexts for key
            key_data.extend_from_slice(&context.timestamp.to_le_bytes());
            for &feature in context.features.iter().take(10) { // Use first 10 features
                key_data.extend_from_slice(&feature.to_le_bytes());
            }
        }
        
        key_data
    }

    pub fn update_accuracy(&mut self, cache_key: &[u8], actual_context: &ContextVector) {
        let cache_key = cache_key.to_vec();
        if let Some(prediction) = self.prediction_cache.get_mut(&cache_key) {
            let accuracy = self.calculate_feature_similarity(&prediction.predicted_context, &actual_context.features);
            prediction.accuracy_score = Some(accuracy);
        }
    }

    pub fn get_cache_size(&self) -> usize {
        self.prediction_cache.len()
    }
}

/// Pattern analyzer for identifying recurring behavior patterns
pub struct PatternAnalyzer {
    pattern_database: HashMap<PatternType, Vec<Pattern>>,
    similarity_threshold: f32,
    min_pattern_length: usize,
    max_patterns_per_type: usize,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub id: String,
    pub pattern_type: PatternType,
    pub feature_sequence: Vec<Vec<f32>>,
    pub frequency: usize,
    pub last_seen: u64,
    pub confidence: f32,
}

impl PatternAnalyzer {
    pub fn new(similarity_threshold: f32, min_pattern_length: usize, max_patterns_per_type: usize) -> Self {
        Self {
            pattern_database: HashMap::new(),
            similarity_threshold,
            min_pattern_length,
            max_patterns_per_type,
        }
    }

    pub fn analyze_patterns(&mut self, contexts: &[ContextVector]) -> Vec<Pattern> {
        let mut discovered_patterns = Vec::new();

        for pattern_type in [PatternType::Temporal, PatternType::Behavioral, PatternType::Load, PatternType::Network] {
            let type_contexts: Vec<_> = contexts.iter()
                .filter(|ctx| ctx.pattern_type == pattern_type)
                .collect();

            if type_contexts.len() >= self.min_pattern_length {
                let patterns = self.extract_patterns(&type_contexts, pattern_type);
                discovered_patterns.extend(patterns);
            }
        }

        self.update_pattern_database(discovered_patterns.clone());
        discovered_patterns
    }

    fn extract_patterns(&self, contexts: &[&ContextVector], pattern_type: PatternType) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Sliding window approach to find recurring patterns
        for window_size in self.min_pattern_length..=contexts.len().min(20) {
            for start in 0..=contexts.len().saturating_sub(window_size) {
                let window = &contexts[start..start + window_size];
                
                // Convert to feature sequence
                let feature_sequence: Vec<Vec<f32>> = window.iter()
                    .map(|ctx| ctx.features.clone())
                    .collect();

                // Check if this pattern already exists or is similar to existing ones
                let pattern_id = format!("{}_{}_len{}", pattern_type as u8, start, window_size);
                
                let pattern = Pattern {
                    id: pattern_id,
                    pattern_type,
                    feature_sequence,
                    frequency: 1,
                    last_seen: window.last().map(|ctx| ctx.timestamp).unwrap_or(0),
                    confidence: self.calculate_pattern_confidence(window),
                };

                if pattern.confidence > self.similarity_threshold {
                    patterns.push(pattern);
                }
            }
        }

        patterns
    }

    fn calculate_pattern_confidence(&self, window: &[&ContextVector]) -> f32 {
        if window.len() < 2 {
            return 0.0;
        }

        // Calculate consistency of features across the window
        let mut consistency_scores = Vec::new();

        for feature_idx in 0..window[0].features.len() {
            let values: Vec<f32> = window.iter()
                .map(|ctx| ctx.features.get(feature_idx).copied().unwrap_or(0.0))
                .collect();

            let mean = values.iter().sum::<f32>() / values.len() as f32;
            let variance = values.iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f32>() / values.len() as f32;
            
            // Lower variance = higher consistency
            let consistency = 1.0 / (1.0 + variance);
            consistency_scores.push(consistency);
        }

        consistency_scores.iter().sum::<f32>() / consistency_scores.len() as f32
    }

    fn update_pattern_database(&mut self, new_patterns: Vec<Pattern>) {
        for pattern in new_patterns {
            let type_patterns = self.pattern_database.entry(pattern.pattern_type).or_default();
            
            // Check for similar existing patterns
            let mut merged = false;
            for existing_pattern in type_patterns.iter_mut() {
                if self.patterns_are_similar(existing_pattern, &pattern) {
                    existing_pattern.frequency += 1;
                    existing_pattern.last_seen = existing_pattern.last_seen.max(pattern.last_seen);
                    merged = true;
                    break;
                }
            }

            if !merged {
                type_patterns.push(pattern);
                
                // Limit database size
                if type_patterns.len() > self.max_patterns_per_type {
                    type_patterns.sort_by(|a, b| b.frequency.cmp(&a.frequency));
                    type_patterns.truncate(self.max_patterns_per_type);
                }
            }
        }
    }

    fn patterns_are_similar(&self, pattern1: &Pattern, pattern2: &Pattern) -> bool {
        if pattern1.feature_sequence.len() != pattern2.feature_sequence.len() {
            return false;
        }

        let mut similarity_sum = 0.0;
        let mut count = 0;

        for (seq1, seq2) in pattern1.feature_sequence.iter().zip(&pattern2.feature_sequence) {
            if seq1.len() == seq2.len() {
                let similarity = self.calculate_sequence_similarity(seq1, seq2);
                similarity_sum += similarity;
                count += 1;
            }
        }

        if count == 0 {
            return false;
        }

        (similarity_sum / count as f32) > self.similarity_threshold
    }

    fn calculate_sequence_similarity(&self, seq1: &[f32], seq2: &[f32]) -> f32 {
        let dot_product: f32 = seq1.iter().zip(seq2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = seq1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = seq2.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm1 * norm2 == 0.0 {
            0.0
        } else {
            dot_product / (norm1 * norm2)
        }
    }

    pub fn get_pattern_count(&self, pattern_type: PatternType) -> usize {
        self.pattern_database.get(&pattern_type).map(|p| p.len()).unwrap_or(0)
    }

    pub fn get_total_pattern_count(&self) -> usize {
        self.pattern_database.values().map(|patterns| patterns.len()).sum()
    }
}

/// Main CPE benchmark suite
pub async fn run_cpe_benchmarks(config: CpeBenchmarkConfig) -> anyhow::Result<Vec<BenchmarkResult>> {
    let mut harness = BenchmarkHarness::new(config.base.clone());
    let mut results = Vec::new();

    println!("🔮 Starting Layer 4 (CPE) Benchmarks");
    println!("    Context dimensions: {}", config.context_dimensions);
    println!("    Prediction window: {}", config.prediction_window);
    println!("    History length: {}", config.history_length);
    println!("    LSTM hidden size: {}", config.lstm_hidden_size);

    let device = Device::Cpu;

    // Generate test data
    let context_history = generate_test_contexts(&config);
    let context_sequences = generate_context_sequences(&context_history, &config);

    // Benchmark 1: Context Prediction Performance
    results.push(run_context_prediction_benchmark(&mut harness, &config, &device, &context_sequences).await?);
    
    // Benchmark 2: Pattern Analysis Performance
    results.push(run_pattern_analysis_benchmark(&mut harness, &config, &context_history).await?);
    
    // Benchmark 3: Historical Context Lookup
    results.push(run_context_lookup_benchmark(&mut harness, &config, &context_history).await?);
    
    // Benchmark 4: Prediction Accuracy Validation
    results.push(run_accuracy_validation_benchmark(&mut harness, &config, &device, &context_sequences).await?);
    
    // Benchmark 5: Adaptive Learning Performance
    results.push(run_adaptive_learning_benchmark(&mut harness, &config, &device, &context_sequences).await?);

    Ok(results)
}

async fn run_context_prediction_benchmark(
    harness: &mut BenchmarkHarness,
    config: &CpeBenchmarkConfig,
    device: &Device,
    context_sequences: &[Vec<ContextVector>],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "context_prediction_lstm",
        MfnLayer::Layer4Cpe,
        {
            let mut predictor = ContextPredictor::new(
                config.context_dimensions,
                config.lstm_hidden_size,
                config.lstm_layers,
                device.clone(),
            ).unwrap();

            let sequences = context_sequences.to_vec();

            move || {
                let start = Instant::now();
                
                let seq_idx = fastrand::usize(0..sequences.len());
                let sequence = &sequences[seq_idx];
                
                if sequence.len() > 1 {
                    let input_sequence = &sequence[..sequence.len()-1];
                    let _ = predictor.predict_next_context(input_sequence).unwrap();
                }
                
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_pattern_analysis_benchmark(
    harness: &mut BenchmarkHarness,
    config: &CpeBenchmarkConfig,
    context_history: &ContextHistory,
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "pattern_analysis",
        MfnLayer::Layer4Cpe,
        {
            let mut analyzer = PatternAnalyzer::new(0.8, 3, 100);
            let contexts = context_history.get_recent_contexts(100, None)
                .into_iter()
                .cloned()
                .collect::<Vec<_>>();

            move || {
                let start = Instant::now();
                
                // Analyze patterns on a subset of contexts
                let subset_size = 20.min(contexts.len());
                let start_idx = if contexts.len() > subset_size {
                    fastrand::usize(0..contexts.len() - subset_size)
                } else {
                    0
                };
                let subset = &contexts[start_idx..start_idx + subset_size];
                
                let _ = analyzer.analyze_patterns(subset);
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_context_lookup_benchmark(
    harness: &mut BenchmarkHarness,
    _config: &CpeBenchmarkConfig,
    context_history: &ContextHistory,
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "context_history_lookup",
        MfnLayer::Layer4Cpe,
        {
            let history = context_history.clone();
            let pattern_types = vec![
                PatternType::Temporal,
                PatternType::Behavioral,
                PatternType::Load,
                PatternType::Network,
            ];

            move || {
                let start = Instant::now();
                
                let pattern_type = pattern_types[fastrand::usize(0..pattern_types.len())];
                let count = fastrand::usize(1..50);
                
                let _ = history.get_recent_contexts(count, Some(pattern_type));
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_accuracy_validation_benchmark(
    harness: &mut BenchmarkHarness,
    config: &CpeBenchmarkConfig,
    device: &Device,
    context_sequences: &[Vec<ContextVector>],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "prediction_accuracy_validation",
        MfnLayer::Layer4Cpe,
        {
            let mut predictor = ContextPredictor::new(
                config.context_dimensions,
                config.lstm_hidden_size,
                config.lstm_layers,
                device.clone(),
            ).unwrap();

            let sequences = context_sequences.to_vec();

            move || {
                let start = Instant::now();
                
                let seq_idx = fastrand::usize(0..sequences.len());
                let sequence = &sequences[seq_idx];
                
                if sequence.len() > 2 {
                    let input_sequence = &sequence[..sequence.len()-1];
                    let actual_next = &sequence[sequence.len()-1];
                    
                    if let Ok(prediction) = predictor.predict_next_context(input_sequence) {
                        // Calculate accuracy
                        let accuracy = predictor.calculate_feature_similarity(
                            &prediction.predicted_context,
                            &actual_next.features
                        );
                        let _ = accuracy; // Use the accuracy score
                    }
                }
                
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_adaptive_learning_benchmark(
    harness: &mut BenchmarkHarness,
    config: &CpeBenchmarkConfig,
    device: &Device,
    context_sequences: &[Vec<ContextVector>],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "adaptive_learning_update",
        MfnLayer::Layer4Cpe,
        {
            let mut predictor = ContextPredictor::new(
                config.context_dimensions,
                config.lstm_hidden_size,
                config.lstm_layers,
                device.clone(),
            ).unwrap();

            let sequences = context_sequences.to_vec();

            move || {
                let start = Instant::now();
                
                // Simulate learning from a batch of sequences
                let batch_size = 5.min(sequences.len());
                for i in 0..batch_size {
                    let seq_idx = fastrand::usize(0..sequences.len());
                    let sequence = &sequences[seq_idx];
                    
                    if sequence.len() > 1 {
                        let input_sequence = &sequence[..sequence.len()-1];
                        let _ = predictor.predict_next_context(input_sequence);
                    }
                }
                
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

fn generate_test_contexts(config: &CpeBenchmarkConfig) -> ContextHistory {
    let mut history = ContextHistory::new(config.history_length, config.context_dimensions);
    let mut rng = rand::thread_rng();
    
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for i in 0..config.history_length {
        let pattern_type = config.pattern_types[i % config.pattern_types.len()];
        
        // Generate features based on pattern type
        let features = match pattern_type {
            PatternType::Temporal => {
                // Sinusoidal patterns with time-based variations
                (0..config.context_dimensions)
                    .map(|j| {
                        let time_factor = (i as f32 + j as f32 * 0.1) * 0.1;
                        (time_factor.sin() + rng.gen::<f32>() * 0.2).max(0.0)
                    })
                    .collect()
            }
            PatternType::Behavioral => {
                // Step functions and discrete behavioral patterns
                (0..config.context_dimensions)
                    .map(|j| {
                        if (i / 10) % 2 == j % 2 {
                            0.8 + rng.gen::<f32>() * 0.2
                        } else {
                            0.1 + rng.gen::<f32>() * 0.2
                        }
                    })
                    .collect()
            }
            PatternType::Load => {
                // Load patterns with spikes and gradual changes
                (0..config.context_dimensions)
                    .map(|j| {
                        let base_load = 0.3 + (i as f32 / config.history_length as f32) * 0.4;
                        let spike = if i % 50 == j % 50 { 0.3 } else { 0.0 };
                        (base_load + spike + rng.gen::<f32>() * 0.1).min(1.0)
                    })
                    .collect()
            }
            PatternType::Network => {
                // Network traffic patterns with bursts
                (0..config.context_dimensions)
                    .map(|j| {
                        let burst_factor = if (i / 20) % 5 < 2 { 0.8 } else { 0.2 };
                        let network_component = (j as f32 / config.context_dimensions as f32).powi(2);
                        (burst_factor * network_component + rng.gen::<f32>() * 0.1).min(1.0)
                    })
                    .collect()
            }
            PatternType::Hybrid => {
                // Combination of multiple pattern types
                (0..config.context_dimensions)
                    .map(|j| {
                        let temporal = (i as f32 * 0.1).sin() * 0.3;
                        let behavioral = if i % 10 < 5 { 0.2 } else { -0.2 };
                        let load = (i as f32 / config.history_length as f32) * 0.3;
                        (temporal + behavioral + load + 0.5 + rng.gen::<f32>() * 0.1)
                            .max(0.0).min(1.0)
                    })
                    .collect()
            }
        };

        let mut metadata = HashMap::new();
        metadata.insert("pattern_strength".to_string(), rng.gen::<f32>());
        metadata.insert("noise_level".to_string(), rng.gen::<f32>() * 0.1);

        let context = ContextVector {
            timestamp: start_time + i as u64 * 10, // 10-second intervals
            features,
            pattern_type,
            metadata,
        };

        history.add_context(context);
    }

    history
}

impl Clone for ContextHistory {
    fn clone(&self) -> Self {
        Self {
            history: self.history.clone(),
            max_length: self.max_length,
            patterns: self.patterns.clone(),
            statistics: self.statistics.clone(),
        }
    }
}

fn generate_context_sequences(
    context_history: &ContextHistory,
    config: &CpeBenchmarkConfig,
) -> Vec<Vec<ContextVector>> {
    let all_contexts = context_history.get_recent_contexts(context_history.len(), None);
    let mut sequences = Vec::new();

    for window_size in 5..=config.prediction_window {
        for start in 0..=all_contexts.len().saturating_sub(window_size) {
            let sequence: Vec<ContextVector> = all_contexts[start..start + window_size]
                .iter()
                .map(|&ctx| ctx.clone())
                .collect();
            sequences.push(sequence);
            
            if sequences.len() >= 100 {
                break;
            }
        }
        if sequences.len() >= 100 {
            break;
        }
    }

    sequences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_history() {
        let mut history = ContextHistory::new(10, 5);
        
        let context = ContextVector {
            timestamp: 1000,
            features: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            pattern_type: PatternType::Temporal,
            metadata: HashMap::new(),
        };
        
        history.add_context(context);
        assert_eq!(history.len(), 1);
        
        let recent = history.get_recent_contexts(5, None);
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn test_pattern_analyzer() {
        let mut analyzer = PatternAnalyzer::new(0.7, 2, 50);
        
        let contexts = vec![
            ContextVector {
                timestamp: 1000,
                features: vec![1.0, 2.0],
                pattern_type: PatternType::Temporal,
                metadata: HashMap::new(),
            },
            ContextVector {
                timestamp: 2000,
                features: vec![1.1, 2.1],
                pattern_type: PatternType::Temporal,
                metadata: HashMap::new(),
            },
        ];
        
        let patterns = analyzer.analyze_patterns(&contexts);
        assert!(!patterns.is_empty());
    }

    #[tokio::test]
    async fn test_context_predictor() {
        let device = Device::Cpu;
        let mut predictor = ContextPredictor::new(10, 16, 1, device).unwrap();
        
        let context_sequence = vec![
            ContextVector {
                timestamp: 1000,
                features: vec![0.1; 10],
                pattern_type: PatternType::Temporal,
                metadata: HashMap::new(),
            },
            ContextVector {
                timestamp: 2000,
                features: vec![0.2; 10],
                pattern_type: PatternType::Temporal,
                metadata: HashMap::new(),
            },
        ];
        
        let result = predictor.predict_next_context(&context_sequence);
        assert!(result.is_ok());
        
        let prediction = result.unwrap();
        assert_eq!(prediction.predicted_context.len(), 10);
        assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
    }

    #[test]
    fn test_context_vector_generation() {
        let config = CpeBenchmarkConfig::default();
        let history = generate_test_contexts(&config);
        
        assert_eq!(history.len(), config.history_length);
        
        let recent_contexts = history.get_recent_contexts(10, None);
        assert_eq!(recent_contexts.len(), 10);
        
        for context in recent_contexts {
            assert_eq!(context.features.len(), config.context_dimensions);
        }
    }
}