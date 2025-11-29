//! MFN Layer 4: Context Prediction Engine (CPE)
//! 
//! The final layer of the Multi-layer Flow Network that provides machine learning-powered
//! context prediction and adaptive routing decisions. This layer uses LSTM/Transformer
//! models for temporal pattern recognition, context embedding generation, and predictive
//! route optimization.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                         CPE Prediction Engine                              │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │    LSTM Pipeline    │  Transformer Attn  │  Context Embedder │  Predictor   │
//! │  - Sequence Model   │  - Self-Attention   │  - 256D Vectors   │  - <2ms Lat. │
//! │  - Temporal Patterns│  - Multi-Head       │  - Similarity     │  - 95% Acc.  │
//! │  - Online Learning  │  - Positional Enc.  │  - Caching        │  - Real-time │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      Layer Integration Interface                            │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │  Layer 3 ALM Input  │    Layer 2 DSR     │    Layer 1 IFR    │ HyperMesh   │
//! │  - Routing Context  │    - Neural Feed    │    - Flow Data    │ - Transport │
//! │  - Load Balancing   │    - Learning Data  │    - Pattern Info │ - Metrics   │
//! │  - Circuit Breaker  │    - Similarity     │    - Bloom Filter │ - Events    │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Performance Targets
//!
//! - **Prediction Latency**: <2ms per prediction
//! - **Prediction Accuracy**: >95% on route predictions
//! - **Context Embedding**: 256-dimensional vectors
//! - **Memory Usage**: <200MB total working set
//! - **Learning Rate**: Online adaptation within 10ms
//! - **Cache Hit Rate**: >90% for repeated patterns
//! - **Throughput**: >50,000 predictions/second
//!
//! ## Key Features
//!
//! - **Multi-Scale Temporal Analysis**: LSTM for sequence modeling
//! - **Attention Mechanisms**: Transformer attention for pattern focus
//! - **Context Similarity**: Fast embedding-based similarity search
//! - **Predictive Caching**: Smart caching of frequent prediction patterns
//! - **Online Learning**: Real-time model adaptation
//! - **Layer Integration**: Seamless data flow from Layers 1-3

pub mod models;
pub mod attention;
pub mod embeddings;
pub mod prediction;
pub mod cache;
pub mod learning;
pub mod integration;
pub mod metrics;

#[cfg(feature = "layer-integration")]
pub mod layer_integration;

#[cfg(feature = "stoq-integration")]
pub mod stoq_integration;

pub use models::{LstmModel, TransformerModel, ModelType};
pub use attention::{AttentionLayer, MultiHeadAttention, AttentionConfig};
pub use embeddings::{ContextEmbedder, EmbeddingConfig, SimilaritySearch};
pub use prediction::{ContextPredictor, PredictionResult, PredictionConfig};
pub use cache::{PredictionCache, CacheStrategy, CacheMetrics};
pub use learning::{OnlineLearner, LearningConfig, AdaptationStrategy};
pub use integration::{LayerConnector, IntegrationConfig, Layer2Message, Layer3Message, RoutingSuggestion};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// Main configuration for the CPE system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpeConfig {
    /// Model configuration
    pub model_type: ModelType,
    pub context_dimension: usize,
    pub sequence_length: usize,
    pub hidden_size: usize,
    pub num_layers: usize,
    
    /// Attention configuration (for transformer models)
    pub attention_heads: usize,
    pub attention_dropout: f64,
    
    /// Prediction configuration
    pub prediction_horizon: usize,
    pub confidence_threshold: f64,
    pub max_prediction_age_ms: u64,
    
    /// Cache configuration
    pub cache_size: usize,
    pub cache_strategy: CacheStrategy,
    pub cache_ttl_ms: u64,
    
    /// Learning configuration
    pub learning_rate: f64,
    pub adaptation_threshold: f64,
    pub batch_size: usize,
    pub online_learning_enabled: bool,
    
    /// Performance configuration
    pub enable_gpu: bool,
    pub max_concurrent_predictions: usize,
    pub prediction_timeout_ms: u64,
    
    /// Integration configuration
    pub enable_layer_integration: bool,
    pub layer2_feedback_enabled: bool,
    pub layer3_routing_enabled: bool,
    pub hypermesh_metrics_enabled: bool,
}

