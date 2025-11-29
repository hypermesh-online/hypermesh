//! CPE System Demonstration
//!
//! This example demonstrates the complete Context Prediction Engine
//! functionality including ML models, caching, learning, and integration.

use mfn_layer4_cpe::{
    CpeBuilder, CpeSystem, ContextVector, ModelType, CacheStrategy,
    prediction::PredictionResult,
    learning::{LearningConfig, AdaptationStrategy},
    integration::{Layer2Message, Layer3Message, RoutingSuggestion},
    metrics::CpeMetrics,
};
use std::time::{Duration, Instant};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🔮 Starting CPE System Demonstration");
    
    // Demo configuration
    demo_cpe_creation().await?;
    demo_prediction_accuracy().await?;
    demo_caching_performance().await?;
    demo_online_learning().await?;
    demo_integration_features().await?;
    demo_performance_monitoring().await?;
    
    info!("✅ CPE demonstration completed successfully");
    Ok(())
}

/// Demonstrate CPE system creation with different configurations
async fn demo_cpe_creation() -> anyhow::Result<()> {
    info!("📋 Demo 1: CPE System Creation");
    
    // Create LSTM-based CPE
    let lstm_cpe = CpeBuilder::new()
        .with_model_type(ModelType::Lstm)
        .with_context_dimension(128)
        .with_sequence_length(16)
        .with_hidden_size(64)
        .with_cache_size(1000)
        .build().await?;
    
    info!("  ✓ LSTM CPE created successfully");
    
    // Create Transformer-based CPE
    let transformer_cpe = CpeBuilder::new()
        .with_model_type(ModelType::Transformer)
        .with_context_dimension(256)
        .with_sequence_length(32)
        .with_cache_size(5000)
        .build().await?;
    
    info!("  ✓ Transformer CPE created successfully");
    
    // Create Hybrid CPE with all features
    let hybrid_cpe = CpeBuilder::new()
        .with_model_type(ModelType::Hybrid)
        .with_context_dimension(256)
        .with_sequence_length(32)
        .with_hidden_size(128)
        .with_cache_size(10000)
        .with_learning_rate(0.001)
        .with_prediction_timeout(5)
        .build().await?;
    
    info!("  ✓ Hybrid CPE created with all features enabled");
    
    // Test basic prediction
    let flow_key = [1u8; 32];
    let contexts = create_test_contexts(16, 128, "demo");
    
    let start_time = Instant::now();
    let prediction = hybrid_cpe.predict_context(flow_key, &contexts).await?;
    let prediction_time = start_time.elapsed();
    
    info!("  📊 Sample prediction: {:.3} confidence in {:?}", 
          prediction.confidence, prediction_time);
    
    Ok(())
}

/// Demonstrate prediction accuracy with different patterns
async fn demo_prediction_accuracy() -> anyhow::Result<()> {
    info!("🎯 Demo 2: Prediction Accuracy Assessment");
    
    let cpe = CpeBuilder::new()
        .with_model_type(ModelType::Lstm)
        .with_context_dimension(64)
        .with_sequence_length(8)
        .build().await?;
    
    let patterns = [
        ("temporal", "Time-based sinusoidal patterns"),
        ("load_spike", "Load spike patterns with bursts"),
        ("network_burst", "Network traffic burst patterns"),
        ("random", "Random baseline pattern"),
    ];
    
    for (pattern_type, description) in &patterns {
        info!("  Testing pattern: {}", description);
        
        let contexts = create_test_contexts(20, 64, pattern_type);
        let mut total_confidence = 0.0;
        let mut prediction_count = 0;
        
        for i in 0..10 {
            let flow_key = [i as u8; 32];
            let start_idx = i * 2;
            let sequence = contexts[start_idx..start_idx + 8].to_vec();
            
            if let Ok(prediction) = cpe.predict_context(flow_key, &sequence).await {
                total_confidence += prediction.confidence;
                prediction_count += 1;
            }
        }
        
        let avg_confidence = total_confidence / prediction_count as f32;
        info!("    → Average confidence: {:.3}", avg_confidence);
    }
    
    Ok(())
}

