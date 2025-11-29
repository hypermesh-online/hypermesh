//! MFN Layer 2: Dynamic Similarity Reservoir (DSR)
//! 
//! Implements a spiking neural network for dynamic similarity detection and routing optimization.
//! This layer provides the core neural intelligence for pattern recognition, adaptive learning,
//! and network routing decisions in the HyperMesh ecosystem.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                            DSR Neural Engine                                │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │  SpikingNeuron  │  NeuralNetwork  │ CompetitiveDynamics │ AdaptationEngine  │
//! │  - LIF Model    │  - 1000 neurons │ - Lateral Inhibit.  │ - STDP Learning   │
//! │  - Threshold    │  - 50K synapses │ - Winner-Take-All   │ - Online Adapt.   │
//! │  - Refractory   │  - Topology     │ - Competition       │ - Forgetting      │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                         STOQ Integration Layer                              │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │     RoutingOptimizer    │    ServiceMeshIntelligence    │   PatternCache    │
//! │     - Path Selection    │    - Load Balancing           │   - LRU Eviction │
//! │     - 777% Improvement  │    - Circuit Breaking         │   - Fast Lookup  │
//! │     - ML Predictions    │    - Traffic Analysis         │   - Adaptive Size │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Performance Targets
//!
//! - **Neural Similarity Detection**: <1ms per analysis
//! - **Adaptation Rate**: <100ms to network changes  
//! - **Pattern Recognition**: >95% accuracy
//! - **Memory Usage**: <100MB neural network state
//! - **Learning Convergence**: <5 minutes for new patterns
//! - **Routing Improvement**: 777% performance increase

pub mod spiking;
pub mod network;
pub mod competitive;
pub mod adaptation;
pub mod routing;
pub mod service_mesh;
pub mod cache;
pub mod metrics;

#[cfg(feature = "stoq-integration")]
pub mod stoq_integration;

pub use spiking::{SpikingNeuron, NeuronState, SpikeEvent};
pub use network::{NeuralNetwork, NetworkTopology, SynapticConnection};
pub use competitive::{CompetitiveDynamics, WinnerTakeAll};
pub use adaptation::{AdaptationEngine, STDPLearning, OnlineLearning};
pub use routing::{RoutingOptimizer, RoutingDecision, PathOptimization};
pub use service_mesh::{ServiceMeshIntelligence, LoadBalancingStrategy, CircuitBreakerState};
pub use cache::{PatternCache, SimilarityResult, CacheStats};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// Configuration for the DSR system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsrConfig {
    /// Number of neurons in the network (default: 1000)
    pub neuron_count: usize,
    
    /// Average synaptic connections per neuron (default: 50)
    pub synapses_per_neuron: usize,
    
    /// Neural similarity threshold (default: 0.8)
    pub similarity_threshold: f64,
    
    /// Learning rate for STDP (default: 0.01)
    pub learning_rate: f64,
    
    /// Adaptation frequency in milliseconds (default: 100)
    pub adaptation_frequency_ms: u64,
    
    /// Maximum pattern cache size (default: 10000)
    pub max_cache_size: usize,
    
    /// Lateral inhibition strength (default: 0.5)
    pub inhibition_strength: f64,
    
    /// Winner-take-all competition radius (default: 0.1)
    pub competition_radius: f64,
    
    /// Memory forgetting rate (default: 0.001)
    pub forgetting_rate: f64,
}

impl Default for DsrConfig {
    fn default() -> Self {
        Self {
            neuron_count: 1000,
            synapses_per_neuron: 50,
            similarity_threshold: 0.8,
            learning_rate: 0.01,
            adaptation_frequency_ms: 100,
            max_cache_size: 10000,
            inhibition_strength: 0.5,
            competition_radius: 0.1,
            forgetting_rate: 0.001,
        }
    }
}

/// Main DSR system combining all neural components
pub struct DsrSystem {
    config: DsrConfig,
    neural_network: Arc<RwLock<NeuralNetwork>>,
    competitive_dynamics: Arc<RwLock<CompetitiveDynamics>>,
    adaptation_engine: Arc<RwLock<AdaptationEngine>>,
    routing_optimizer: Arc<RwLock<RoutingOptimizer>>,
    service_mesh_intelligence: Arc<RwLock<ServiceMeshIntelligence>>,
    pattern_cache: Arc<RwLock<PatternCache>>,
    
    #[cfg(feature = "stoq-integration")]
    stoq_integration: Arc<RwLock<stoq_integration::StoqIntegration>>,
    
    #[cfg(feature = "metrics")]
    metrics: Arc<RwLock<metrics::DsrMetrics>>,
    
    /// Performance tracking
    start_time: Instant,
    processing_count: Arc<std::sync::atomic::AtomicU64>,
}

