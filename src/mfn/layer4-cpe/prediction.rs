//! Context Prediction System
//!
//! This module implements the main prediction system that combines ML models,
//! attention mechanisms, and embedding similarity to predict future contexts.

use anyhow::Result;
use candle_core::Device;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{ContextVector, ContextEmbedding};
use crate::models::{ModelType, ModelFactory, PredictionModel};
use crate::embeddings::ContextEmbedder;

/// Configuration for the prediction system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionConfig {
    pub model_type: ModelType,
    pub context_dimension: usize,
    pub sequence_length: usize,
    pub hidden_size: usize,
    pub num_layers: usize,
    pub prediction_horizon: usize,
    pub confidence_threshold: f64,
    pub enable_gpu: bool,
    pub timeout_ms: u64,
}

impl Default for PredictionConfig {
    fn default() -> Self {
        Self {
            model_type: ModelType::Lstm,
            context_dimension: 256,
            sequence_length: 32,
            hidden_size: 128,
            num_layers: 2,
            prediction_horizon: 5,
            confidence_threshold: 0.7,
            enable_gpu: false,
            timeout_ms: 2,
        }
    }
}

/// Result of a context prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    pub predicted_context: Vec<f32>,
    pub confidence: f32,
    pub prediction_horizon: usize,
    pub model_used: String,
    pub processing_time_ms: f64,
    pub similar_patterns: Vec<String>,
    pub metadata: HashMap<String, f32>,
}

impl PredictionResult {
    pub fn new(predicted_context: Vec<f32>, confidence: f32) -> Self {
        Self {
            predicted_context,
            confidence,
            prediction_horizon: 1,
            model_used: "unknown".to_string(),
            processing_time_ms: 0.0,
            similar_patterns: Vec::new(),
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_metadata(mut self, key: String, value: f32) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    pub fn with_similar_patterns(mut self, patterns: Vec<String>) -> Self {
        self.similar_patterns = patterns;
        self
    }
}

/// Multi-step prediction result for longer horizons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiStepPrediction {
    pub predictions: Vec<PredictionResult>,
    pub aggregated_confidence: f32,
    pub total_processing_time_ms: f64,
}

/// Main context predictor that orchestrates all prediction components
pub struct ContextPredictor {
    config: PredictionConfig,
    model: Arc<RwLock<Box<dyn PredictionModel>>>,
    device: Device,
    
    // Prediction statistics
    prediction_count: Arc<std::sync::atomic::AtomicU64>,
    total_processing_time: Arc<RwLock<Duration>>,
    confidence_scores: Arc<RwLock<Vec<f32>>>,
    
    // Model performance tracking
    model_accuracy: Arc<RwLock<Vec<f32>>>,
    recent_predictions: Arc<RwLock<std::collections::VecDeque<PredictionResult>>>,
}

impl ContextPredictor {
    /// Create a new context predictor
    pub async fn new(config: PredictionConfig) -> Result<Self> {
        info!("Initializing ContextPredictor with model: {:?}", config.model_type);
        
        let device = if config.enable_gpu && Device::nova_if_available(0).is_nova() {
            Device::nova_if_available(0)
        } else {
            Device::Cpu
        };
        
        info!("Using device: {:?}", device);
        
        // Create the prediction model
        let model = ModelFactory::create_model(
            config.model_type,
            config.context_dimension,
            config.hidden_size,
            config.num_layers,
            config.sequence_length,
            device.clone(),
        )?;
        
        let recent_predictions = Arc::new(RwLock::new(
            std::collections::VecDeque::with_capacity(1000)
        ));
        
        Ok(Self {
            config,
            model: Arc::new(RwLock::new(model)),
            device,
            prediction_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_processing_time: Arc::new(RwLock::new(Duration::ZERO)),
            confidence_scores: Arc::new(RwLock::new(Vec::new())),
            model_accuracy: Arc::new(RwLock::new(Vec::new())),
            recent_predictions,
        })
    }
    
