/*!
# Layer 4 (CPE) Criterion Benchmarks

Context prediction and adaptive learning benchmarks for the Context Prediction Engine layer.
Tests LSTM models, pattern prediction, and adaptive context management.
*/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mfn_benchmarks::{common::*, layer4::*};
use std::time::Duration;

fn bench_lstm_context_prediction(c: &mut Criterion) {
    let mut group = c.benchmark_group("lstm_context_prediction");
    group.measurement_time(Duration::from_secs(25));
    
    // Test different LSTM configurations
    let sequence_lengths = [16, 32, 64, 128];
    let hidden_sizes = [64, 128, 256];
    let context_dimensions = [128, 256, 512];
    
    for &seq_len in sequence_lengths.iter() {
        for &hidden_size in hidden_sizes.iter() {
            for &context_dim in context_dimensions.iter() {
                let mut lstm_predictor = LstmContextPredictor::new(
                    context_dim, 
                    hidden_size, 
                    seq_len
                );
                
                // Generate test sequences
                let test_sequences = generate_context_sequences(50, seq_len, context_dim);
                
                group.throughput(Throughput::Elements(test_sequences.len() as u64));
                group.bench_with_input(
                    BenchmarkId::new(
                        "lstm_prediction", 
                        format!("seq{}_h{}_d{}", seq_len, hidden_size, context_dim)
                    ),
                    &(&lstm_predictor, &test_sequences),
                    |b, (predictor, sequences)| {
                        b.iter(|| {
                            let mut predictions = Vec::new();
                            for sequence in sequences.iter() {
                                let prediction = predictor.predict_next_context(sequence);
                                predictions.push(black_box(prediction));
                            }
                            black_box(predictions)
                        })
                    },
                );
            }
        }
    }
    
    group.finish();
}