impl Default for CpeConfig {
    fn default() -> Self {
        Self {
            model_type: ModelType::Lstm,
            context_dimension: 256,
            sequence_length: 32,
            hidden_size: 128,
            num_layers: 2,
            attention_heads: 8,
            attention_dropout: 0.1,
            prediction_horizon: 5,
            confidence_threshold: 0.7,
            max_prediction_age_ms: 1000,
            cache_size: 10000,
            cache_strategy: CacheStrategy::Lru,
            cache_ttl_ms: 5000,
            learning_rate: 0.001,
            adaptation_threshold: 0.1,
            batch_size: 32,
            online_learning_enabled: true,
            enable_gpu: false,
            max_concurrent_predictions: 1000,
            prediction_timeout_ms: 2,
            enable_layer_integration: true,
            layer2_feedback_enabled: true,
            layer3_routing_enabled: true,
            hypermesh_metrics_enabled: true,
        }
    }
}

/// Flow key for identifying unique network flows
pub type FlowKey = [u8; 32];

/// Context vector representing system state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextVector {
    pub timestamp: u64,
    pub flow_key: FlowKey,
    pub features: Vec<f32>,
    pub metadata: HashMap<String, f32>,
    pub pattern_id: Option<String>,
}

impl ContextVector {
    pub fn new(flow_key: FlowKey, features: Vec<f32>) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            flow_key,
            features,
            metadata: HashMap::new(),
            pattern_id: None,
        }
    }
    
    pub fn with_metadata(mut self, key: String, value: f32) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    pub fn with_pattern_id(mut self, pattern_id: String) -> Self {
        self.pattern_id = Some(pattern_id);
        self
    }
}

/// Main CPE system that coordinates all prediction components
pub struct CpeSystem {
    config: CpeConfig,
    predictor: Arc<RwLock<ContextPredictor>>,
    embedder: Arc<RwLock<ContextEmbedder>>,
    cache: Arc<RwLock<PredictionCache>>,
    learner: Arc<RwLock<OnlineLearner>>,
    
    #[cfg(feature = "layer-integration")]
    layer_connector: Arc<RwLock<LayerConnector>>,
    
    #[cfg(feature = "metrics")]
    metrics: Arc<RwLock<metrics::CpeMetrics>>,
    
    /// Performance tracking
    start_time: Instant,
    prediction_count: Arc<std::sync::atomic::AtomicU64>,
}

