//! Adaptation Engine with STDP Learning
//!
//! Implements Spike-Timing-Dependent Plasticity (STDP) and online learning algorithms
//! for dynamic adaptation of the neural network based on routing feedback.

use crate::network_usage::{NeuralNetwork, SynapticConnection};
use crate::spiking::SpikeEvent;
use anyhow::Result;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tracing::{debug, info, trace, warn};

/// STDP learning rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STDPConfig {
    /// Maximum time window for STDP (ms)
    pub time_window: f64,
    /// Learning rate for potentiation (LTP)
    pub ltp_rate: f64,
    /// Learning rate for depression (LTD)  
    pub ltd_rate: f64,
    /// STDP decay time constant (ms)
    pub tau_stdp: f64,
    /// Minimum weight bound
    pub weight_min: f64,
    /// Maximum weight bound
    pub weight_max: f64,
    /// Homeostatic scaling factor
    pub homeostatic_scaling: f64,
}

impl Default for STDPConfig {
    fn default() -> Self {
        Self {
            time_window: 50.0,   // 50ms window
            ltp_rate: 0.01,      // 1% potentiation
            ltd_rate: 0.005,     // 0.5% depression
            tau_stdp: 20.0,      // 20ms time constant
            weight_min: -2.0,    // Minimum weight
            weight_max: 2.0,     // Maximum weight
            homeostatic_scaling: 0.001, // Weak homeostasis
        }
    }
}

/// STDP learning implementation
pub struct STDPLearning {
    config: STDPConfig,
    /// Spike timing records for STDP calculation
    spike_records: HashMap<usize, VecDeque<f64>>,
    /// Connection weight changes history
    weight_changes: HashMap<u64, Vec<(f64, f64)>>, // connection_id -> (time, delta_weight)
    /// Learning statistics
    potentiation_count: u64,
    depression_count: u64,
    total_weight_change: f64,
}

impl STDPLearning {
    pub fn new(config: STDPConfig) -> Self {
        Self {
            config,
            spike_records: HashMap::new(),
            weight_changes: HashMap::new(),
            potentiation_count: 0,
            depression_count: 0,
            total_weight_change: 0.0,
        }
    }
    
    /// Record spike event for STDP calculation
    pub fn record_spike(&mut self, spike: &SpikeEvent) {
        let record = self.spike_records
            .entry(spike.neuron_id)
            .or_insert_with(|| VecDeque::with_capacity(1000));
        
        record.push_back(spike.timestamp);
        
        // Keep only spikes within STDP time window
        let cutoff_time = spike.timestamp - self.config.time_window;
        while let Some(&front_time) = record.front() {
            if front_time < cutoff_time {
                record.pop_front();
            } else {
                break;
            }
        }
    }
    
    /// Apply STDP to a synaptic connection
    pub fn apply_stdp(&mut self, 
        connection: &mut SynapticConnection,
        current_time: f64
    ) -> Result<f64> {
        let pre_spikes = self.spike_records
            .get(&connection.pre_neuron_id)
            .cloned()
            .unwrap_or_default();
        
        let post_spikes = self.spike_records
            .get(&connection.post_neuron_id)
            .cloned()
            .unwrap_or_default();
        
        if pre_spikes.is_empty() || post_spikes.is_empty() {
            return Ok(0.0);
        }
        
        let mut total_weight_change = 0.0;
        
        // Calculate STDP for each pre-post spike pair
        for &pre_time in &pre_spikes {
            for &post_time in &post_spikes {
                let delta_t = post_time - pre_time;
                
                if delta_t.abs() <= self.config.time_window {
                    let stdp_change = self.calculate_stdp_change(delta_t);
                    total_weight_change += stdp_change;
                }
            }
        }
        
        // Normalize by number of spike pairs
        if !pre_spikes.is_empty() && !post_spikes.is_empty() {
            total_weight_change /= (pre_spikes.len() * post_spikes.len()) as f64;
        }
        
        // Apply weight change with bounds
        let old_weight = connection.weight;
        let new_weight = (old_weight + total_weight_change)
            .max(self.config.weight_min)
            .min(self.config.weight_max);
        
        connection.update_weight(new_weight, current_time);
        
        // Record statistics
        let actual_change = new_weight - old_weight;
        if actual_change > 0.0 {
            self.potentiation_count += 1;
        } else if actual_change < 0.0 {
            self.depression_count += 1;
        }
        
        self.total_weight_change += actual_change.abs();
        
        // Record weight change history
        self.weight_changes
            .entry(connection.id)
            .or_insert_with(Vec::new)
            .push((current_time, actual_change));
        
        trace!("STDP applied to connection {}: {:.4} -> {:.4} (Δ={:.4})",
               connection.id, old_weight, new_weight, actual_change);
        
        Ok(actual_change)
    }
    