fn bench_pattern_learning(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_learning");
    group.measurement_time(Duration::from_secs(20));
    
    let pattern_complexities = [10, 50, 100, 500]; // Number of unique patterns
    let learning_rates = [0.001, 0.01, 0.1];
    
    for &complexity in pattern_complexities.iter() {
        for &learning_rate in learning_rates.iter() {
            let mut pattern_learner = PatternLearner::new(256, complexity, learning_rate);
            
            // Generate training patterns
            let training_patterns = generate_training_patterns(complexity * 10, 256);
            
            group.throughput(Throughput::Elements(training_patterns.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("pattern_training", format!("{}patterns_lr{}", complexity, learning_rate)),
                &(&mut pattern_learner, &training_patterns),
                |b, (learner, patterns)| {
                    b.iter(|| {
                        for pattern in patterns.iter() {
                            let learning_result = learner.learn_pattern(pattern);
                            black_box(learning_result);
                        }
                        
                        // Update model weights
                        let update_result = learner.update_weights();
                        black_box(update_result)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_context_similarity_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_similarity_matching");
    
    let context_database_sizes = [1000, 5000, 10000, 50000];
    let query_batch_sizes = [10, 50, 100];
    
    for &db_size in context_database_sizes.iter() {
        for &batch_size in query_batch_sizes.iter() {
            let mut context_matcher = ContextSimilarityMatcher::new(256);
            
            // Pre-populate context database
            let stored_contexts = generate_stored_contexts(db_size, 256);
            for context in &stored_contexts {
                context_matcher.add_context(context.clone());
            }
            
            // Generate query contexts
            let query_contexts = generate_query_contexts(batch_size, 256, &stored_contexts);
            
            group.throughput(Throughput::Elements(query_contexts.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("similarity_search", format!("{}db_{}queries", db_size, batch_size)),
                &(&context_matcher, &query_contexts),
                |b, (matcher, queries)| {
                    b.iter(|| {
                        let mut matches = Vec::new();
                        for query in queries.iter() {
                            let similar_contexts = matcher.find_similar_contexts(query, 10);
                            matches.push(black_box(similar_contexts));
                        }
                        black_box(matches)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_adaptive_context_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("adaptive_context_management");
    group.measurement_time(Duration::from_secs(30));
    
    let context_window_sizes = [50, 100, 200, 500];
    let adaptation_frequencies = [10, 25, 50]; // Context updates before adaptation
    
    for &window_size in context_window_sizes.iter() {
        for &frequency in adaptation_frequencies.iter() {
            let mut adaptive_manager = AdaptiveContextManager::new(256, window_size);
            
            // Generate streaming context data
            let context_stream = generate_context_stream(frequency * 20, 256);
            
            group.throughput(Throughput::Elements(context_stream.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("adaptive_management", format!("w{}_f{}", window_size, frequency)),
                &(&mut adaptive_manager, &context_stream, frequency),
                |b, (manager, stream, freq)| {
                    b.iter(|| {
                        for (i, context) in stream.iter().enumerate() {
                            manager.update_context(context);
                            
                            if i % freq == freq - 1 {
                                let adaptation_result = manager.adapt_prediction_model();
                                black_box(adaptation_result);
                            }
                        }
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_multi_scale_prediction(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_scale_prediction");
    group.measurement_time(Duration::from_secs(25));
    
    let time_scales = vec!["short", "medium", "long"]; // Different prediction horizons
    let prediction_horizons = [1, 5, 10, 20]; // Steps ahead to predict
    
    for scale in time_scales.iter() {
        for &horizon in prediction_horizons.iter() {
            let mut multi_scale_predictor = MultiScalePredictor::new(*scale, 256, horizon);
            
            // Generate multi-scale context data
            let scale_factor = match *scale {
                "short" => 1,
                "medium" => 5,
                "long" => 10,
            };
            let context_data = generate_multi_scale_contexts(100 * scale_factor, 256);
            
            group.throughput(Throughput::Elements(context_data.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("multi_scale", format!("{}_h{}", scale, horizon)),
                &(&multi_scale_predictor, &context_data),
                |b, (predictor, data)| {
                    b.iter(|| {
                        let mut predictions = Vec::new();
                        for context in data.iter() {
                            let prediction = predictor.predict_multi_step(context);
                            predictions.push(black_box(prediction));
                        }
                        black_box(predictions)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_context_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_compression");
    
    let compression_ratios = [0.25, 0.5, 0.75]; // Target compression ratios
    let context_sizes = [256, 512, 1024, 2048];
    
    for &ratio in compression_ratios.iter() {
        for &size in context_sizes.iter() {
            let compressed_size = (size as f32 * ratio) as usize;
            let mut context_compressor = ContextCompressor::new(size, compressed_size);
            
            // Generate contexts to compress
            let contexts = generate_compressible_contexts(100, size);
            
            group.throughput(Throughput::Elements(contexts.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("compression", format!("{}to{}", size, compressed_size)),
                &(&context_compressor, &contexts),
                |b, (compressor, contexts)| {
                    b.iter(|| {
                        let mut compressed = Vec::new();
                        for context in contexts.iter() {
                            let compressed_context = compressor.compress_context(context);
                            let reconstructed = compressor.decompress_context(&compressed_context);
                            compressed.push(black_box((compressed_context, reconstructed)));
                        }
                        black_box(compressed)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_attention_mechanisms(c: &mut Criterion) {
    let mut group = c.benchmark_group("attention_mechanisms");
    group.measurement_time(Duration::from_secs(30));
    
    let attention_types = ["self_attention", "cross_attention", "multi_head"];
    let sequence_lengths = [32, 64, 128];
    let attention_heads = [4, 8, 16];
    
    for attention_type in attention_types.iter() {
        for &seq_len in sequence_lengths.iter() {
            for &heads in attention_heads.iter() {
                let mut attention_layer = AttentionLayer::new(*attention_type, 256, heads);
                
                // Generate attention input sequences
                let input_sequences = generate_attention_inputs(50, seq_len, 256);
                
                group.throughput(Throughput::Elements(input_sequences.len() as u64));
                group.bench_with_input(
                    BenchmarkId::new("attention", format!("{}_{}_h{}", attention_type, seq_len, heads)),
                    &(&attention_layer, &input_sequences),
                    |b, (layer, sequences)| {
                        b.iter(|| {
                            let mut attended = Vec::new();
                            for sequence in sequences.iter() {
                                let attention_output = layer.apply_attention(sequence);
                                attended.push(black_box(attention_output));
                            }
                            black_box(attended)
                        })
                    },
                );
            }
        }
    }
    
    group.finish();
}

fn bench_context_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_cache_performance");
    
    let cache_sizes = [1000, 5000, 10000, 50000];
    let cache_strategies = ["lru", "lfu", "adaptive"];
    
    for &cache_size in cache_sizes.iter() {
        for strategy in cache_strategies.iter() {
            let mut context_cache = ContextCache::new(*strategy, cache_size, 256);
            
            // Pre-populate cache
            let cached_contexts = generate_cached_contexts(cache_size / 2, 256);
            for context in &cached_contexts {
                context_cache.insert(context.key, context.value.clone());
            }
            
            // Generate cache access patterns
            let access_patterns = generate_cache_access_patterns(1000, &cached_contexts);
            
            group.throughput(Throughput::Elements(access_patterns.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("context_cache", format!("{}_{}", cache_size, strategy)),
                &(&mut context_cache, &access_patterns),
                |b, (cache, patterns)| {
                    b.iter(|| {
                        let mut results = Vec::new();
                        for pattern in patterns.iter() {
                            match pattern {
                                CacheAccessPattern::Get(key) => {
                                    let result = cache.get(*key);
                                    results.push(black_box(result));
                                }
                                CacheAccessPattern::Put(key, value) => {
                                    cache.insert(*key, value.clone());
                                    results.push(black_box(()));
                                }
                            }
                        }
                        black_box(results)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_integrated_cpe_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrated_cpe_pipeline");
    group.measurement_time(Duration::from_secs(45));
    
    // Benchmark the complete CPE pipeline
    let cpe_config = CpeConfig {
        context_dimension: 256,
        sequence_length: 64,
        hidden_size: 128,
        prediction_horizon: 5,
        cache_size: 10000,
        learning_rate: 0.01,
        adaptation_threshold: 0.1,
        compression_ratio: 0.5,
        attention_heads: 8,
    };
    
    let mut cpe_system = CpeSystem::new(cpe_config);
    
    // Generate realistic context prediction workload
    let context_requests = generate_context_prediction_workload(1000, 256);
    
    group.throughput(Throughput::Elements(context_requests.len() as u64));
    group.bench_function("full_pipeline", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for request in &context_requests {
                let flow_key = black_box(request.flow_key);
                let historical_context = black_box(&request.historical_context);
                
                // Complete CPE processing pipeline
                let prediction_result = cpe_system.predict_context(flow_key, historical_context);
                results.push(black_box(prediction_result));
            }
            black_box(results)
        })
    });
    
    group.finish();
}

// Helper functions for benchmark data generation
fn generate_context_sequences(count: usize, seq_len: usize, context_dim: usize) -> Vec<Vec<Vec<f32>>> {
    (0..count)
        .map(|i| {
            (0..seq_len)
                .map(|j| {
                    (0..context_dim)
                        .map(|k| ((i + j + k) as f32 * 0.01).sin())
                        .collect()
                })
                .collect()
        })
        .collect()
}

fn generate_training_patterns(count: usize, dimension: usize) -> Vec<TrainingPattern> {
    (0..count)
        .map(|i| {
            TrainingPattern {
                input: (0..dimension)
                    .map(|j| ((i + j) as f32 * 0.02).cos())
                    .collect(),
                target: (0..dimension)
                    .map(|j| ((i + j) as f32 * 0.02 + 0.1).cos())
                    .collect(),
                weight: 1.0 + (i as f32 * 0.001),
            }
        })
        .collect()
}

fn generate_stored_contexts(count: usize, dimension: usize) -> Vec<StoredContext> {
    (0..count)
        .map(|i| {
            StoredContext {
                id: i as u64,
                vector: (0..dimension)
                    .map(|j| ((i * 7 + j) as f32 * 0.03).sin())
                    .collect(),
                metadata: ContextMetadata {
                    timestamp: i as u64,
                    flow_count: fastrand::u32(1..1000),
                    similarity_threshold: fastrand::f32() * 0.5 + 0.5,
                },
            }
        })
        .collect()
}

fn generate_query_contexts(count: usize, dimension: usize, stored: &[StoredContext]) -> Vec<QueryContext> {
    (0..count)
        .map(|i| {
            if i < stored.len() && i % 3 == 0 {
                // Similar to existing context
                let base = &stored[i].vector;
                let noise: Vec<f32> = (0..dimension)
                    .map(|_| fastrand::f32() * 0.1 - 0.05)
                    .collect();
                QueryContext {
                    vector: base.iter().zip(noise.iter()).map(|(a, b)| a + b).collect(),
                    max_results: 10,
                }
            } else {
                // Random context
                QueryContext {
                    vector: (0..dimension)
                        .map(|j| ((i * 13 + j) as f32 * 0.04).cos())
                        .collect(),
                    max_results: 10,
                }
            }
        })
        .collect()
}

fn generate_context_stream(count: usize, dimension: usize) -> Vec<StreamingContext> {
    (0..count)
        .map(|i| {
            StreamingContext {
                timestamp: i as u64,
                flow_id: (i / 10) as u64, // Multiple contexts per flow
                context_vector: (0..dimension)
                    .map(|j| ((i + j) as f32 * 0.05).sin() + ((i as f32) * 0.001).cos())
                    .collect(),
                confidence: fastrand::f32() * 0.4 + 0.6,
            }
        })
        .collect()
}

fn generate_multi_scale_contexts(count: usize, dimension: usize) -> Vec<MultiScaleContext> {
    (0..count)
        .map(|i| {
            MultiScaleContext {
                short_term: (0..dimension)
                    .map(|j| ((i + j) as f32 * 0.1).sin())
                    .collect(),
                medium_term: (0..dimension)
                    .map(|j| ((i + j) as f32 * 0.05).sin())
                    .collect(),
                long_term: (0..dimension)
                    .map(|j| ((i + j) as f32 * 0.01).sin())
                    .collect(),
                scale_weights: vec![0.5, 0.3, 0.2],
            }
        })
        .collect()
}

fn generate_compressible_contexts(count: usize, dimension: usize) -> Vec<CompressibleContext> {
    (0..count)
        .map(|i| {
            // Generate contexts with patterns that compress well
            let base_pattern: Vec<f32> = (0..dimension / 4)
                .map(|j| ((i + j) as f32 * 0.02).sin())
                .collect();
            
            // Repeat pattern with noise
            let mut full_vector = Vec::new();
            for _ in 0..4 {
                for &val in &base_pattern {
                    full_vector.push(val + fastrand::f32() * 0.1 - 0.05);
                }
            }
            
            CompressibleContext {
                original: full_vector,
                compression_priority: fastrand::f32(),
            }
        })
        .collect()
}

fn generate_attention_inputs(count: usize, seq_len: usize, dimension: usize) -> Vec<AttentionInput> {
    (0..count)
        .map(|i| {
            AttentionInput {
                sequence: (0..seq_len)
                    .map(|j| {
                        (0..dimension)
                            .map(|k| ((i + j + k) as f32 * 0.03).sin())
                            .collect()
                    })
                    .collect(),
                mask: if i % 4 == 0 {
                    Some(vec![true; seq_len])
                } else {
                    None
                },
            }
        })
        .collect()
}

fn generate_cached_contexts(count: usize, dimension: usize) -> Vec<CachedContext> {
    (0..count)
        .map(|i| {
            CachedContext {
                key: i as u64,
                value: (0..dimension)
                    .map(|j| ((i + j) as f32 * 0.02).cos())
                    .collect(),
                access_count: fastrand::u32(1..100),
                last_access: i as u64,
            }
        })
        .collect()
}

fn generate_cache_access_patterns(count: usize, cached: &[CachedContext]) -> Vec<CacheAccessPattern> {
    (0..count)
        .map(|i| {
            if i % 3 == 0 {
                // Cache miss - new entry
                CacheAccessPattern::Put(
                    (i + 10000) as u64,
                    (0..256).map(|j| ((i + j) as f32 * 0.03).sin()).collect(),
                )
            } else if i < cached.len() {
                // Cache hit
                CacheAccessPattern::Get(cached[i % cached.len()].key)
            } else {
                // Cache miss - lookup
                CacheAccessPattern::Get((i + 20000) as u64)
            }
        })
        .collect()
}

fn generate_context_prediction_workload(count: usize, dimension: usize) -> Vec<ContextPredictionRequest> {
    (0..count)
        .map(|i| {
            let mut flow_key = [0u8; 32];
            flow_key[..4].copy_from_slice(&(i as u32).to_le_bytes());
            
            let sequence_length = 32 + (i % 32);
            let historical_context: Vec<Vec<f32>> = (0..sequence_length)
                .map(|j| {
                    (0..dimension)
                        .map(|k| ((i + j + k) as f32 * 0.02).sin())
                        .collect()
                })
                .collect();
            
            ContextPredictionRequest {
                flow_key,
                historical_context,
                prediction_horizon: 1 + (i % 10),
                confidence_threshold: 0.7 + (fastrand::f32() * 0.3),
            }
        })
        .collect()
}

criterion_group!(
    cpe_benchmarks,
    bench_lstm_context_prediction,
    bench_pattern_learning,
    bench_context_similarity_matching,
    bench_adaptive_context_management,
    bench_multi_scale_prediction,
    bench_context_compression,
    bench_attention_mechanisms,
    bench_context_cache_performance,
    bench_integrated_cpe_pipeline
);

criterion_main!(cpe_benchmarks);