    /// Predict the next context in a sequence
    pub async fn predict(
        &mut self,
        context_sequence: &[ContextVector],
        embeddings: Option<&[ContextEmbedding]>,
    ) -> Result<PredictionResult> {
        let start_time = Instant::now();
        
        if context_sequence.is_empty() {
            return Err(anyhow::anyhow!("Empty context sequence provided"));
        }
        
        // Apply timeout check
        let timeout = Duration::from_millis(self.config.timeout_ms);
        
        // Run prediction with timeout
        let prediction_future = self.run_prediction_internal(context_sequence, embeddings);
        
        match tokio::time::timeout(timeout, prediction_future).await {
            Ok(result) => {
                let processing_time = start_time.elapsed();
                self.update_statistics(processing_time, &result).await;
                result
            }
            Err(_) => {
                warn!("Prediction timeout after {:?}", timeout);
                Err(anyhow::anyhow!("Prediction timeout"))
            }
        }
    }
    
    /// Internal prediction logic
    async fn run_prediction_internal(
        &self,
        context_sequence: &[ContextVector],
        embeddings: Option<&[ContextEmbedding]>,
    ) -> Result<PredictionResult> {
        let start_time = Instant::now();
        
        // Get model prediction
        let model_prediction = {
            let model = self.model.read().await;
            model.predict_sequence(context_sequence)?
        };
        
        // Calculate confidence based on sequence consistency and model certainty
        let confidence = self.calculate_prediction_confidence(context_sequence, &model_prediction);
        
        // Enhance with embedding similarity if available
        let similar_patterns = if let Some(embs) = embeddings {
            self.extract_similar_patterns(embs).await
        } else {
            Vec::new()
        };
        
        let processing_time = start_time.elapsed();
        
        let mut result = PredictionResult::new(model_prediction, confidence)
            .with_metadata("sequence_length".to_string(), context_sequence.len() as f32)
            .with_metadata("model_confidence".to_string(), confidence)
            .with_similar_patterns(similar_patterns);
        
        result.model_used = format!("{:?}", self.config.model_type);
        result.processing_time_ms = processing_time.as_secs_f64() * 1000.0;
        result.prediction_horizon = self.config.prediction_horizon;
        
        // Add contextual metadata
        if let Some(last_context) = context_sequence.last() {
            if let Some(&flow_confidence) = last_context.metadata.get("confidence") {
                result = result.with_metadata("flow_confidence".to_string(), flow_confidence);
            }
        }
        
        self.prediction_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        debug!("Prediction completed in {:?} with confidence {:.3}", 
               processing_time, confidence);
        
        Ok(result)
    }
    
    /// Predict multiple steps ahead
    pub async fn predict_multi_step(
        &mut self,
        context_sequence: &[ContextVector],
        steps: usize,
    ) -> Result<MultiStepPrediction> {
        let start_time = Instant::now();
        let mut predictions = Vec::with_capacity(steps);
        let mut current_sequence = context_sequence.to_vec();
        
        for step in 0..steps {
            let prediction = self.predict(&current_sequence, None).await?;
            
            // Create next context from prediction
            let next_context = ContextVector::new(
                current_sequence.last().unwrap().flow_key,
                prediction.predicted_context.clone(),
            );
            
            current_sequence.push(next_context);
            
            // Keep sequence length manageable
            if current_sequence.len() > self.config.sequence_length {
                current_sequence.remove(0);
            }
            
            predictions.push(prediction);
            
            debug!("Multi-step prediction {}/{} completed", step + 1, steps);
        }
        
        // Calculate aggregated confidence
        let aggregated_confidence = predictions.iter()
            .map(|p| p.confidence)
            .sum::<f32>() / predictions.len() as f32;
        
        let total_time = start_time.elapsed();
        
        Ok(MultiStepPrediction {
            predictions,
            aggregated_confidence,
            total_processing_time_ms: total_time.as_secs_f64() * 1000.0,
        })
    }
    
