/*!
# Layer 2 (DSR) Criterion Benchmarks

Neural similarity detection benchmarks for the Data Similarity Registry layer.
Tests neural network performance, pattern recognition, and adaptive learning.
*/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mfn_benchmarks::{common::*, layer2::*};
use std::time::Duration;

fn bench_neural_similarity_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("neural_similarity_detection");
    group.measurement_time(Duration::from_secs(20));
    
    // Test different input sizes and similarity thresholds
    let input_sizes = [256, 512, 1024, 2048];
    let thresholds = [0.7, 0.8, 0.9];
    
    for &input_size in input_sizes.iter() {
        for &threshold in thresholds.iter() {
            let mut similarity_detector = NeuralSimilarityDetector::new(input_size, threshold);
            
            // Generate test data
            let test_flows: Vec<_> = (0..100)
                .map(|i| generate_test_flow_vector(input_size, i))
                .collect();
            
            let reference_flow = generate_test_flow_vector(input_size, 42);
            
            group.throughput(Throughput::Elements(test_flows.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("similarity_batch", format!("{}d_{}thresh", input_size, threshold)),
                &(&similarity_detector, &test_flows, &reference_flow),
                |b, (detector, flows, reference)| {
                    b.iter(|| {
                        let mut similarities = Vec::new();
                        for flow in flows.iter() {
                            let similarity = detector.compute_similarity(reference, flow);
                            similarities.push(black_box(similarity));
                        }
                        black_box(similarities)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_lstm_training_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("lstm_training_performance");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(10);
    
    let sequence_lengths = [32, 64, 128];
    let batch_sizes = [16, 32, 64];
    
    for &seq_len in sequence_lengths.iter() {
        for &batch_size in batch_sizes.iter() {
            let mut lstm_model = LstmSimilarityModel::new(256, 128, seq_len);
            
            // Generate training data
            let training_data = generate_training_sequences(batch_size, seq_len, 256);
            let labels = generate_similarity_labels(batch_size);
            
            group.throughput(Throughput::Elements(batch_size as u64));
            group.bench_with_input(
                BenchmarkId::new("lstm_training", format("seq{}_batch{}", seq_len, batch_size)),
                &(&mut lstm_model, &training_data, &labels),
                |b, (model, data, labels)| {
                    b.iter(|| {
                        let loss = model.train_batch(data, labels);
                        black_box(loss)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_pattern_recognition_system(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_recognition_system");
    
    // Test different pattern complexities
    let pattern_counts = [100, 500, 1000, 5000];
    
    for &pattern_count in pattern_counts.iter() {
        let mut pattern_recognizer = PatternRecognitionSystem::new(256);
        
        // Pre-populate with patterns
        let patterns = generate_test_patterns(pattern_count, 256);
        for pattern in &patterns {
            pattern_recognizer.add_pattern(pattern.clone());
        }
        
        let test_inputs = generate_test_flow_vectors(100, 256);
        
        group.throughput(Throughput::Elements(test_inputs.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("pattern_matching", format!("{}patterns", pattern_count)),
            &(&pattern_recognizer, &test_inputs),
            |b, (recognizer, inputs)| {
                b.iter(|| {
                    let mut matches = Vec::new();
                    for input in inputs.iter() {
                        let matched_patterns = recognizer.recognize_patterns(input);
                        matches.push(black_box(matched_patterns));
                    }
                    black_box(matches)
                })
            },
        );
    }
    
    group.finish();
}

fn bench_adaptive_learning_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("adaptive_learning_performance");
    group.measurement_time(Duration::from_secs(25));
    
    let learning_rates = [0.001, 0.01, 0.1];
    let adaptation_frequencies = [10, 50, 100]; // Updates per adaptation
    
    for &learning_rate in learning_rates.iter() {
        for &freq in adaptation_frequencies.iter() {
            let mut adaptive_system = AdaptiveLearningSystem::new(256, learning_rate);
            
            // Generate streaming data
            let stream_data = generate_streaming_similarity_data(freq * 5);
            
            group.throughput(Throughput::Elements(freq as u64));
            group.bench_with_input(
                BenchmarkId::new("adaptive_learning", format!("lr{}_freq{}", learning_rate, freq)),
                &(&mut adaptive_system, &stream_data, freq),
                |b, (system, data, frequency)| {
                    b.iter(|| {
                        for (i, sample) in data.iter().take(*frequency).enumerate() {
                            system.process_sample(sample);
                            if i % 10 == 9 {
                                let adaptation_result = system.adapt_model();
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

fn bench_similarity_clustering(c: &mut Criterion) {
    let mut group = c.benchmark_group("similarity_clustering");
    group.measurement_time(Duration::from_secs(15));
    
    let cluster_counts = [5, 10, 20, 50];
    let data_sizes = [100, 500, 1000];
    
    for &cluster_count in cluster_counts.iter() {
        for &data_size in data_sizes.iter() {
            let mut clustering_system = SimilarityClusteringSystem::new(cluster_count, 256);
            
            let flow_data = generate_clusterable_flow_data(data_size, 256, cluster_count);
            
            group.throughput(Throughput::Elements(data_size as u64));
            group.bench_with_input(
                BenchmarkId::new("kmeans_clustering", format!("{}clusters_{}points", cluster_count, data_size)),
                &(&mut clustering_system, &flow_data),
                |b, (system, data)| {
                    b.iter(|| {
                        let cluster_assignments = system.cluster_flows(data);
                        black_box(cluster_assignments)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_feature_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("feature_extraction");
    
    let feature_dimensions = [64, 128, 256, 512];
    let raw_data_sizes = [1024, 2048, 4096];
    
    for &feature_dim in feature_dimensions.iter() {
        for &raw_size in raw_data_sizes.iter() {
            let mut feature_extractor = FlowFeatureExtractor::new(raw_size, feature_dim);
            
            let raw_flows = generate_raw_flow_data(100, raw_size);
            
            group.throughput(Throughput::Elements(raw_flows.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("feature_extraction", format!("{}to{}dim", raw_size, feature_dim)),
                &(&feature_extractor, &raw_flows),
                |b, (extractor, flows)| {
                    b.iter(|| {
                        let mut features = Vec::new();
                        for flow in flows.iter() {
                            let extracted = extractor.extract_features(flow);
                            features.push(black_box(extracted));
                        }
                        black_box(features)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_similarity_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("similarity_cache_performance");
    
    let cache_sizes = [1000, 5000, 10000, 50000];
    
    for &cache_size in cache_sizes.iter() {
        let mut similarity_cache = SimilarityCache::new(cache_size);
        
        // Pre-populate cache
        let flow_pairs = generate_flow_pairs(cache_size / 2);
        for (flow1, flow2, similarity) in &flow_pairs {
            similarity_cache.insert(*flow1, *flow2, *similarity);
        }
        
        // Generate test queries (mix of hits and misses)
        let test_queries = generate_cache_queries(1000, &flow_pairs);
        
        group.throughput(Throughput::Elements(test_queries.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("cache_lookup", format!("{}entries", cache_size)),
            &(&similarity_cache, &test_queries),
            |b, (cache, queries)| {
                b.iter(|| {
                    let mut results = Vec::new();
                    for (flow1, flow2) in queries.iter() {
                        let result = cache.get(*flow1, *flow2);
                        results.push(black_box(result));
                    }
                    black_box(results)
                })
            },
        );
    }
    
    group.finish();
}

fn bench_integrated_dsr_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrated_dsr_pipeline");
    group.measurement_time(Duration::from_secs(30));
    
    // Benchmark the complete DSR pipeline
    let mut dsr_system = DsrSystem::new(DsrConfig {
        input_dimension: 256,
        similarity_threshold: 0.8,
        cache_size: 10000,
        cluster_count: 20,
        learning_rate: 0.01,
        adaptation_frequency: 50,
    });
    
    // Generate realistic flow data
    let flow_stream = generate_realistic_flow_stream(1000, 256);
    
    group.throughput(Throughput::Elements(flow_stream.len() as u64));
    group.bench_function("full_pipeline", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for flow in &flow_stream {
                let flow_key = black_box(flow.key);
                
                // Complete DSR processing pipeline
                let similarity_results = dsr_system.process_flow_similarity(flow_key, &flow.data);
                results.push(black_box(similarity_results));
            }
            black_box(results)
        })
    });
    
    group.finish();
}

// Helper functions for benchmark data generation
fn generate_test_flow_vector(size: usize, seed: usize) -> Vec<f32> {
    (0..size)
        .map(|i| ((seed + i) as f32 * 0.1).sin())
        .collect()
}

fn generate_training_sequences(batch_size: usize, seq_len: usize, feature_dim: usize) -> Vec<Vec<Vec<f32>>> {
    (0..batch_size)
        .map(|batch| {
            (0..seq_len)
                .map(|seq| generate_test_flow_vector(feature_dim, batch * seq_len + seq))
                .collect()
        })
        .collect()
}

fn generate_similarity_labels(batch_size: usize) -> Vec<f32> {
    (0..batch_size)
        .map(|i| if i % 3 == 0 { 0.9 } else { 0.3 })
        .collect()
}

fn generate_test_patterns(count: usize, dimension: usize) -> Vec<FlowPattern> {
    (0..count)
        .map(|i| FlowPattern {
            id: i as u32,
            features: generate_test_flow_vector(dimension, i),
            weight: 1.0 + (i as f32 * 0.01),
        })
        .collect()
}

fn generate_test_flow_vectors(count: usize, dimension: usize) -> Vec<Vec<f32>> {
    (0..count)
        .map(|i| generate_test_flow_vector(dimension, i))
        .collect()
}

fn generate_streaming_similarity_data(count: usize) -> Vec<SimilarityDataPoint> {
    (0..count)
        .map(|i| SimilarityDataPoint {
            flow1: generate_test_flow_vector(256, i),
            flow2: generate_test_flow_vector(256, i + 100),
            similarity: if i % 4 == 0 { 0.85 } else { 0.25 },
            timestamp: i as u64,
        })
        .collect()
}

fn generate_clusterable_flow_data(count: usize, dimension: usize, cluster_count: usize) -> Vec<Vec<f32>> {
    (0..count)
        .map(|i| {
            let cluster = i % cluster_count;
            let base_value = cluster as f32 * 2.0;
            (0..dimension)
                .map(|j| base_value + ((i + j) as f32 * 0.01).sin())
                .collect()
        })
        .collect()
}

fn generate_raw_flow_data(count: usize, raw_size: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| (0..raw_size).map(|j| ((i + j) % 256) as u8).collect())
        .collect()
}

fn generate_flow_pairs(count: usize) -> Vec<([u8; 32], [u8; 32], f32)> {
    (0..count)
        .map(|i| {
            let mut flow1 = [0u8; 32];
            let mut flow2 = [0u8; 32];
            
            flow1[..4].copy_from_slice(&(i as u32).to_le_bytes());
            flow2[..4].copy_from_slice(&((i + 1000) as u32).to_le_bytes());
            
            let similarity = if i % 3 == 0 { 0.85 } else { 0.25 };
            
            (flow1, flow2, similarity)
        })
        .collect()
}

fn generate_cache_queries(count: usize, existing_pairs: &[([u8; 32], [u8; 32], f32)]) -> Vec<([u8; 32], [u8; 32])> {
    (0..count)
        .map(|i| {
            if i < existing_pairs.len() && i % 2 == 0 {
                // Cache hit
                (existing_pairs[i].0, existing_pairs[i].1)
            } else {
                // Cache miss
                let mut flow1 = [0u8; 32];
                let mut flow2 = [0u8; 32];
                flow1[..4].copy_from_slice(&((i + 10000) as u32).to_le_bytes());
                flow2[..4].copy_from_slice(&((i + 20000) as u32).to_le_bytes());
                (flow1, flow2)
            }
        })
        .collect()
}

fn generate_realistic_flow_stream(count: usize, dimension: usize) -> Vec<FlowData> {
    (0..count)
        .map(|i| {
            let mut key = [0u8; 32];
            key[..4].copy_from_slice(&(i as u32).to_le_bytes());
            
            FlowData {
                key,
                data: generate_test_flow_vector(dimension, i),
                timestamp: i as u64,
            }
        })
        .collect()
}

criterion_group!(
    dsr_benchmarks,
    bench_neural_similarity_detection,
    bench_lstm_training_performance,
    bench_pattern_recognition_system,
    bench_adaptive_learning_performance,
    bench_similarity_clustering,
    bench_feature_extraction,
    bench_similarity_cache_performance,
    bench_integrated_dsr_pipeline
);

criterion_main!(dsr_benchmarks);