    /// Calculate STDP weight change for given spike timing difference
    fn calculate_stdp_change(&self, delta_t: f64) -> f64 {
        if delta_t > 0.0 {
            // Post-synaptic spike after pre-synaptic spike -> potentiation (LTP)
            self.config.ltp_rate * (-delta_t / self.config.tau_stdp).exp()
        } else if delta_t < 0.0 {
            // Post-synaptic spike before pre-synaptic spike -> depression (LTD)
            -self.config.ltd_rate * (delta_t / self.config.tau_stdp).exp()
        } else {
            0.0
        }
    }
    
    /// Apply homeostatic scaling to maintain network stability
    pub fn apply_homeostatic_scaling(&mut self, 
        connections: &mut HashMap<u64, SynapticConnection>,
        target_activity: f64,
        current_activity: f64
    ) -> Result<()> {
        if current_activity <= 0.0 {
            return Ok(());
        }
        
        let scaling_factor = 1.0 + self.config.homeostatic_scaling * 
            (target_activity - current_activity) / current_activity;
        
        let mut scaled_connections = 0;
        for (_, connection) in connections.iter_mut() {
            let old_weight = connection.weight;
            let new_weight = (old_weight * scaling_factor)
                .max(self.config.weight_min)
                .min(self.config.weight_max);
            
            if (new_weight - old_weight).abs() > 1e-6 {
                connection.weight = new_weight;
                scaled_connections += 1;
            }
        }
        
        debug!("Homeostatic scaling applied: factor={:.4}, connections={}", 
               scaling_factor, scaled_connections);
        
        Ok(())
    }
    
    /// Get learning statistics
    pub fn get_stats(&self) -> STDPStats {
        STDPStats {
            potentiation_events: self.potentiation_count,
            depression_events: self.depression_count,
            total_weight_change: self.total_weight_change,
            active_connections: self.weight_changes.len(),
            spike_records: self.spike_records.len(),
        }
    }
    
    /// Reset learning state
    pub fn reset(&mut self) {
        self.spike_records.clear();
        self.weight_changes.clear();
        self.potentiation_count = 0;
        self.depression_count = 0;
        self.total_weight_change = 0.0;
    }
}

/// Online learning for continuous adaptation
pub struct OnlineLearning {
    /// Learning rate for online updates
    learning_rate: f64,
    /// Forgetting rate for old patterns
    forgetting_rate: f64,
    /// Recent pattern history
    pattern_history: VecDeque<(Vec<f64>, Vec<f64>, f64)>, // (input, output, target)
    /// Running average of network performance
    performance_history: VecDeque<f64>,
    /// Adaptation momentum for stability
    momentum: f64,
    /// Previous weight updates for momentum
    previous_updates: HashMap<u64, f64>,
}

impl OnlineLearning {
    pub fn new(learning_rate: f64, forgetting_rate: f64) -> Self {
        Self {
            learning_rate,
            forgetting_rate,
            pattern_history: VecDeque::with_capacity(1000),
            performance_history: VecDeque::with_capacity(1000),
            momentum: 0.9,
            previous_updates: HashMap::new(),
        }
    }
    
    /// Add training example for online learning
    pub fn add_training_example(&mut self, 
        input: Vec<f64>,
        output: Vec<f64>, 
        target_similarity: f64
    ) {
        self.pattern_history.push_back((input, output, target_similarity));
        
        // Keep limited history
        if self.pattern_history.len() > 1000 {
            self.pattern_history.pop_front();
        }
    }
    
    /// Perform online adaptation step
    pub async fn adapt_step(&mut self, 
        network_usage: &mut NeuralNetwork,
        recent_performance: f64
    ) -> Result<f64> {
        if self.pattern_history.is_empty() {
            return Ok(0.0);
        }
        
        // Record performance
        self.performance_history.push_back(recent_performance);
        if self.performance_history.len() > 1000 {
            self.performance_history.pop_front();
        }
        
        // Calculate performance gradient
        let performance_trend = self.calculate_performance_trend();
        
        // Adapt learning rate based on performance
        let adaptive_lr = self.learning_rate * (1.0 + 0.1 * performance_trend);
        
        // Apply gradient-based updates
        let total_change = self.apply_gradient_updates(network, adaptive_lr).await?;
        
        // Apply forgetting to prevent overfitting
        self.apply_forgetting(network).await?;
        
        debug!("Online adaptation step: lr={:.4}, change={:.4}, trend={:.4}", 
               adaptive_lr, total_change, performance_trend);
        
        Ok(total_change)
    }
    