    /// Calculate prediction confidence based on various factors
    fn calculate_prediction_confidence(&self, sequence: &[ContextVector], prediction: &[f32]) -> f32 {
        if sequence.is_empty() || prediction.is_empty() {
            return 0.0;
        }
        
        // Factor 1: Sequence consistency (how similar are recent contexts)
        let consistency_score = self.calculate_sequence_consistency(sequence);
        
        // Factor 2: Prediction certainty (entropy of prediction distribution)
        let certainty_score = self.calculate_prediction_certainty(prediction);
        
        // Factor 3: Historical accuracy (how well have recent predictions performed)
        let historical_score = self.calculate_historical_accuracy();
        
        // Factor 4: Sequence length factor (longer sequences usually provide better context)
        let length_factor = (sequence.len() as f32 / self.config.sequence_length as f32).min(1.0);
        
        // Combine factors with weights
        let confidence = (consistency_score * 0.3) + 
                        (certainty_score * 0.3) + 
                        (historical_score * 0.2) + 
                        (length_factor * 0.2);
        
        confidence.max(0.0).min(1.0)
    }
    
    /// Calculate how consistent the input sequence is
    fn calculate_sequence_consistency(&self, sequence: &[ContextVector]) -> f32 {
        if sequence.len() < 2 {
            return 0.5; // Neutral score for single context
        }
        
        let mut total_similarity = 0.0;
        let mut comparisons = 0;
        
        for i in 1..sequence.len() {
            let similarity = self.cosine_similarity(&sequence[i-1].features, &sequence[i].features);
            total_similarity += similarity;
            comparisons += 1;
        }
        
        if comparisons == 0 {
            0.5
        } else {
            total_similarity / comparisons as f32
        }
    }
    
    /// Calculate prediction certainty based on output distribution
    fn calculate_prediction_certainty(&self, prediction: &[f32]) -> f32 {
        if prediction.is_empty() {
            return 0.0;
        }
        
        // Calculate entropy of the prediction
        let sum: f32 = prediction.iter().map(|x| x.abs()).sum();
        if sum == 0.0 {
            return 0.0;
        }
        
        let normalized: Vec<f32> = prediction.iter().map(|x| x.abs() / sum).collect();
        let entropy: f32 = normalized.iter()
            .filter(|&&p| p > 0.0)
            .map(|&p| -p * p.ln())
            .sum();
        
        let max_entropy = (prediction.len() as f32).ln();
        
        if max_entropy == 0.0 {
            1.0
        } else {
            1.0 - (entropy / max_entropy) // Higher certainty = lower entropy
        }
    }
    
    /// Calculate historical accuracy of recent predictions
    fn calculate_historical_accuracy(&self) -> f32 {
        // This would be populated by the learning system
        // For now, return a default value
        0.8
    }
    
    /// Extract pattern information from embeddings
    async fn extract_similar_patterns(&self, embeddings: &[ContextEmbedding]) -> Vec<String> {
        embeddings.iter()
            .filter_map(|emb| {
                // Extract pattern information from metadata or context
                if emb.metadata.get("confidence").unwrap_or(&0.0) > &0.7 {
                    Some(format!("pattern_{}", emb.pattern_hash))
                } else {
                    None
                }
            })
            .take(5) // Limit to top 5 patterns
            .collect()
    }
    
    /// Cosine similarity between two feature vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let min_len = a.len().min(b.len());
        if min_len == 0 {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).take(min_len).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().take(min_len).map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().take(min_len).map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a * norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }
    