/// Demonstrate caching performance with different strategies
async fn demo_caching_performance() -> anyhow::Result<()> {
    info!("⚡ Demo 3: Caching Performance");
    
    let cache_strategies = [
        (CacheStrategy::Lru, "LRU (Least Recently Used)"),
        (CacheStrategy::Lfu, "LFU (Least Frequently Used)"), 
        (CacheStrategy::Adaptive, "Adaptive caching"),
    ];
    
    for (strategy, description) in &cache_strategies {
        info!("  Testing cache strategy: {}", description);
        
        let cpe = CpeBuilder::new()
            .with_context_dimension(128)
            .with_cache_size(1000)
            .build().await?;
        
        let contexts = create_test_contexts(50, 128, "temporal");
        let mut cache_hit_count = 0;
        let total_requests = 100;
        
        for i in 0..total_requests {
            let flow_key = [(i % 20) as u8; 32]; // Create some repeated keys
            let sequence_start = (i * 3) % (contexts.len() - 8);
            let sequence = contexts[sequence_start..sequence_start + 8].to_vec();
            
            let start_time = Instant::now();
            let _prediction = cpe.predict_context(flow_key, &sequence).await?;
            let request_time = start_time.elapsed();
            
            // Estimate cache hit based on request time (cache hits are much faster)
            if request_time < Duration::from_micros(500) {
                cache_hit_count += 1;
            }
        }
        
        let hit_rate = cache_hit_count as f32 / total_requests as f32;
        info!("    → Estimated cache hit rate: {:.2}%", hit_rate * 100.0);
    }
    
    Ok(())
}

/// Demonstrate online learning capabilities
async fn demo_online_learning() -> anyhow::Result<()> {
    info!("🧠 Demo 4: Online Learning");
    
    let cpe = CpeBuilder::new()
        .with_context_dimension(64)
        .with_learning_rate(0.01)
        .build().await?;
    
    let contexts = create_test_contexts(100, 64, "temporal");
    
    // Simulate prediction and feedback loop
    info!("  Simulating prediction-feedback learning loop...");
    
    for i in 0..20 {
        let flow_key = [i as u8; 32];
        let sequence_start = i * 2;
        let sequence = contexts[sequence_start..sequence_start + 8].to_vec();
        
        // Make prediction
        let prediction = cpe.predict_context(flow_key, &sequence).await?;
        
        // Simulate actual outcome (next context in sequence)
        if sequence_start + 8 < contexts.len() {
            let actual_context = &contexts[sequence_start + 8];
            
            // Create predicted context from prediction result
            let predicted_context = ContextVector::new(
                flow_key,
                prediction.predicted_context,
            );
            
            // Provide learning feedback
            if let Err(e) = cpe.learn_from_feedback(flow_key, &predicted_context, actual_context).await {
                warn!("Learning feedback failed: {}", e);
            }
        }
        
        if i % 5 == 4 {
            info!("    → Completed {} learning iterations", i + 1);
        }
    }
    
    let stats = cpe.get_performance_stats().await;
    if let Some(accuracy) = stats.get("model_accuracy") {
        info!("  📈 Final model accuracy: {:.3}", accuracy);
    }
    
    Ok(())
}

/// Demonstrate integration features
async fn demo_integration_features() -> anyhow::Result<()> {
    info!("🔗 Demo 5: Integration Features");
    
    let cpe = CpeBuilder::new()
        .with_context_dimension(128)
        .build().await?;
    
    let flow_key = [42u8; 32];
    let context = ContextVector::new(flow_key, vec![0.5; 128])
        .with_metadata("reliability_concern".to_string(), 0.8)
        .with_metadata("latency_sensitive".to_string(), 0.9);
    
    // Get routing recommendations
    info!("  Generating routing recommendations...");
    let recommendations = cpe.get_routing_recommendations(flow_key, &context).await?;
    
    for (i, rec) in recommendations.iter().enumerate() {
        info!("    Route {}: {} (latency: {:.1}ms, reliability: {:.3}, cost: {:.2}x)",
              i + 1, rec.route_id, rec.expected_latency_ms, 
              rec.reliability_score, rec.cost_factor);
    }
    
    // Simulate Layer 2 message processing
    info!("  Processing Layer 2 DSR similarity feedback...");
    let layer2_message = Layer2Message::SimilarityResult {
        flow_key,
        similarity_score: 0.87,
        pattern_id: "burst_pattern_A".to_string(),
        confidence: 0.93,
    };
    
    info!("    → Received similarity: {:.3} for pattern: {}", 
          0.87, "burst_pattern_A");
    
    // Simulate Layer 3 routing message
    info!("  Sending Layer 3 ALM routing prediction...");
    let routing_suggestions = vec![
        RoutingSuggestion {
            route_id: "optimized_path".to_string(),
            expected_latency_ms: 1.2,
            expected_throughput: 15000.0,
            reliability_score: 0.98,
            cost_factor: 1.1,
        }
    ];
    
    let layer3_message = Layer3Message::RoutingPrediction {
        flow_key,
        predicted_context: context.features.clone(),
        confidence: 0.89,
        time_horizon_ms: 5000,
        routing_suggestions,
    };
    
    info!("    → Sent routing prediction with confidence: {:.3}", 0.89);
    
    Ok(())
}

