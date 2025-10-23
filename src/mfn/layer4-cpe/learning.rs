//! Online Learning System
//!
//! This module implements online learning algorithms that continuously adapt
//! the prediction models based on real-time feedback and performance metrics.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use rand::Rng;

use crate::{ContextVector, models::PredictionModel};
use crate::prediction::{ContextPredictor, PredictionResult};

/// Learning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    pub learning_rate: f64,
    pub adaptation_threshold: f64,
    pub batch_size: usize,
    pub strategy: AdaptationStrategy,
    pub enabled: bool,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            adaptation_threshold: 0.1,
            batch_size: 32,
            strategy: AdaptationStrategy::GradualDescent,
            enabled: true,
        }
    }
}

/// Adaptation strategies for online learning
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdaptationStrategy {
    /// Standard gradient descent
    GradualDescent,
    /// Adaptive learning rate based on performance
    AdaptiveLearningRate,
    /// Experience replay with priority sampling
    ExperienceReplay,
    /// Meta-learning adaptation
    MetaLearning,
}

/// Training example for online learning
#[derive(Debug, Clone)]
pub struct TrainingExample {
    pub context_sequence: Vec<ContextVector>,
    pub target_context: ContextVector,
    pub prediction_result: Option<PredictionResult>,
    pub loss: f32,
    pub timestamp: Instant,
    pub importance_weight: f32,
}

impl TrainingExample {
    pub fn new(
        context_sequence: Vec<ContextVector>,
        target_context: ContextVector,
        prediction_result: Option<PredictionResult>,
    ) -> Self {
        let loss = if let Some(ref pred) = prediction_result {
            Self::calculate_loss(&pred.predicted_context, &target_context.features)
        } else {
            1.0 // High loss if no prediction available
        };
        
        Self {
            context_sequence,
            target_context,
            prediction_result,
            loss,
            timestamp: Instant::now(),
            importance_weight: 1.0,
        }
    }
    
    pub fn with_importance_weight(mut self, weight: f32) -> Self {
        self.importance_weight = weight;
        self
    }
    
    fn calculate_loss(predicted: &[f32], actual: &[f32]) -> f32 {
        if predicted.len() != actual.len() {
            return 1.0;
        }
        
        let mse: f32 = predicted.iter()
            .zip(actual.iter())
            .map(|(p, a)| (p - a).powi(2))
            .sum::<f32>() / predicted.len() as f32;
        
        mse.sqrt() // RMSE
    }
}

/// Experience replay buffer for storing training examples
pub struct ExperienceBuffer {
    buffer: VecDeque<TrainingExample>,
    max_size: usize,
    priority_sampling: bool,
    total_importance: f32,
}

impl ExperienceBuffer {
    pub fn new(max_size: usize, priority_sampling: bool) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_size),
            max_size,
            priority_sampling,
            total_importance: 0.0,
        }
    }
    
    pub fn add_example(&mut self, example: TrainingExample) {
        if self.buffer.len() >= self.max_size {
            if let Some(old_example) = self.buffer.pop_front() {
                self.total_importance -= old_example.importance_weight;
            }
        }
        
        self.total_importance += example.importance_weight;
        self.buffer.push_back(example);
    }
    
    pub fn sample_batch(&self, batch_size: usize) -> Vec<&TrainingExample> {
        if self.buffer.is_empty() {
            return Vec::new();
        }
        
        let mut batch = Vec::with_capacity(batch_size);
        let mut rng = rand::thread_rng();
        
        if self.priority_sampling && self.total_importance > 0.0 {
            // Priority sampling based on importance weights
            for _ in 0..batch_size.min(self.buffer.len()) {
                let target = rng.gen::<f32>() * self.total_importance;
                let mut cumsum = 0.0;
                
                for example in &self.buffer {
                    cumsum += example.importance_weight;
                    if cumsum >= target {
                        batch.push(example);
                        break;
                    }
                }
            }
        } else {
            // Uniform random sampling
            for _ in 0..batch_size.min(self.buffer.len()) {
                let idx = rng.gen_range(0..self.buffer.len());
                batch.push(&self.buffer[idx]);
            }
        }
        
        batch
    }
    
    pub fn get_recent_examples(&self, count: usize) -> Vec<&TrainingExample> {
        self.buffer.iter()
            .rev()
            .take(count)
            .collect()
    }
    
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
    
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.total_importance = 0.0;
    }
}