    /// Update prediction statistics
    async fn update_statistics(&self, processing_time: Duration, result: &Result<PredictionResult>) {
        // Update processing time
        {
            let mut total_time = self.total_processing_time.write().await;
            *total_time += processing_time;
        }
        
        // Update confidence scores
        if let Ok(prediction) = result {
            let mut confidence_scores = self.confidence_scores.write().await;
            confidence_scores.push(prediction.confidence);
            
            // Keep only recent scores (last 1000)
            if confidence_scores.len() > 1000 {
                confidence_scores.remove(0);
            }
            
            // Store recent predictions
            let mut recent = self.recent_predictions.write().await;
            recent.push_back(prediction.clone());
            if recent.len() > 1000 {
                recent.pop_front();
            }
        }
    }
    
    /// Get prediction statistics
    pub async fn get_statistics(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        
        let prediction_count = self.prediction_count.load(std::sync::atomic::Ordering::Relaxed);
        stats.insert("total_predictions".to_string(), prediction_count as f64);
        
        if prediction_count > 0 {
            let total_time = self.total_processing_time.read().await;
            let avg_time_ms = total_time.as_secs_f64() * 1000.0 / prediction_count as f64;
            stats.insert("avg_processing_time_ms".to_string(), avg_time_ms);
        }
        
        // Confidence statistics
        let confidence_scores = self.confidence_scores.read().await;
        if !confidence_scores.is_empty() {
            let avg_confidence = confidence_scores.iter().sum::<f32>() / confidence_scores.len() as f32;
            stats.insert("avg_confidence".to_string(), avg_confidence as f64);
            
            let mut sorted_confidences = confidence_scores.clone();
            sorted_confidences.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            let len = sorted_confidences.len();
            if len > 0 {
                stats.insert("min_confidence".to_string(), sorted_confidences[0] as f64);
                stats.insert("max_confidence".to_string(), sorted_confidences[len - 1] as f64);
                stats.insert("median_confidence".to_string(), sorted_confidences[len / 2] as f64);
            }
        }
        
        // Model information
        let model = self.model.read().await;
        let model_info = model.get_model_info();
        for (key, value) in model_info {
            stats.insert(format!("model_{}", key), value as f64);
        }
        
        stats.insert("device_type".to_string(), if self.device.is_nova() { 1.0 } else { 0.0 });
        stats.insert("timeout_ms".to_string(), self.config.timeout_ms as f64);
        stats.insert("prediction_horizon".to_string(), self.config.prediction_horizon as f64);
        
        stats
    }
    
    /// Update model accuracy based on actual outcomes
    pub async fn update_accuracy(&mut self, predicted: &ContextVector, actual: &ContextVector) {
        let accuracy = self.cosine_similarity(&predicted.features, &actual.features);
        
        let mut model_accuracy = self.model_accuracy.write().await;
        model_accuracy.push(accuracy);
        
        // Keep only recent accuracy scores
        if model_accuracy.len() > 1000 {
            model_accuracy.remove(0);
        }
        
        debug!("Model accuracy updated: {:.3}", accuracy);
    }
    
    /// Get recent predictions for analysis
    pub async fn get_recent_predictions(&self, count: usize) -> Vec<PredictionResult> {
        let recent = self.recent_predictions.read().await;
        recent.iter().rev().take(count).cloned().collect()
    }
    