    /// Calculate performance trend (gradient)
    fn calculate_performance_trend(&self) -> f64 {
        if self.performance_history.len() < 2 {
            return 0.0;
        }
        
        let window_size = 10.min(self.performance_history.len());
        let recent: Vec<f64> = self.performance_history.iter()
            .rev()
            .take(window_size)
            .cloned()
            .collect();
        
        if recent.len() < 2 {
            return 0.0;
        }
        
        // Simple linear regression for trend
        let n = recent.len() as f64;
        let sum_x = (0..recent.len()).map(|i| i as f64).sum::<f64>();
        let sum_y = recent.iter().sum::<f64>();
        let sum_xy = recent.iter().enumerate()
            .map(|(i, &y)| (i as f64) * y)
            .sum::<f64>();
        let sum_x2 = (0..recent.len()).map(|i| (i as f64).powi(2)).sum::<f64>();
        
        let denominator = n * sum_x2 - sum_x.powi(2);
        if denominator.abs() < 1e-6 {
            return 0.0;
        }
        
        let slope = (n * sum_xy - sum_x * sum_y) / denominator;
        slope
    }
    
    /// Apply gradient-based weight updates
    async fn apply_gradient_updates(&mut self,
        network_usage: &mut NeuralNetwork,
        learning_rate: f64
    ) -> Result<f64> {
        let mut total_change = 0.0;
        let batch_size = 10.min(self.pattern_history.len());
        
        // Sample recent patterns
        let recent_patterns: Vec<_> = self.pattern_history.iter()
            .rev()
            .take(batch_size)
            .cloned()
            .collect();
        
        if recent_patterns.is_empty() {
            return Ok(0.0);
        }
        
        // Calculate weight gradients (simplified)
        let mut weight_gradients = HashMap::new();
        
        for (input, output, target) in &recent_patterns {
            // Simplified error calculation
            let predicted_similarity = output.iter().sum::<f64>() / output.len() as f64;
            let error = target - predicted_similarity;
            
            // Propagate error back through connections (simplified)
            // In practice, this would be more sophisticated backpropagation
            let gradient_scale = error * learning_rate / batch_size as f64;
            
            // Apply to random subset of connections for stability
            let connection_count = network.get_network_stats().connection_count;
            let sample_size = (connection_count / 10).max(1);
            
            let mut rng = thread_rng();
            for _ in 0..sample_size {
                let conn_id = rng.gen::<u64>(); // This would be replaced with actual connection sampling
                let gradient = gradient_scale * (rng.gen::<f64>() - 0.5) * 2.0;
                *weight_gradients.entry(conn_id).or_insert(0.0) += gradient;
            }
        }
        
        // Apply gradients with momentum
        for (conn_id, gradient) in weight_gradients {
            let previous_update = self.previous_updates.get(&conn_id).cloned().unwrap_or(0.0);
            let update = self.momentum * previous_update + (1.0 - self.momentum) * gradient;
            
            // This would apply the actual weight update to the network
            // For now, we track the total change
            total_change += update.abs();
            self.previous_updates.insert(conn_id, update);
        }
        
        Ok(total_change)
    }
    
    /// Apply forgetting to prevent overfitting
    async fn apply_forgetting(&mut self, network_usage: &mut NeuralNetwork) -> Result<()> {
        if self.forgetting_rate <= 0.0 {
            return Ok(());
        }
        
        // Apply weight decay (L2 regularization)
        let decay_factor = 1.0 - self.forgetting_rate;
        
        // This would iterate through all connections and apply decay
        // For now, we just update the tracking statistics
        
        trace!("Applied forgetting with rate {:.4}", self.forgetting_rate);
        Ok(())
    }
    
    /// Get online learning statistics
    pub fn get_stats(&self) -> OnlineLearningStats {
        let avg_performance = if !self.performance_history.is_empty() {
            self.performance_history.iter().sum::<f64>() / self.performance_history.len() as f64
        } else {
            0.0
        };
        
        let performance_trend = self.calculate_performance_trend();
        
        OnlineLearningStats {
            pattern_history_size: self.pattern_history.len(),
            average_performance: avg_performance,
            performance_trend,
            active_updates: self.previous_updates.len(),
        }
    }
}