/// Learning rate scheduler
pub struct LearningRateScheduler {
    initial_lr: f64,
    current_lr: f64,
    decay_factor: f64,
    min_lr: f64,
    performance_window: VecDeque<f32>,
    window_size: usize,
    adaptation_count: u64,
}

impl LearningRateScheduler {
    pub fn new(initial_lr: f64, decay_factor: f64, min_lr: f64, window_size: usize) -> Self {
        Self {
            initial_lr,
            current_lr: initial_lr,
            decay_factor,
            min_lr,
            performance_window: VecDeque::with_capacity(window_size),
            window_size,
            adaptation_count: 0,
        }
    }
    
    pub fn get_learning_rate(&self) -> f64 {
        self.current_lr
    }
    
    pub fn update(&mut self, performance_metric: f32) {
        self.performance_window.push_back(performance_metric);
        if self.performance_window.len() > self.window_size {
            self.performance_window.pop_front();
        }
        
        self.adaptation_count += 1;
        
        // Adaptive learning rate based on performance trend
        if self.performance_window.len() >= self.window_size {
            let recent_avg = self.performance_window.iter()
                .rev()
                .take(self.window_size / 2)
                .sum::<f32>() / (self.window_size / 2) as f32;
            
            let older_avg = self.performance_window.iter()
                .take(self.window_size / 2)
                .sum::<f32>() / (self.window_size / 2) as f32;
            
            if recent_avg < older_avg {
                // Performance is improving, maintain or slightly increase LR
                self.current_lr = (self.current_lr * 1.01).min(self.initial_lr * 2.0);
            } else {
                // Performance is degrading, decrease LR
                self.current_lr = (self.current_lr * self.decay_factor).max(self.min_lr);
            }
        }
        
        debug!("Learning rate updated to: {:.6}", self.current_lr);
    }
    
    pub fn reset(&mut self) {
        self.current_lr = self.initial_lr;
        self.performance_window.clear();
        self.adaptation_count = 0;
    }
}

/// Main online learner that coordinates model adaptation
pub struct OnlineLearner {
    config: LearningConfig,
    predictor: Arc<RwLock<ContextPredictor>>,
    experience_buffer: Arc<RwLock<ExperienceBuffer>>,
    lr_scheduler: Arc<RwLock<LearningRateScheduler>>,
    
    // Learning statistics
    adaptation_count: Arc<std::sync::atomic::AtomicU64>,
    total_loss: Arc<RwLock<f64>>,
    recent_losses: Arc<RwLock<VecDeque<f32>>>,
    learning_history: Arc<RwLock<Vec<LearningEvent>>>,
    
    // Performance tracking
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    last_adaptation: Arc<RwLock<Option<Instant>>>,
}

/// Learning event for tracking adaptation history
#[derive(Debug, Clone)]
struct LearningEvent {
    timestamp: Instant,
    strategy_used: AdaptationStrategy,
    learning_rate: f64,
    batch_loss: f32,
    examples_processed: usize,
    adaptation_time_ms: f64,
}

/// Performance metrics for learning system
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    average_loss: f32,
    loss_trend: f32, // Positive = improving, Negative = degrading
    adaptation_frequency: f64, // Adaptations per hour
    learning_efficiency: f32, // Improvement per adaptation
    convergence_rate: f32,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            average_loss: 1.0,
            loss_trend: 0.0,
            adaptation_frequency: 0.0,
            learning_efficiency: 0.0,
            convergence_rate: 0.0,
        }
    }
}