    /// Clear prediction history and reset statistics
    pub async fn reset_statistics(&mut self) {
        self.prediction_count.store(0, std::sync::atomic::Ordering::Relaxed);
        
        *self.total_processing_time.write().await = Duration::ZERO;
        self.confidence_scores.write().await.clear();
        self.model_accuracy.write().await.clear();
        self.recent_predictions.write().await.clear();
        
        info!("Prediction statistics reset");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContextVector;
    
    fn create_test_contexts(count: usize, feature_dim: usize) -> Vec<ContextVector> {
        (0..count).map(|i| {
            let flow_key = [i as u8; 32];
            let features: Vec<f32> = (0..feature_dim)
                .map(|j| (i + j) as f32 * 0.01)
                .collect();
            ContextVector::new(flow_key, features)
        }).collect()
    }
    
    #[tokio::test]
    async fn test_context_predictor_creation() {
        let config = PredictionConfig::default();
        let predictor = ContextPredictor::new(config).await;
        assert!(predictor.is_ok());
    }
    
    #[tokio::test]
    async fn test_single_prediction() {
        let config = PredictionConfig {
            context_dimension: 64,
            sequence_length: 8,
            hidden_size: 32,
            num_layers: 1,
            ..Default::default()
        };
        
        let mut predictor = ContextPredictor::new(config).await.unwrap();
        let contexts = create_test_contexts(5, 64);
        
        let result = predictor.predict(&contexts, None).await;
        assert!(result.is_ok());
        
        let prediction = result.unwrap();
        assert_eq!(prediction.predicted_context.len(), 64);
        assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
        assert!(prediction.processing_time_ms > 0.0);
    }
    
    #[tokio::test]
    async fn test_multi_step_prediction() {
        let config = PredictionConfig {
            context_dimension: 32,
            sequence_length: 4,
            hidden_size: 16,
            num_layers: 1,
            ..Default::default()
        };
        
        let mut predictor = ContextPredictor::new(config).await.unwrap();
        let contexts = create_test_contexts(3, 32);
        
        let result = predictor.predict_multi_step(&contexts, 3).await;
        assert!(result.is_ok());
        
        let multi_prediction = result.unwrap();
        assert_eq!(multi_prediction.predictions.len(), 3);
        assert!(multi_prediction.aggregated_confidence >= 0.0 && multi_prediction.aggregated_confidence <= 1.0);
        assert!(multi_prediction.total_processing_time_ms > 0.0);
    }
    
    #[tokio::test]
    async fn test_prediction_confidence() {
        let config = PredictionConfig {
            context_dimension: 16,
            ..Default::default()
        };
        
        let predictor = ContextPredictor::new(config).await.unwrap();
        
        // Test with consistent sequence
        let consistent_contexts = vec![
            ContextVector::new([1u8; 32], vec![1.0, 2.0, 3.0, 4.0]),
            ContextVector::new([1u8; 32], vec![1.1, 2.1, 3.1, 4.1]),
            ContextVector::new([1u8; 32], vec![1.2, 2.2, 3.2, 4.2]),
        ];
        
        let confidence = predictor.calculate_prediction_confidence(&consistent_contexts, &vec![1.3, 2.3, 3.3, 4.3]);
        assert!(confidence > 0.5); // Should have reasonable confidence
        
        // Test with inconsistent sequence
        let inconsistent_contexts = vec![
            ContextVector::new([1u8; 32], vec![1.0, 2.0, 3.0, 4.0]),
            ContextVector::new([1u8; 32], vec![10.0, 20.0, 30.0, 40.0]),
            ContextVector::new([1u8; 32], vec![-5.0, -10.0, -15.0, -20.0]),
        ];
        
        let confidence2 = predictor.calculate_prediction_confidence(&inconsistent_contexts, &vec![0.0, 0.0, 0.0, 0.0]);
        assert!(confidence > confidence2); // Consistent sequence should have higher confidence
    }
    
    #[tokio::test]
    async fn test_prediction_statistics() {
        let config = PredictionConfig {
            context_dimension: 32,
            ..Default::default()
        };
        
        let mut predictor = ContextPredictor::new(config).await.unwrap();
        let contexts = create_test_contexts(3, 32);
        
        // Make a few predictions
        for _ in 0..5 {
            let _ = predictor.predict(&contexts, None).await;
        }
        
        let stats = predictor.get_statistics().await;
        assert!(stats.contains_key("total_predictions"));
        assert!(stats.contains_key("avg_processing_time_ms"));
        assert!(stats.contains_key("avg_confidence"));
        
        let total_predictions = stats.get("total_predictions").unwrap();
        assert_eq!(*total_predictions, 5.0);
    }
}