impl DsrSystem {
    /// Create a new DSR system with the specified configuration
    pub async fn new(config: DsrConfig) -> Result<Self> {
        info!("Initializing DSR System with config: {:?}", config);
        
        let neural_network = Arc::new(RwLock::new(
            NeuralNetwork::new(config.neuron_count, config.synapses_per_neuron).await?
        ));
        
        let competitive_dynamics = Arc::new(RwLock::new(
            CompetitiveDynamics::new(
                config.inhibition_strength,
                config.competition_radius,
            )?
        ));
        
        let adaptation_engine = Arc::new(RwLock::new(
            AdaptationEngine::new(config.learning_rate, config.forgetting_rate)?
        ));
        
        let routing_optimizer = Arc::new(RwLock::new(
            RoutingOptimizer::new(neural_network.clone()).await?
        ));
        
        let service_mesh_intelligence = Arc::new(RwLock::new(
            ServiceMeshIntelligence::new(neural_network.clone()).await?
        ));
        
        let pattern_cache = Arc::new(RwLock::new(
            PatternCache::new(config.max_cache_size)?
        ));
        
        #[cfg(feature = "stoq-integration")]
        let stoq_integration = Arc::new(RwLock::new(
            stoq_integration::StoqIntegration::new(
                routing_optimizer.clone(),
                service_mesh_intelligence.clone(),
            ).await?
        ));
        
        #[cfg(feature = "metrics")]
        let metrics = Arc::new(RwLock::new(
            metrics::DsrMetrics::new()?
        ));
        
        let system = Self {
            config,
            neural_network,
            competitive_dynamics,
            adaptation_engine,
            routing_optimizer,
            service_mesh_intelligence,
            pattern_cache,
            
            #[cfg(feature = "stoq-integration")]
            stoq_integration,
            
            #[cfg(feature = "metrics")]
            metrics,
            
            start_time: Instant::now(),
            processing_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        };
        
        info!("DSR System initialized successfully");
        Ok(system)
    }
    
    /// Process a similarity request and return routing decision
    pub async fn process_similarity(&self, 
        input_pattern: &[f64],
        context: Option<&[f64]>
    ) -> Result<SimilarityResult> {
        let start = Instant::now();
        
        // Check cache first
        let cache_key = self.compute_cache_key(input_pattern, context);
        {
            let cache = self.pattern_cache.read().await;
            if let Some(cached_result) = cache.get(&cache_key) {
                return Ok(cached_result.clone());
            }
        }
        
        // Neural processing
        let network_output = {
            let mut network = self.neural_network.write().await;
            network.process_input(input_pattern, context).await?
        };
        
        // Competitive dynamics
        let competition_result = {
            let mut competition = self.competitive_dynamics.write().await;
            competition.apply_competition(&network_output).await?
        };
        
        // Create similarity result
        let similarity_result = SimilarityResult {
            similarity_score: competition_result.winning_similarity,
            pattern_id: competition_result.winner_id,
            confidence: competition_result.confidence,
            processing_time: start.elapsed(),
            cache_hit: false,
        };
        
        // Cache the result
        {
            let mut cache = self.pattern_cache.write().await;
            cache.insert(cache_key, similarity_result.clone());
        }
        
        // Update metrics
        self.processing_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        #[cfg(feature = "metrics")]
        {
            let mut metrics = self.metrics.write().await;
            metrics.record_processing_time(start.elapsed());
            metrics.record_similarity_score(similarity_result.similarity_score);
        }
        
        debug!("Processed similarity in {:?}", start.elapsed());
        Ok(similarity_result)
    }
    
    /// Adapt the neural network based on feedback
    pub async fn adapt_network(&self, 
        training_data: &[(Vec<f64>, Vec<f64>, f64)]
    ) -> Result<()> {
        info!("Starting network adaptation with {} samples", training_data.len());
        let start = Instant::now();
        
        {
            let mut adaptation = self.adaptation_engine.write().await;
            let mut network = self.neural_network.write().await;
            
            adaptation.adapt_network(&mut network, training_data).await?;
        }
        
        info!("Network adaptation completed in {:?}", start.elapsed());
        Ok(())
    }
    
    /// Get routing optimization for a given network state
    pub async fn optimize_routing(&self, 
        network_state: &HashMap<String, f64>
    ) -> Result<RoutingDecision> {
        let routing_optimizer = self.routing_optimizer.read().await;
        routing_optimizer.optimize_path(network_state).await
    }
    
    /// Get service mesh intelligence recommendations
    pub async fn get_service_mesh_recommendations(&self,
        service_metrics: &HashMap<String, f64>
    ) -> Result<Vec<service_mesh::Recommendation>> {
        let service_mesh = self.service_mesh_intelligence.read().await;
        service_mesh.analyze_and_recommend(service_metrics).await
    }
    