impl CpeSystem {
    /// Create a new CPE system with the specified configuration
    pub async fn new(config: CpeConfig) -> Result<Self> {
        info!("Initializing CPE System with config: {:?}", config);
        
        // Initialize predictor based on model type
        let prediction_config = PredictionConfig {
            model_type: config.model_type,
            context_dimension: config.context_dimension,
            sequence_length: config.sequence_length,
            hidden_size: config.hidden_size,
            num_layers: config.num_layers,
            prediction_horizon: config.prediction_horizon,
            confidence_threshold: config.confidence_threshold,
            enable_gpu: config.enable_gpu,
            timeout_ms: config.prediction_timeout_ms,
        };
        
        let predictor = Arc::new(RwLock::new(
            ContextPredictor::new(prediction_config).await?
        ));
        
        // Initialize embedder
        let embedding_config = EmbeddingConfig {
            dimension: config.context_dimension,
            similarity_threshold: 0.8,
            max_neighbors: 10,
        };
        
        let embedder = Arc::new(RwLock::new(
            ContextEmbedder::new(embedding_config).await?
        ));
        
        // Initialize cache
        let cache = Arc::new(RwLock::new(
            PredictionCache::new(
                config.cache_strategy,
                config.cache_size,
                Duration::from_millis(config.cache_ttl_ms),
            )?
        ));
        
        // Initialize online learner
        let learning_config = LearningConfig {
            learning_rate: config.learning_rate,
            adaptation_threshold: config.adaptation_threshold,
            batch_size: config.batch_size,
            strategy: AdaptationStrategy::GradualDescent,
            enabled: config.online_learning_enabled,
        };
        
        let learner = Arc::new(RwLock::new(
            OnlineLearner::new(learning_config, predictor.clone()).await?
        ));
        
        // Initialize layer integration
        #[cfg(feature = "layer-integration")]
        let layer_connector = {
            let integration_config = IntegrationConfig {
                enable_layer2_feedback: config.layer2_feedback_enabled,
                enable_layer3_routing: config.layer3_routing_enabled,
                enable_hypermesh_metrics: config.hypermesh_metrics_enabled,
            };
            
            Arc::new(RwLock::new(
                LayerConnector::new(integration_config).await?
            ))
        };
        
        // Initialize metrics
        #[cfg(feature = "metrics")]
        let metrics = Arc::new(RwLock::new(
            metrics::CpeMetrics::new()?
        ));
        
        let system = Self {
            config,
            predictor,
            embedder,
            cache,
            learner,
            
            #[cfg(feature = "layer-integration")]
            layer_connector,
            
            #[cfg(feature = "metrics")]
            metrics,
            
            start_time: Instant::now(),
            prediction_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        };
        
        info!("CPE System initialized successfully");
        Ok(system)
    }
    
    /// Predict next context for a flow based on historical data
    pub async fn predict_context(
        &self,
        flow_key: FlowKey,
        historical_context: &[ContextVector],
    ) -> Result<PredictionResult> {
        let start = Instant::now();
        
        // Check cache first
        let cache_key = self.compute_cache_key(flow_key, historical_context);
        {
            let cache = self.cache.read().await;
            if let Some(cached_result) = cache.get(&cache_key).await {
                debug!("Cache hit for flow prediction");
                
                #[cfg(feature = "metrics")]
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.record_cache_hit();
                    metrics.record_prediction_latency(start.elapsed());
                }
                
                return Ok(cached_result);
            }
        }
        
        // Generate embeddings for context similarity
        let embeddings = {
            let mut embedder = self.embedder.write().await;
            embedder.embed_contexts(historical_context).await?
        };
        