/// Main adaptation engine combining STDP and online learning
pub struct AdaptationEngine {
    stdp: STDPLearning,
    online: OnlineLearning,
    /// Adaptation frequency control
    last_adaptation: Instant,
    adaptation_interval: Duration,
    /// Performance tracking
    recent_performance: VecDeque<f64>,
    adaptation_count: u64,
}

impl AdaptationEngine {
    pub fn new(learning_rate: f64, forgetting_rate: f64) -> Result<Self> {
        Ok(Self {
            stdp: STDPLearning::new(STDPConfig::default()),
            online: OnlineLearning::new(learning_rate, forgetting_rate),
            last_adaptation: Instant::now(),
            adaptation_interval: Duration::from_millis(100), // 100ms adaptation interval
            recent_performance: VecDeque::with_capacity(100),
            adaptation_count: 0,
        })
    }
    
    /// Record spike for STDP learning
    pub fn record_spike(&mut self, spike: &SpikeEvent) {
        self.stdp.record_spike(spike);
    }
    
    /// Add training example for online learning
    pub fn add_training_example(&mut self,
        input: Vec<f64>,
        output: Vec<f64>,
        target_similarity: f64
    ) {
        self.online.add_training_example(input, output, target_similarity);
        self.recent_performance.push_back(target_similarity);
        
        if self.recent_performance.len() > 100 {
            self.recent_performance.pop_front();
        }
    }
    
    /// Adapt neural network using both STDP and online learning
    pub async fn adapt_network(&mut self, 
        network_usage: &mut NeuralNetwork,
        training_data: &[(Vec<f64>, Vec<f64>, f64)]
    ) -> Result<()> {
        info!("Starting network adaptation with {} training samples", training_data.len());
        
        // Add training data to online learner
        for (input, output, target) in training_data {
            self.online.add_training_example(input.clone(), output.clone(), *target);
        }
        
        // Perform adaptation if enough time has passed
        if self.last_adaptation.elapsed() >= self.adaptation_interval {
            let avg_performance = if !self.recent_performance.is_empty() {
                self.recent_performance.iter().sum::<f64>() / self.recent_performance.len() as f64
            } else {
                0.5 // Default performance
            };
            
            // Apply online adaptation
            let online_change = self.online.adapt_step(network, avg_performance).await?;
            
            // Apply STDP to connections (would need access to connection map)
            // This is simplified - in practice would iterate through all connections
            let stdp_change = 0.0; // Placeholder
            
            // Apply homeostatic scaling
            let target_activity = 10.0; // Target firing rate in Hz
            let current_activity = avg_performance * 20.0; // Estimate from performance
            
            // This would access the actual connection map
            // self.stdp.apply_homeostatic_scaling(connections, target_activity, current_activity)?;
            
            self.last_adaptation = Instant::now();
            self.adaptation_count += 1;
            
            info!("Adaptation completed: online_change={:.4}, count={}", 
                  online_change, self.adaptation_count);
        }
        
        Ok(())
    }
    
    /// Continuous adaptation for real-time learning
    pub async fn continuous_adaptation(&mut self, network_usage: &mut NeuralNetwork) -> Result<()> {
        if self.last_adaptation.elapsed() >= self.adaptation_interval {
            let avg_performance = if !self.recent_performance.is_empty() {
                self.recent_performance.iter().sum::<f64>() / self.recent_performance.len() as f64
            } else {
                0.5
            };
            
            let change = self.online.adapt_step(network, avg_performance).await?;
            self.last_adaptation = Instant::now();
            
            trace!("Continuous adaptation: change={:.6}", change);
        }
        
        Ok(())
    }
    
    /// Get combined adaptation statistics
    pub fn get_adaptation_stats(&self) -> AdaptationStats {
        let stdp_stats = self.stdp.get_stats();
        let online_stats = self.online.get_stats();
        
        AdaptationStats {
            stdp_stats,
            online_stats,
            adaptation_count: self.adaptation_count,
            last_adaptation_ms: self.last_adaptation.elapsed().as_millis() as f64,
            recent_performance: self.recent_performance.iter().cloned().collect(),
        }
    }
    
    /// Reset adaptation state
    pub fn reset_adaptation(&mut self) {
        self.stdp.reset();
        self.recent_performance.clear();
        self.adaptation_count = 0;
        self.last_adaptation = Instant::now();
    }
    
    /// Configure adaptation parameters
    pub fn configure(&mut self, 
        stdp_config: STDPConfig,
        learning_rate: f64,
        forgetting_rate: f64,
        adaptation_interval_ms: u64
    ) {
        self.stdp = STDPLearning::new(stdp_config);
        self.online.learning_rate = learning_rate;
        self.online.forgetting_rate = forgetting_rate;
        self.adaptation_interval = Duration::from_millis(adaptation_interval_ms);
    }
}