    /// Get system performance statistics
    pub async fn get_performance_stats(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        
        stats.insert("uptime_seconds".to_string(), 
            self.start_time.elapsed().as_secs_f64());
        stats.insert("total_processed".to_string(), 
            self.processing_count.load(std::sync::atomic::Ordering::Relaxed) as f64);
        
        let cache = self.pattern_cache.read().await;
        let cache_stats = cache.get_stats();
        stats.insert("cache_hit_rate".to_string(), cache_stats.hit_rate);
        stats.insert("cache_size".to_string(), cache_stats.current_size as f64);
        
        #[cfg(feature = "metrics")]
        {
            let metrics = self.metrics.read().await;
            for (key, value) in metrics.get_all_metrics() {
                stats.insert(key, value);
            }
        }
        
        stats
    }
    
    /// Start continuous adaptation loop
    pub async fn start_adaptation_loop(&self) -> Result<()> {
        let adaptation_interval = Duration::from_millis(self.config.adaptation_frequency_ms);
        let adaptation_engine = self.adaptation_engine.clone();
        let neural_network = self.neural_network.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(adaptation_interval);
            loop {
                interval.tick().await;
                
                // Perform continuous adaptation based on recent patterns
                if let Ok(mut adaptation) = adaptation_engine.write().await {
                    if let Ok(mut network) = neural_network.write().await {
                        if let Err(e) = adaptation.continuous_adaptation(&mut network).await {
                            warn!("Continuous adaptation failed: {}", e);
                        }
                    }
                }
            }
        });
        
        info!("Started continuous adaptation loop");
        Ok(())
    }
    
    /// Initialize with STOQ protocol integration
    #[cfg(feature = "stoq-integration")]
    pub async fn initialize_stoq_integration(&self) -> Result<()> {
        let mut stoq_integration = self.stoq_integration.write().await;
        stoq_integration.initialize().await?;
        info!("STOQ integration initialized");
        Ok(())
    }
    
    /// Compute cache key for input pattern and context
    fn compute_cache_key(&self, pattern: &[f64], context: Option<&[f64]>) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        for &value in pattern {
            hasher.update(value.to_le_bytes());
        }
        if let Some(ctx) = context {
            hasher.update(b"CONTEXT");
            for &value in ctx {
                hasher.update(value.to_le_bytes());
            }
        }
        
        format!("{:x}", hasher.finalize())
    }
}

/// DSR system builder for customized configuration
pub struct DsrBuilder {
    config: DsrConfig,
}

impl Default for DsrBuilder {
    fn default() -> Self {
        Self {
            config: DsrConfig::default(),
        }
    }
}

impl DsrBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_neuron_count(mut self, count: usize) -> Self {
        self.config.neuron_count = count;
        self
    }
    
    pub fn with_learning_rate(mut self, rate: f64) -> Self {
        self.config.learning_rate = rate;
        self
    }
    
    pub fn with_similarity_threshold(mut self, threshold: f64) -> Self {
        self.config.similarity_threshold = threshold;
        self
    }
    
    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.config.max_cache_size = size;
        self
    }
    
    pub async fn build(self) -> Result<DsrSystem> {
        DsrSystem::new(self.config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_dsr_system_creation() {
        let config = DsrConfig::default();
        let system = DsrSystem::new(config).await;
        assert!(system.is_ok());
    }
    
    #[tokio::test]
    async fn test_similarity_processing() {
        let system = DsrBuilder::new()
            .with_neuron_count(100)
            .build().await.unwrap();
        
        let pattern = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let context = vec![0.9, 0.8, 0.7, 0.6, 0.5];
        
        let result = system.process_similarity(&pattern, Some(&context)).await;
        assert!(result.is_ok());
        
        let similarity_result = result.unwrap();
        assert!(similarity_result.similarity_score >= 0.0);
        assert!(similarity_result.similarity_score <= 1.0);
    }
    
    #[tokio::test]
    async fn test_caching_functionality() {
        let system = DsrBuilder::new()
            .with_cache_size(100)
            .build().await.unwrap();
        
        let pattern = vec![0.1, 0.2, 0.3];
        
        // First call
        let result1 = system.process_similarity(&pattern, None).await.unwrap();
        assert!(!result1.cache_hit);
        
        // Second call should be cached
        let result2 = system.process_similarity(&pattern, None).await.unwrap();
        assert_eq!(result1.similarity_score, result2.similarity_score);
    }
    
    #[tokio::test]
    async fn test_performance_stats() {
        let system = DsrBuilder::new().build().await.unwrap();
        let stats = system.get_performance_stats().await;
        
        assert!(stats.contains_key("uptime_seconds"));
        assert!(stats.contains_key("total_processed"));
        assert!(stats.contains_key("cache_hit_rate"));
    }
}