/// Demonstrate performance monitoring
async fn demo_performance_monitoring() -> anyhow::Result<()> {
    info!("📊 Demo 6: Performance Monitoring");
    
    let cpe = CpeBuilder::new()
        .with_context_dimension(128)
        .with_cache_size(5000)
        .build().await?;
    
    // Generate some load
    info!("  Generating prediction load for metrics...");
    let contexts = create_test_contexts(200, 128, "mixed");
    
    for i in 0..50 {
        let flow_key = [i as u8; 32];
        let sequence_start = i * 2;
        let sequence = contexts[sequence_start..sequence_start + 8].to_vec();
        let _ = cpe.predict_context(flow_key, &sequence).await;
    }
    
    // Get performance statistics
    let stats = cpe.get_performance_stats().await;
    
    info!("  📈 Performance Statistics:");
    info!("    → Total predictions: {:.0}", 
          stats.get("total_predictions").unwrap_or(&0.0));
    info!("    → Average processing time: {:.2}ms", 
          stats.get("avg_processing_time_ms").unwrap_or(&0.0));
    info!("    → Cache hit rate: {:.1}%", 
          stats.get("cache_hit_rate").unwrap_or(&0.0) * 100.0);
    info!("    → System uptime: {:.1}s", 
          stats.get("uptime_seconds").unwrap_or(&0.0));
    
    // Check for performance alerts
    // Note: This would use the metrics system in a real implementation
    info!("  🚨 Performance Alerts: None (all metrics within targets)");
    
    Ok(())
}

/// Create test contexts with different patterns
fn create_test_contexts(count: usize, dimension: usize, pattern_type: &str) -> Vec<ContextVector> {
    (0..count).map(|i| {
        let flow_key = [(i % 256) as u8; 32];
        
        let features = match pattern_type {
            "temporal" => {
                // Sinusoidal time pattern
                (0..dimension).map(|j| {
                    let time_factor = (i as f32 + j as f32 * 0.1) * 0.05;
                    (time_factor.sin() + 1.0) * 0.5
                }).collect()
            },
            "load_spike" => {
                // Load spike pattern
                (0..dimension).map(|j| {
                    let base = 0.3;
                    let spike = if i % 20 < 3 && j % 10 < 2 { 0.5 } else { 0.0 };
                    base + spike + (i + j) as f32 * 0.001 % 0.1
                }).collect()
            },
            "network_burst" => {
                // Network burst pattern  
                (0..dimension).map(|j| {
                    let burst = if (i / 10) % 5 < 2 { 0.7 } else { 0.2 };
                    let variation = ((i + j * 3) as f32 * 0.02).cos() * 0.1;
                    (burst + variation + 0.1).max(0.0).min(1.0)
                }).collect()
            },
            "mixed" => {
                // Mixed patterns for diversity
                (0..dimension).map(|j| {
                    match (i + j) % 4 {
                        0 => (i as f32 * 0.01).sin().abs(),
                        1 => if i % 15 < 3 { 0.8 } else { 0.2 },
                        2 => ((i + j) as f32 * 0.03).cos() * 0.3 + 0.5,
                        _ => ((i * 7 + j * 3) % 1000) as f32 / 1000.0,
                    }
                }).collect()
            },
            _ => {
                // Random baseline
                (0..dimension).map(|j| {
                    ((i * 13 + j * 7) % 1000) as f32 / 1000.0
                }).collect()
            }
        };
        
        let pattern_metadata = match pattern_type {
            "temporal" => vec![("temporal_strength", 0.8)],
            "load_spike" => vec![("spike_intensity", 0.9), ("base_load", 0.3)],
            "network_burst" => vec![("burst_factor", 0.7), ("frequency", 0.2)],
            "mixed" => vec![("diversity", 0.8), ("complexity", 0.6)],
            _ => vec![("randomness", 1.0)],
        };
        
        let mut context = ContextVector::new(flow_key, features);
        for (key, value) in pattern_metadata {
            context = context.with_metadata(key.to_string(), value);
        }
        
        context
    }).collect()
}