impl OnlineLearner {
    /// Create a new online learner
    pub async fn new(
        config: LearningConfig,
        predictor: Arc<RwLock<ContextPredictor>>,
    ) -> Result<Self> {
        info!("Initializing OnlineLearner with strategy: {:?}", config.strategy);
        
        let experience_buffer = Arc::new(RwLock::new(ExperienceBuffer::new(
            10000, // Max buffer size
            config.strategy == AdaptationStrategy::ExperienceReplay,
        )));
        
        let lr_scheduler = Arc::new(RwLock::new(LearningRateScheduler::new(
            config.learning_rate,
            0.95, // Decay factor
            config.learning_rate * 0.01, // Min LR
            20, // Window size
        )));
        
        Ok(Self {
            config,
            predictor,
            experience_buffer,
            lr_scheduler,
            adaptation_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_loss: Arc::new(RwLock::new(0.0)),
            recent_losses: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            learning_history: Arc::new(RwLock::new(Vec::new())),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            last_adaptation: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Learn from a single example
    pub async fn learn_from_example(
        &mut self,
        predicted_context: &ContextVector,
        actual_context: &ContextVector,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // Create training example
        let example = TrainingExample::new(
            vec![predicted_context.clone()],
            actual_context.clone(),
            None, // Prediction result would be passed in real implementation
        );
        
        // Add to experience buffer
        {
            let mut buffer = self.experience_buffer.write().await;
            buffer.add_example(example);
        }
        
        // Check if we should trigger batch learning
        let should_adapt = self.should_trigger_adaptation().await;
        
        if should_adapt {
            self.trigger_batch_learning().await?;
        }
        
        Ok(())
    }
    
    /// Learn from a batch of examples
    pub async fn learn_from_batch(&mut self, examples: &[TrainingExample]) -> Result<()> {
        if !self.config.enabled || examples.is_empty() {
            return Ok(());
        }
        
        let start_time = Instant::now();
        let mut batch_loss = 0.0;
        
        // Add examples to buffer
        {
            let mut buffer = self.experience_buffer.write().await;
            for example in examples {
                buffer.add_example(example.clone());
                batch_loss += example.loss;
            }
        }
        
        batch_loss /= examples.len() as f32;
        
        // Perform adaptation based on strategy
        match self.config.strategy {
            AdaptationStrategy::GradualDescent => {
                self.gradual_descent_update(examples).await?;
            }
            AdaptationStrategy::AdaptiveLearningRate => {
                self.adaptive_lr_update(examples).await?;
            }
            AdaptationStrategy::ExperienceReplay => {
                self.experience_replay_update().await?;
            }
            AdaptationStrategy::MetaLearning => {
                self.meta_learning_update(examples).await?;
            }
        }
        
        // Update statistics
        self.update_learning_statistics(batch_loss, examples.len(), start_time.elapsed()).await;
        
        debug!("Batch learning completed: {} examples, loss: {:.4}", examples.len(), batch_loss);
        Ok(())
    }
    
    /// Gradual descent update
    async fn gradual_descent_update(&self, examples: &[TrainingExample]) -> Result<()> {
        let lr = self.lr_scheduler.read().await.get_learning_rate();
        
        // In a real implementation, this would update model weights
        // For now, we simulate the update
        info!("Gradual descent update with LR: {:.6}, examples: {}", lr, examples.len());
        
        // Update learning rate scheduler
        let avg_loss = examples.iter().map(|e| e.loss).sum::<f32>() / examples.len() as f32;
        self.lr_scheduler.write().await.update(avg_loss);
        
        Ok(())
    }
    
    /// Adaptive learning rate update
    async fn adaptive_lr_update(&self, examples: &[TrainingExample]) -> Result<()> {
        let mut scheduler = self.lr_scheduler.write().await;
        let avg_loss = examples.iter().map(|e| e.loss).sum::<f32>() / examples.len() as f32;
        
        // Update based on loss trend
        scheduler.update(avg_loss);
        let current_lr = scheduler.get_learning_rate();
        
        info!("Adaptive LR update: {:.6}, loss: {:.4}", current_lr, avg_loss);
        Ok(())
    }
    
    /// Experience replay update
    async fn experience_replay_update(&self) -> Result<()> {
        let buffer = self.experience_buffer.read().await;
        let batch = buffer.sample_batch(self.config.batch_size);
        
        if batch.is_empty() {
            return Ok(());
        }
        
        // Process prioritized batch
        let total_loss: f32 = batch.iter().map(|e| e.loss * e.importance_weight).sum();
        let weighted_avg_loss = total_loss / batch.iter().map(|e| e.importance_weight).sum::<f32>();
        
        info!("Experience replay update: {} examples, weighted loss: {:.4}", 
              batch.len(), weighted_avg_loss);
        
        // Update importance weights based on learning success
        // (This would modify the examples in the buffer)
        
        Ok(())
    }
    
    /// Meta-learning update
    async fn meta_learning_update(&self, examples: &[TrainingExample]) -> Result<()> {
        // Meta-learning adaptation based on recent performance patterns
        let recent_performance = self.analyze_recent_performance().await;
        
        // Adjust learning strategy based on meta-analysis
        let strategy_effectiveness = self.evaluate_strategy_effectiveness().await;
        
        info!("Meta-learning update: performance trend: {:.3}, strategy effectiveness: {:.3}",
              recent_performance, strategy_effectiveness);
        
        // In a full implementation, this would dynamically adjust hyperparameters
        // or even switch between different learning algorithms
        
        Ok(())
    }
    
    /// Check if adaptation should be triggered
    async fn should_trigger_adaptation(&self) -> bool {
        let buffer = self.experience_buffer.read().await;
        
        // Trigger if buffer has enough examples
        if buffer.len() >= self.config.batch_size {
            return true;
        }
        
        // Trigger if enough time has passed since last adaptation
        let last_adaptation = self.last_adaptation.read().await;
        if let Some(last_time) = *last_adaptation {
            if last_time.elapsed() > Duration::from_secs(60) {
                return true;
            }
        } else {
            return true; // First adaptation
        }
        
        // Trigger if recent performance has degraded significantly
        let recent_losses = self.recent_losses.read().await;
        if recent_losses.len() >= 10 {
            let recent_avg = recent_losses.iter().rev().take(5).sum::<f32>() / 5.0;
            let older_avg = recent_losses.iter().take(5).sum::<f32>() / 5.0;
            
            if recent_avg > older_avg * 1.2 { // 20% degradation
                return true;
            }
        }
        
        false
    }
    
    /// Trigger batch learning
    async fn trigger_batch_learning(&mut self) -> Result<()> {
        let batch = {
            let buffer = self.experience_buffer.read().await;
            buffer.sample_batch(self.config.batch_size)
        };
        
        if !batch.is_empty() {
            let examples: Vec<TrainingExample> = batch.iter().map(|&e| e.clone()).collect();
            self.learn_from_batch(&examples).await?;
        }
        
        // Update last adaptation time
        *self.last_adaptation.write().await = Some(Instant::now());
        
        Ok(())
    }
    
    /// Periodic optimization (called by background task)
    pub async fn periodic_optimization(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // Clean old examples from buffer
        self.cleanup_old_examples().await;
        
        // Update performance metrics
        self.update_performance_metrics().await;
        
        // Check if model needs reinitialization or major adjustment
        let should_reset = self.should_reset_model().await;
        if should_reset {
            self.reset_learning_state().await;
            info!("Learning state reset due to poor performance");
        }
        
        Ok(())
    }
    
    /// Clean up old examples from experience buffer
    async fn cleanup_old_examples(&self) {
        let mut buffer = self.experience_buffer.write().await;
        let cutoff_time = Instant::now() - Duration::from_secs(3600); // 1 hour
        
        // Remove examples older than cutoff
        while let Some(front) = buffer.buffer.front() {
            if front.timestamp < cutoff_time {
                if let Some(old_example) = buffer.buffer.pop_front() {
                    buffer.total_importance -= old_example.importance_weight;
                }
            } else {
                break;
            }
        }
    }
    
    /// Update performance metrics
    async fn update_performance_metrics(&self) {
        let recent_losses = self.recent_losses.read().await;
        
        if recent_losses.len() < 10 {
            return;
        }
        
        let mut metrics = self.performance_metrics.write().await;
        
        // Calculate average loss
        metrics.average_loss = recent_losses.iter().sum::<f32>() / recent_losses.len() as f32;
        
        // Calculate loss trend (improvement rate)
        let mid_point = recent_losses.len() / 2;
        let recent_half: f32 = recent_losses.iter().rev().take(mid_point).sum::<f32>() / mid_point as f32;
        let older_half: f32 = recent_losses.iter().take(mid_point).sum::<f32>() / mid_point as f32;
        
        metrics.loss_trend = (older_half - recent_half) / older_half; // Positive = improving
        
        // Calculate adaptation frequency
        let adaptation_count = self.adaptation_count.load(std::sync::atomic::Ordering::Relaxed);
        let history = self.learning_history.read().await;
        
        if let Some(first_event) = history.first() {
            let hours_elapsed = first_event.timestamp.elapsed().as_secs_f64() / 3600.0;
            if hours_elapsed > 0.0 {
                metrics.adaptation_frequency = adaptation_count as f64 / hours_elapsed;
            }
        }
        
        // Calculate learning efficiency
        if history.len() >= 2 {
            let recent_loss = history.last().unwrap().batch_loss;
            let older_loss = history[history.len() / 2].batch_loss;
            metrics.learning_efficiency = (older_loss - recent_loss) / adaptation_count as f32;
        }
    }
    
    /// Analyze recent performance for meta-learning
    async fn analyze_recent_performance(&self) -> f32 {
        let metrics = self.performance_metrics.read().await;
        
        // Combine multiple performance indicators
        let trend_score = metrics.loss_trend.max(-1.0).min(1.0); // Clamp to [-1, 1]
        let efficiency_score = metrics.learning_efficiency.max(-1.0).min(1.0);
        
        (trend_score + efficiency_score) / 2.0
    }
    
    /// Evaluate strategy effectiveness
    async fn evaluate_strategy_effectiveness(&self) -> f32 {
        let history = self.learning_history.read().await;
        
        if history.len() < 5 {
            return 0.5; // Neutral score for insufficient data
        }
        
        // Analyze loss reduction rate with current strategy
        let recent_events: Vec<_> = history.iter().rev().take(5).collect();
        let mut improvements = 0;
        
        for i in 1..recent_events.len() {
            if recent_events[i].batch_loss < recent_events[i-1].batch_loss {
                improvements += 1;
            }
        }
        
        improvements as f32 / (recent_events.len() - 1) as f32
    }
    
    /// Check if model should be reset
    async fn should_reset_model(&self) -> bool {
        let metrics = self.performance_metrics.read().await;
        
        // Reset if performance has been consistently bad
        metrics.loss_trend < -0.5 && metrics.average_loss > 2.0
    }
    
    /// Reset learning state
    async fn reset_learning_state(&self) {
        // Clear experience buffer
        self.experience_buffer.write().await.clear();
        
        // Reset learning rate scheduler
        self.lr_scheduler.write().await.reset();
        
        // Clear statistics
        *self.total_loss.write().await = 0.0;
        self.recent_losses.write().await.clear();
        self.learning_history.write().await.clear();
        
        // Reset metrics
        *self.performance_metrics.write().await = PerformanceMetrics::default();
        
        self.adaptation_count.store(0, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// Update learning statistics
    async fn update_learning_statistics(
        &self,
        batch_loss: f32,
        examples_processed: usize,
        adaptation_time: Duration,
    ) {
        // Update total loss
        *self.total_loss.write().await += batch_loss as f64;
        
        // Update recent losses
        {
            let mut recent = self.recent_losses.write().await;
            recent.push_back(batch_loss);
            if recent.len() > 100 {
                recent.pop_front();
            }
        }
        
        // Record learning event
        let event = LearningEvent {
            timestamp: Instant::now(),
            strategy_used: self.config.strategy,
            learning_rate: self.lr_scheduler.read().await.get_learning_rate(),
            batch_loss,
            examples_processed,
            adaptation_time_ms: adaptation_time.as_secs_f64() * 1000.0,
        };
        
        {
            let mut history = self.learning_history.write().await;
            history.push(event);
            
            // Keep only recent history
            if history.len() > 1000 {
                history.remove(0);
            }
        }
        
        self.adaptation_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// Get learning statistics
    pub async fn get_statistics(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        
        let adaptation_count = self.adaptation_count.load(std::sync::atomic::Ordering::Relaxed);
        stats.insert("adaptation_count".to_string(), adaptation_count as f64);
        
        let total_loss = *self.total_loss.read().await;
        if adaptation_count > 0 {
            stats.insert("average_loss_per_adaptation".to_string(), total_loss / adaptation_count as f64);
        }
        
        let recent_losses = self.recent_losses.read().await;
        if !recent_losses.is_empty() {
            let avg_recent_loss = recent_losses.iter().sum::<f32>() / recent_losses.len() as f32;
            stats.insert("recent_average_loss".to_string(), avg_recent_loss as f64);
        }
        
        let buffer = self.experience_buffer.read().await;
        stats.insert("experience_buffer_size".to_string(), buffer.len() as f64);
        
        let lr = self.lr_scheduler.read().await.get_learning_rate();
        stats.insert("current_learning_rate".to_string(), lr);
        
        let metrics = self.performance_metrics.read().await;
        stats.insert("loss_trend".to_string(), metrics.loss_trend as f64);
        stats.insert("adaptation_frequency".to_string(), metrics.adaptation_frequency);
        stats.insert("learning_efficiency".to_string(), metrics.learning_efficiency as f64);
        
        stats.insert("strategy".to_string(), self.config.strategy as u8 as f64);
        stats.insert("enabled".to_string(), if self.config.enabled { 1.0 } else { 0.0 });
        
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prediction::{ContextPredictor, PredictionConfig};
    
    fn create_test_context(features: Vec<f32>) -> ContextVector {
        let flow_key = [0u8; 32];
        ContextVector::new(flow_key, features)
    }
    
    #[tokio::test]
    async fn test_online_learner_creation() {
        let config = LearningConfig::default();
        let pred_config = PredictionConfig::default();
        let predictor = Arc::new(RwLock::new(ContextPredictor::new(pred_config).await.unwrap()));
        
        let learner = OnlineLearner::new(config, predictor).await;
        assert!(learner.is_ok());
    }
    
    #[tokio::test]
    async fn test_training_example() {
        let context_seq = vec![create_test_context(vec![1.0, 2.0, 3.0])];
        let target = create_test_context(vec![1.1, 2.1, 3.1]);
        
        let example = TrainingExample::new(context_seq, target, None);
        assert!(example.loss > 0.0);
        assert_eq!(example.importance_weight, 1.0);
    }
    
    #[tokio::test]
    async fn test_experience_buffer() {
        let mut buffer = ExperienceBuffer::new(3, false);
        
        // Add examples
        for i in 0..5 {
            let context_seq = vec![create_test_context(vec![i as f32])];
            let target = create_test_context(vec![i as f32 + 0.1]);
            let example = TrainingExample::new(context_seq, target, None);
            buffer.add_example(example);
        }
        
        // Should only keep the most recent 3 examples
        assert_eq!(buffer.len(), 3);
        
        // Sample batch
        let batch = buffer.sample_batch(2);
        assert_eq!(batch.len(), 2);
    }
    
    #[tokio::test]
    async fn test_learning_rate_scheduler() {
        let mut scheduler = LearningRateScheduler::new(0.01, 0.9, 0.001, 10);
        
        assert_eq!(scheduler.get_learning_rate(), 0.01);
        
        // Simulate improving performance
        for _ in 0..15 {
            scheduler.update(0.5 - 0.01 * scheduler.adaptation_count as f32);
        }
        
        // Learning rate should have increased or stayed stable
        assert!(scheduler.get_learning_rate() >= 0.01 * 0.9);
    }
    
    #[tokio::test]
    async fn test_learning_from_example() {
        let config = LearningConfig {
            batch_size: 2,
            ..Default::default()
        };
        
        let pred_config = PredictionConfig::default();
        let predictor = Arc::new(RwLock::new(ContextPredictor::new(pred_config).await.unwrap()));
        
        let mut learner = OnlineLearner::new(config, predictor).await.unwrap();
        
        let predicted = create_test_context(vec![1.0, 2.0, 3.0]);
        let actual = create_test_context(vec![1.1, 2.1, 3.1]);
        
        let result = learner.learn_from_example(&predicted, &actual).await;
        assert!(result.is_ok());
        
        let stats = learner.get_statistics().await;
        assert!(stats.get("experience_buffer_size").unwrap() >= &1.0);
    }
    
    #[test]
    fn test_adaptation_strategies() {
        let strategies = [
            AdaptationStrategy::GradualDescent,
            AdaptationStrategy::AdaptiveLearningRate,
            AdaptationStrategy::ExperienceReplay,
            AdaptationStrategy::MetaLearning,
        ];
        
        for strategy in &strategies {
            let config = LearningConfig {
                strategy: *strategy,
                ..Default::default()
            };
            assert_eq!(config.strategy, *strategy);
        }
    }
}