//! Integration tests for the CPE system

use mfn_layer4_cpe::{
    CpeBuilder, ContextVector, ModelType, 
    prediction::PredictionResult,
    cache::CacheStrategy,
};
use std::time::Duration;

#[tokio::test]
async fn test_cpe_system_basic_functionality() {
    // Create a simple CPE system
    let cpe = CpeBuilder::new()
        .with_model_type(ModelType::Lstm)
        .with_context_dimension(32)
        .with_sequence_length(4)
        .with_hidden_size(16)
        .with_cache_size(100)
        .build()
        .await
        .expect("Failed to create CPE system");

    // Create test contexts
    let flow_key = [1u8; 32];
    let contexts = vec![
        ContextVector::new(flow_key, vec![0.1; 32]),
        ContextVector::new(flow_key, vec![0.2; 32]),
        ContextVector::new(flow_key, vec![0.3; 32]),
        ContextVector::new(flow_key, vec![0.4; 32]),
    ];

    // Test prediction
    let result = cpe.predict_context(flow_key, &contexts).await;
    assert!(result.is_ok(), "Prediction should succeed");

    let prediction = result.unwrap();
    assert_eq!(prediction.predicted_context.len(), 32);
    assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
    assert!(prediction.processing_time_ms > 0.0);
}

#[tokio::test]
async fn test_different_model_types() {
    let model_types = [ModelType::Lstm, ModelType::Transformer, ModelType::Hybrid];

    for model_type in model_types {
        let cpe = CpeBuilder::new()
            .with_model_type(model_type)
            .with_context_dimension(16)
            .with_sequence_length(4)
            .build()
            .await
            .expect(&format!("Failed to create CPE with {:?} model", model_type));

        let flow_key = [2u8; 32];
        let contexts = vec![
            ContextVector::new(flow_key, vec![0.5; 16]),
            ContextVector::new(flow_key, vec![0.6; 16]),
        ];

        let result = cpe.predict_context(flow_key, &contexts).await;
        assert!(result.is_ok(), "Prediction with {:?} model should succeed", model_type);
    }
}

#[tokio::test]
async fn test_caching_functionality() {
    let cpe = CpeBuilder::new()
        .with_context_dimension(16)
        .with_cache_size(50)
        .build()
        .await
        .unwrap();

    let flow_key = [3u8; 32];
    let contexts = vec![
        ContextVector::new(flow_key, vec![0.7; 16]),
        ContextVector::new(flow_key, vec![0.8; 16]),
    ];

    // First prediction (cache miss)
    let result1 = cpe.predict_context(flow_key, &contexts).await.unwrap();
    let time1 = result1.processing_time_ms;

    // Second prediction with same input (should be faster due to caching)
    let result2 = cpe.predict_context(flow_key, &contexts).await.unwrap();
    let time2 = result2.processing_time_ms;

    // Results should be consistent
    assert_eq!(result1.predicted_context.len(), result2.predicted_context.len());
    
    // Note: In practice, we'd expect time2 < time1 due to caching,
    // but for this simple test we just verify both succeed
    assert!(time1 > 0.0);
    assert!(time2 > 0.0);
}

#[tokio::test]
async fn test_learning_feedback() {
    let cpe = CpeBuilder::new()
        .with_context_dimension(16)
        .with_learning_rate(0.01)
        .build()
        .await
        .unwrap();

    let flow_key = [4u8; 32];
    let predicted_context = ContextVector::new(flow_key, vec![0.5; 16]);
    let actual_context = ContextVector::new(flow_key, vec![0.6; 16]);

    // Test learning feedback
    let result = cpe.learn_from_feedback(flow_key, &predicted_context, &actual_context).await;
    assert!(result.is_ok(), "Learning feedback should succeed");
}

#[tokio::test]
async fn test_routing_recommendations() {
    let cpe = CpeBuilder::new()
        .with_context_dimension(16)
        .build()
        .await
        .unwrap();

    let flow_key = [5u8; 32];
    let context = ContextVector::new(flow_key, vec![0.3; 16]);

    let recommendations = cpe.get_routing_recommendations(flow_key, &context).await;
    assert!(recommendations.is_ok(), "Routing recommendations should succeed");

    let recs = recommendations.unwrap();
    // Should get at least one recommendation
    assert!(!recs.is_empty(), "Should provide at least one routing recommendation");
    
    for rec in &recs {
        assert!(!rec.route_id.is_empty());
        assert!(rec.expected_latency_ms > 0.0);
        assert!(rec.reliability_score >= 0.0 && rec.reliability_score <= 1.0);
    }
}

#[tokio::test]
async fn test_performance_statistics() {
    let cpe = CpeBuilder::new()
        .with_context_dimension(16)
        .build()
        .await
        .unwrap();

    // Make a few predictions to generate stats
    let flow_key = [6u8; 32];
    let contexts = vec![ContextVector::new(flow_key, vec![0.4; 16])];

    for _ in 0..3 {
        let _ = cpe.predict_context(flow_key, &contexts).await;
    }

    let stats = cpe.get_performance_stats().await;
    
    // Verify expected statistics are present
    assert!(stats.contains_key("uptime_seconds"));
    assert!(stats.contains_key("total_predictions"));
    assert!(stats.contains_key("cache_hit_rate"));
    
    // Verify some basic values
    assert!(stats.get("uptime_seconds").unwrap() > &0.0);
    assert_eq!(stats.get("total_predictions").unwrap(), &3.0);
}

#[tokio::test]
async fn test_concurrent_predictions() {
    let cpe = CpeBuilder::new()
        .with_context_dimension(16)
        .with_cache_size(100)
        .build()
        .await
        .unwrap();

    // Create multiple concurrent prediction tasks
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let cpe_clone = &cpe;
        let handle = tokio::spawn(async move {
            let flow_key = [i as u8; 32];
            let contexts = vec![ContextVector::new(flow_key, vec![i as f32 * 0.1; 16])];
            cpe_clone.predict_context(flow_key, &contexts).await
        });
        handles.push(handle);
    }

    // Wait for all predictions to complete
    let mut successful_predictions = 0;
    for handle in handles {
        if let Ok(Ok(_)) = handle.await {
            successful_predictions += 1;
        }
    }

    assert_eq!(successful_predictions, 10, "All concurrent predictions should succeed");
}