/// STDP learning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STDPStats {
    pub potentiation_events: u64,
    pub depression_events: u64,
    pub total_weight_change: f64,
    pub active_connections: usize,
    pub spike_records: usize,
}

/// Online learning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineLearningStats {
    pub pattern_history_size: usize,
    pub average_performance: f64,
    pub performance_trend: f64,
    pub active_updates: usize,
}

/// Combined adaptation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationStats {
    pub stdp_stats: STDPStats,
    pub online_stats: OnlineLearningStats,
    pub adaptation_count: u64,
    pub last_adaptation_ms: f64,
    pub recent_performance: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spiking::SpikeEvent;
    
    #[test]
    fn test_stdp_creation() {
        let stdp = STDPLearning::new(STDPConfig::default());
        let stats = stdp.get_stats();
        assert_eq!(stats.potentiation_events, 0);
        assert_eq!(stats.depression_events, 0);
    }
    
    #[test]
    fn test_spike_recording() {
        let mut stdp = STDPLearning::new(STDPConfig::default());
        
        let spike = SpikeEvent {
            neuron_id: 5,
            timestamp: 10.0,
            voltage_at_spike: -45.0,
        };
        
        stdp.record_spike(&spike);
        
        assert!(stdp.spike_records.contains_key(&5));
        assert_eq!(stdp.spike_records[&5].len(), 1);
    }
    
    #[test]
    fn test_stdp_calculation() {
        let stdp = STDPLearning::new(STDPConfig::default());
        
        // Test potentiation (post after pre)
        let ltp_change = stdp.calculate_stdp_change(10.0);
        assert!(ltp_change > 0.0);
        
        // Test depression (post before pre)  
        let ltd_change = stdp.calculate_stdp_change(-10.0);
        assert!(ltd_change < 0.0);
        
        // Test no change at zero delay
        let no_change = stdp.calculate_stdp_change(0.0);
        assert_eq!(no_change, 0.0);
    }
    
    #[tokio::test]
    async fn test_online_learning() {
        let mut online = OnlineLearning::new(0.01, 0.001);
        
        let input = vec![0.5, 0.3, 0.8];
        let output = vec![0.1, 0.7, 0.2];
        let target = 0.8;
        
        online.add_training_example(input, output, target);
        
        let stats = online.get_stats();
        assert_eq!(stats.pattern_history_size, 1);
    }
    
    #[tokio::test] 
    async fn test_adaptation_engine() {
        let mut engine = AdaptationEngine::new(0.01, 0.001).unwrap();
        
        let spike = SpikeEvent {
            neuron_id: 3,
            timestamp: 15.0,
            voltage_at_spike: -50.0,
        };
        
        engine.record_spike(&spike);
        engine.add_training_example(vec![0.1, 0.2], vec![0.8, 0.3], 0.9);
        
        let stats = engine.get_adaptation_stats();
        assert!(stats.stdp_stats.spike_records > 0);
        assert!(stats.online_stats.pattern_history_size > 0);
    }
    
    #[test]
    fn test_performance_trend() {
        let mut online = OnlineLearning::new(0.01, 0.001);
        
        // Add increasing performance trend
        for i in 0..20 {
            online.performance_history.push_back(i as f64 / 20.0);
        }
        
        let trend = online.calculate_performance_trend();
        assert!(trend > 0.0); // Should detect positive trend
    }
    
    #[tokio::test]
    async fn test_stdp_weight_bounds() {
        let mut stdp = STDPLearning::new(STDPConfig {
            weight_min: -1.0,
            weight_max: 1.0,
            ..Default::default()
        });
        
        let mut connection = SynapticConnection::new(0, 1, true);
        connection.weight = 0.9; // Near upper bound
        
        // Record spikes that would cause potentiation
        let pre_spike = SpikeEvent {
            neuron_id: 0,
            timestamp: 10.0,
            voltage_at_spike: -45.0,
        };
        let post_spike = SpikeEvent {
            neuron_id: 1,
            timestamp: 15.0, // After pre-spike
            voltage_at_spike: -45.0,
        };
        
        stdp.record_spike(&pre_spike);
        stdp.record_spike(&post_spike);
        
        let change = stdp.apply_stdp(&mut connection, 20.0).unwrap();
        
        // Weight should be bounded
        assert!(connection.weight <= 1.0);
        assert!(connection.weight >= -1.0);
    }
}