        // Run prediction
        let prediction = {
            let mut predictor = self.predictor.write().await;
            predictor.predict(historical_context, Some(&embeddings)).await?
        };
        
        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, prediction.clone()).await;
        }
        
        // Update metrics and counters
        self.prediction_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        #[cfg(feature = "metrics")]
        {
            let mut metrics = self.metrics.write().await;
            metrics.record_cache_miss();
            metrics.record_prediction_latency(start.elapsed());
            metrics.record_prediction_confidence(prediction.confidence);
        }
        
        debug!("Context prediction completed in {:?}", start.elapsed());
        Ok(prediction)
    }
    
    /// Update the model with new training data (online learning)
    pub async fn learn_from_feedback(
        &self,
        flow_key: FlowKey,
        predicted_context: &ContextVector,
        actual_context: &ContextVector,
    ) -> Result<()> {
        if !self.config.online_learning_enabled {
            return Ok(());
        }
        
        let start = Instant::now();
        
        // Calculate prediction accuracy
        let accuracy = self.calculate_accuracy(predicted_context, actual_context);
        
        // Only learn if accuracy is below threshold
        if accuracy < self.config.adaptation_threshold {
            let mut learner = self.learner.write().await;
            learner.learn_from_example(predicted_context, actual_context).await?;
            
            info!("Model adapted based on feedback, accuracy: {:.3}", accuracy);
        }
        
        #[cfg(feature = "metrics")]
        {
            let mut metrics = self.metrics.write().await;
            metrics.record_learning_time(start.elapsed());
            metrics.record_prediction_accuracy(accuracy);
        }
        
        Ok(())
    }
    
    /// Get routing recommendations based on predicted context
    pub async fn get_routing_recommendations(
        &self,
        flow_key: FlowKey,
        current_context: &ContextVector,
    ) -> Result<Vec<RoutingRecommendation>> {
        let prediction = self.predict_context(
            flow_key,
            &[current_context.clone()],
        ).await?;
        
        let embedder = self.embedder.read().await;
        let similar_contexts = embedder.find_similar_contexts(
            &prediction.predicted_context,
            5,
        ).await?;
        
        let mut recommendations = Vec::new();
        
        for similar in similar_contexts {
            if let Some(pattern_id) = &similar.pattern_id {
                let recommendation = RoutingRecommendation {
                    route_type: RouteType::from_pattern(pattern_id),
                    confidence: similar.metadata.get("confidence").copied().unwrap_or(0.5),
                    predicted_latency: similar.metadata.get("latency").copied(),
                    predicted_throughput: similar.metadata.get("throughput").copied(),
                    load_balancing_weight: Some(prediction.confidence),
                };
                recommendations.push(recommendation);
            }
        }
        
        Ok(recommendations)
    }
    
    /// Start the CPE background processing loop
    pub async fn start_background_processing(&self) -> Result<()> {
        let predictor = self.predictor.clone();
        let learner = self.learner.clone();
        let cache = self.cache.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            
            loop {
                interval.tick().await;
                
                // Periodic cache cleanup
                {
                    let mut cache = cache.write().await;
                    cache.cleanup_expired().await;
                }
                
                // Periodic model optimization
                if let Ok(mut learner) = learner.try_write() {
                    if let Err(e) = learner.periodic_optimization().await {
                        warn!("Background optimization failed: {}", e);
                    }
                }
            }
        });
        
        info!("CPE background processing started");
        Ok(())
    }
    
    /// Get system performance statistics
    pub async fn get_performance_stats(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        
        stats.insert("uptime_seconds".to_string(), 
            self.start_time.elapsed().as_secs_f64());
        stats.insert("total_predictions".to_string(), 
            self.prediction_count.load(std::sync::atomic::Ordering::Relaxed) as f64);
        
        // Cache statistics
        {
            let cache = self.cache.read().await;
            let cache_stats = cache.get_metrics();
            stats.insert("cache_hit_rate".to_string(), cache_stats.hit_rate);
            stats.insert("cache_size".to_string(), cache_stats.current_size as f64);
        }
        
        // Predictor statistics
        {
            let predictor = self.predictor.read().await;
            let predictor_stats = predictor.get_statistics().await;
            stats.extend(predictor_stats);
        }
        
        #[cfg(feature = "metrics")]
        {
            let metrics = self.metrics.read().await;
            let detailed_metrics = metrics.get_all_metrics();
            stats.extend(detailed_metrics);
        }
        
        stats
    }
    
    fn compute_cache_key(&self, flow_key: FlowKey, contexts: &[ContextVector]) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(flow_key);
        
        for context in contexts.iter().take(5) {
            hasher.update(context.timestamp.to_le_bytes());
            for &feature in context.features.iter().take(10) {
                hasher.update(feature.to_le_bytes());
            }
        }
        
        format!("{:x}", hasher.finalize())
    }
    
    fn calculate_accuracy(&self, predicted: &ContextVector, actual: &ContextVector) -> f32 {
        if predicted.features.len() != actual.features.len() {
            return 0.0;
        }
        
        let mse: f32 = predicted.features.iter()
            .zip(&actual.features)
            .map(|(p, a)| (p - a).powi(2))
            .sum::<f32>() / predicted.features.len() as f32;
        
        // Convert MSE to accuracy score (0-1)
        (1.0 / (1.0 + mse)).max(0.0).min(1.0)
    }
}

/// Routing recommendation from the prediction engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRecommendation {
    pub route_type: RouteType,
    pub confidence: f32,
    pub predicted_latency: Option<f32>,
    pub predicted_throughput: Option<f32>,
    pub load_balancing_weight: Option<f32>,
}

/// Types of routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouteType {
    FastPath,
    BalancedPath,
    HighThroughput,
    LowLatency,
    FaultTolerant,
    Custom(String),
}

impl RouteType {
    fn from_pattern(pattern_id: &str) -> Self {
        match pattern_id {
            id if id.contains("fast") => RouteType::FastPath,
            id if id.contains("balanced") => RouteType::BalancedPath,
            id if id.contains("throughput") => RouteType::HighThroughput,
            id if id.contains("latency") => RouteType::LowLatency,
            id if id.contains("fault") => RouteType::FaultTolerant,
            _ => RouteType::Custom(pattern_id.to_string()),
        }
    }
}

/// CPE system builder for customized configuration
pub struct CpeBuilder {
    config: CpeConfig,
}

impl Default for CpeBuilder {
    fn default() -> Self {
        Self {
            config: CpeConfig::default(),
        }
    }
}

impl CpeBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_model_type(mut self, model_type: ModelType) -> Self {
        self.config.model_type = model_type;
        self
    }
    
    pub fn with_context_dimension(mut self, dimension: usize) -> Self {
        self.config.context_dimension = dimension;
        self
    }
    
    pub fn with_sequence_length(mut self, length: usize) -> Self {
        self.config.sequence_length = length;
        self
    }
    
    pub fn with_hidden_size(mut self, size: usize) -> Self {
        self.config.hidden_size = size;
        self
    }
    
    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.config.cache_size = size;
        self
    }
    
    pub fn with_learning_rate(mut self, rate: f64) -> Self {
        self.config.learning_rate = rate;
        self
    }
    
    pub fn enable_gpu(mut self) -> Self {
        self.config.enable_gpu = true;
        self
    }
    
    pub fn with_prediction_timeout(mut self, timeout_ms: u64) -> Self {
        self.config.prediction_timeout_ms = timeout_ms;
        self
    }
    
    pub async fn build(self) -> Result<CpeSystem> {
        CpeSystem::new(self.config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_cpe_system_creation() {
        let config = CpeConfig::default();
        let system = CpeSystem::new(config).await;
        assert!(system.is_ok());
    }
    
    #[tokio::test]
    async fn test_context_prediction() {
        let system = CpeBuilder::new()
            .with_context_dimension(64)
            .with_sequence_length(8)
            .build().await.unwrap();
        
        let flow_key = [1u8; 32];
        let contexts = vec![
            ContextVector::new(flow_key, vec![0.1; 64]),
            ContextVector::new(flow_key, vec![0.2; 64]),
        ];
        
        let result = system.predict_context(flow_key, &contexts).await;
        assert!(result.is_ok());
        
        let prediction = result.unwrap();
        assert_eq!(prediction.predicted_context.len(), 64);
        assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
    }
    
    #[tokio::test]
    async fn test_learning_feedback() {
        let system = CpeBuilder::new()
            .with_context_dimension(32)
            .build().await.unwrap();
        
        let flow_key = [2u8; 32];
        let predicted = ContextVector::new(flow_key, vec![0.5; 32]);
        let actual = ContextVector::new(flow_key, vec![0.6; 32]);
        
        let result = system.learn_from_feedback(flow_key, &predicted, &actual).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_routing_recommendations() {
        let system = CpeBuilder::new()
            .with_context_dimension(16)
            .build().await.unwrap();
        
        let flow_key = [3u8; 32];
        let context = ContextVector::new(flow_key, vec![0.3; 16])
            .with_pattern_id("fast_path".to_string());
        
        let recommendations = system.get_routing_recommendations(flow_key, &context).await;
        assert!(recommendations.is_ok());
    }
    
    #[tokio::test]
    async fn test_performance_stats() {
        let system = CpeBuilder::new().build().await.unwrap();
        let stats = system.get_performance_stats().await;
        
        assert!(stats.contains_key("uptime_seconds"));
        assert!(stats.contains_key("total_predictions"));
        assert!(stats.contains_key("cache_hit_rate"));
    }
}