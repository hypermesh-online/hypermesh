//! Spiking Neural Network Implementation
//!
//! This module implements Leaky Integrate-and-Fire (LIF) spiking neurons with
//! realistic biological dynamics for network pattern recognition and routing decisions.

use anyhow::Result;
use rand::{Rng, thread_rng};
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, trace};

/// Spiking neuron state representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuronState {
    /// Membrane potential (voltage)
    pub voltage: f64,
    /// Threshold for spike generation
    pub threshold: f64,
    /// Resting potential
    pub resting_potential: f64,
    /// Membrane time constant (ms)
    pub tau_membrane: f64,
    /// Refractory period duration (ms)
    pub refractory_period: f64,
    /// Time since last spike (ms)
    pub time_since_spike: f64,
    /// Is neuron in refractory period
    pub in_refractory: bool,
    /// Accumulated current input
    pub input_current: f64,
    /// Spike history for STDP
    pub spike_history: Vec<f64>,
    /// Neuron unique identifier
    pub id: usize,
}

impl NeuronState {
    pub fn new(id: usize) -> Self {
        let mut rng = thread_rng();
        
        Self {
            voltage: -70.0, // Typical resting potential in mV
            threshold: -55.0, // Typical spike threshold in mV
            resting_potential: -70.0,
            tau_membrane: 20.0, // 20ms membrane time constant
            refractory_period: 2.0, // 2ms absolute refractory period
            time_since_spike: 100.0, // Start well past refractory
            in_refractory: false,
            input_current: 0.0,
            spike_history: Vec::with_capacity(1000),
            id,
        }
    }
    
    /// Create neuron with randomized parameters for diversity
    pub fn new_randomized(id: usize) -> Self {
        let mut rng = thread_rng();
        let normal_threshold = Normal::new(-55.0, 2.0).unwrap();
        let normal_tau = Normal::new(20.0, 3.0).unwrap();
        
        Self {
            voltage: -70.0,
            threshold: normal_threshold.sample(&mut rng),
            resting_potential: -70.0,
            tau_membrane: normal_tau.sample(&mut rng).max(10.0).min(40.0),
            refractory_period: 1.0 + rng.gen::<f64>() * 2.0, // 1-3ms
            time_since_spike: 100.0,
            in_refractory: false,
            input_current: 0.0,
            spike_history: Vec::with_capacity(1000),
            id,
        }
    }
}

/// Spike event with timing and neuron ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeEvent {
    pub neuron_id: usize,
    pub timestamp: f64,
    pub voltage_at_spike: f64,
}

/// Leaky Integrate-and-Fire spiking neuron
pub struct SpikingNeuron {
    state: NeuronState,
    noise_amplitude: f64,
    adaptation_current: f64,
    adaptation_decay: f64,
}

impl SpikingNeuron {
    pub fn new(id: usize) -> Self {
        Self {
            state: NeuronState::new(id),
            noise_amplitude: 0.5, // Small amount of noise for realism
            adaptation_current: 0.0,
            adaptation_decay: 0.95, // Adaptation current decay
        }
    }
    
    pub fn new_randomized(id: usize) -> Self {
        let mut rng = thread_rng();
        
        Self {
            state: NeuronState::new_randomized(id),
            noise_amplitude: 0.1 + rng.gen::<f64>() * 0.8, // 0.1-0.9
            adaptation_current: 0.0,
            adaptation_decay: 0.9 + rng.gen::<f64>() * 0.09, // 0.9-0.99
        }
    }
    
    /// Update neuron dynamics for one time step
    pub fn update(&mut self, dt: f64, input_current: f64) -> Option<SpikeEvent> {
        // Update time since last spike
        self.state.time_since_spike += dt;
        
        // Check if still in refractory period
        if self.state.in_refractory {
            if self.state.time_since_spike >= self.state.refractory_period {
                self.state.in_refractory = false;
            } else {
                // During refractory, voltage is clamped to resting potential
                self.state.voltage = self.state.resting_potential;
                return None;
            }
        }
        
        // Add noise for realistic dynamics
        let mut rng = thread_rng();
        let noise = if self.noise_amplitude > 0.0 {
            Normal::new(0.0, self.noise_amplitude).unwrap().sample(&mut rng)
        } else {
            0.0
        };
        
        // Total input current including noise and adaptation
        let total_current = input_current + noise - self.adaptation_current;
        
        // Leaky integrate-and-fire dynamics
        // dV/dt = (V_rest - V + R*I_total) / tau
        let voltage_derivative = (self.state.resting_potential - self.state.voltage + total_current) 
            / self.state.tau_membrane;
        
        // Euler integration
        self.state.voltage += voltage_derivative * dt;
        
        // Check for spike
        if self.state.voltage >= self.state.threshold {
            let spike_event = SpikeEvent {
                neuron_id: self.state.id,
                timestamp: 0.0, // Will be set by caller
                voltage_at_spike: self.state.voltage,
            };
            
            // Reset after spike
            self.state.voltage = self.state.resting_potential;
            self.state.time_since_spike = 0.0;
            self.state.in_refractory = true;
            
            // Add spike-frequency adaptation
            self.adaptation_current += 2.0; // Increase adaptation current
            
            // Record spike in history
            self.state.spike_history.push(0.0); // Timestamp will be updated by caller
            if self.state.spike_history.len() > 1000 {
                self.state.spike_history.remove(0);
            }
            
            trace!("Neuron {} spiked at voltage {:.2}mV", 
                  self.state.id, spike_event.voltage_at_spike);
            
            return Some(spike_event);
        }
        
        // Decay adaptation current
        self.adaptation_current *= self.adaptation_decay;
        
        None
    }
    
    /// Get current neuron state
    pub fn get_state(&self) -> &NeuronState {
        &self.state
    }
    
    /// Get mutable reference to state for direct manipulation
    pub fn get_state_mut(&mut self) -> &mut NeuronState {
        &mut self.state
    }
    
    /// Set input current
    pub fn set_input_current(&mut self, current: f64) {
        self.state.input_current = current;
    }
    
    /// Add to input current (for accumulating multiple inputs)
    pub fn add_input_current(&mut self, current: f64) {
        self.state.input_current += current;
    }
    
    /// Reset input current to zero
    pub fn reset_input_current(&mut self) {
        self.state.input_current = 0.0;
    }
    
    /// Get spike history for STDP calculations
    pub fn get_spike_history(&self, window_ms: f64, current_time: f64) -> Vec<f64> {
        self.state.spike_history.iter()
            .filter(|&&spike_time| current_time - spike_time <= window_ms)
            .cloned()
            .collect()
    }
    
    /// Calculate firing rate over a given window
    pub fn get_firing_rate(&self, window_ms: f64, current_time: f64) -> f64 {
        let recent_spikes = self.get_spike_history(window_ms, current_time);
        (recent_spikes.len() as f64) / (window_ms / 1000.0) // Convert to Hz
    }
    
    /// Reset neuron to initial state
    pub fn reset(&mut self) {
        self.state.voltage = self.state.resting_potential;
        self.state.time_since_spike = 100.0;
        self.state.in_refractory = false;
        self.state.input_current = 0.0;
        self.adaptation_current = 0.0;
        self.state.spike_history.clear();
    }
    
    /// Check if neuron is currently excitable
    pub fn is_excitable(&self) -> bool {
        !self.state.in_refractory
    }
    
    /// Get neuron responsiveness (how close to threshold)
    pub fn get_responsiveness(&self) -> f64 {
        if self.state.in_refractory {
            return 0.0;
        }
        
        let voltage_range = self.state.threshold - self.state.resting_potential;
        let current_excitation = self.state.voltage - self.state.resting_potential;
        
        (current_excitation / voltage_range).max(0.0).min(1.0)
    }
    
    /// Set neuron parameters for tuning
    pub fn set_parameters(&mut self, threshold: f64, tau: f64, noise: f64) {
        self.state.threshold = threshold;
        self.state.tau_membrane = tau;
        self.noise_amplitude = noise;
    }
    
    /// Get neuron ID
    pub fn get_id(&self) -> usize {
        self.state.id
    }
}

/// Population of spiking neurons with synchronized dynamics
pub struct NeuronPopulation {
    neurons: Vec<SpikingNeuron>,
    current_time: f64,
    dt: f64, // Time step for integration
}

impl NeuronPopulation {
    pub fn new(size: usize, randomized: bool) -> Self {
        let neurons = if randomized {
            (0..size).map(SpikingNeuron::new_randomized).collect()
        } else {
            (0..size).map(SpikingNeuron::new).collect()
        };
        
        Self {
            neurons,
            current_time: 0.0,
            dt: 0.1, // 0.1ms time step
        }
    }
    
    /// Update all neurons and collect spike events
    pub fn update_population(&mut self, input_currents: &[f64]) -> Vec<SpikeEvent> {
        assert_eq!(input_currents.len(), self.neurons.len());
        
        let mut spike_events = Vec::new();
        
        for (i, neuron) in self.neurons.iter_mut().enumerate() {
            if let Some(mut spike) = neuron.update(self.dt, input_currents[i]) {
                spike.timestamp = self.current_time;
                
                // Update spike history with actual timestamp
                if let Some(last_spike) = neuron.state.spike_history.last_mut() {
                    *last_spike = self.current_time;
                }
                
                spike_events.push(spike);
            }
        }
        
        self.current_time += self.dt;
        spike_events
    }
    
    /// Get population firing rate
    pub fn get_population_firing_rate(&self, window_ms: f64) -> f64 {
        let total_rate: f64 = self.neurons.iter()
            .map(|n| n.get_firing_rate(window_ms, self.current_time))
            .sum();
        
        total_rate / self.neurons.len() as f64
    }
    
    /// Get neurons that spiked in the last time window
    pub fn get_active_neurons(&self, window_ms: f64) -> Vec<usize> {
        self.neurons.iter()
            .enumerate()
            .filter_map(|(i, neuron)| {
                if neuron.get_firing_rate(window_ms, self.current_time) > 0.0 {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Reset all neurons
    pub fn reset_population(&mut self) {
        for neuron in &mut self.neurons {
            neuron.reset();
        }
        self.current_time = 0.0;
    }
    
    /// Get reference to specific neuron
    pub fn get_neuron(&self, id: usize) -> Option<&SpikingNeuron> {
        self.neurons.get(id)
    }
    
    /// Get mutable reference to specific neuron
    pub fn get_neuron_mut(&mut self, id: usize) -> Option<&mut SpikingNeuron> {
        self.neurons.get_mut(id)
    }
    
    /// Get population size
    pub fn size(&self) -> usize {
        self.neurons.len()
    }
    
    /// Get current simulation time
    pub fn get_current_time(&self) -> f64 {
        self.current_time
    }
    
    /// Set time step for integration
    pub fn set_time_step(&mut self, dt: f64) {
        self.dt = dt;
    }
    
    /// Get population state summary
    pub fn get_population_summary(&self) -> PopulationSummary {
        let active_count = self.neurons.iter()
            .filter(|n| n.is_excitable())
            .count();
        
        let avg_voltage = self.neurons.iter()
            .map(|n| n.get_state().voltage)
            .sum::<f64>() / self.neurons.len() as f64;
        
        let avg_responsiveness = self.neurons.iter()
            .map(|n| n.get_responsiveness())
            .sum::<f64>() / self.neurons.len() as f64;
        
        PopulationSummary {
            total_neurons: self.neurons.len(),
            active_neurons: active_count,
            average_voltage: avg_voltage,
            average_responsiveness: avg_responsiveness,
            current_time: self.current_time,
            firing_rate: self.get_population_firing_rate(100.0), // Last 100ms
        }
    }
}

/// Summary statistics for neuron population
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationSummary {
    pub total_neurons: usize,
    pub active_neurons: usize,
    pub average_voltage: f64,
    pub average_responsiveness: f64,
    pub current_time: f64,
    pub firing_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_neuron_creation() {
        let neuron = SpikingNeuron::new(0);
        assert_eq!(neuron.get_id(), 0);
        assert_eq!(neuron.get_state().voltage, -70.0);
        assert!(neuron.is_excitable());
    }
    
    #[test]
    fn test_neuron_spike() {
        let mut neuron = SpikingNeuron::new(0);
        
        // Apply strong current to cause spike
        let dt = 0.1;
        let strong_current = 100.0; // Large current to overcome threshold quickly
        
        let mut spiked = false;
        for _ in 0..1000 { // Try for up to 100ms
            if let Some(spike) = neuron.update(dt, strong_current) {
                assert_eq!(spike.neuron_id, 0);
                spiked = true;
                break;
            }
        }
        
        assert!(spiked, "Neuron should spike with strong input");
    }
    
    #[test]
    fn test_refractory_period() {
        let mut neuron = SpikingNeuron::new(0);
        let dt = 0.1;
        let strong_current = 100.0;
        
        // First spike
        let mut first_spike = None;
        for _ in 0..1000 {
            if let Some(spike) = neuron.update(dt, strong_current) {
                first_spike = Some(spike);
                break;
            }
        }
        
        assert!(first_spike.is_some());
        
        // Should be in refractory period now
        assert!(!neuron.is_excitable());
        
        // Wait for refractory period to pass
        for _ in 0..30 { // 3ms should be enough
            neuron.update(dt, 0.0);
        }
        
        // Should be excitable again
        assert!(neuron.is_excitable());
    }
    
    #[test]
    fn test_population_dynamics() {
        let mut population = NeuronPopulation::new(10, false);
        let inputs = vec![10.0; 10]; // Moderate input to all neurons
        
        let mut total_spikes = 0;
        for _ in 0..1000 { // Run for 100ms
            let spikes = population.update_population(&inputs);
            total_spikes += spikes.len();
        }
        
        assert!(total_spikes > 0, "Population should produce spikes");
        
        let firing_rate = population.get_population_firing_rate(100.0);
        assert!(firing_rate > 0.0, "Population should have non-zero firing rate");
    }
    
    #[test]
    fn test_spike_history() {
        let mut neuron = SpikingNeuron::new(0);
        let dt = 0.1;
        let strong_current = 100.0;
        
        // Generate some spikes
        for i in 0..1000 {
            if let Some(_) = neuron.update(dt, if i % 100 < 50 { strong_current } else { 0.0 }) {
                // Spike occurred
            }
        }
        
        let history = neuron.get_spike_history(50.0, 100.0); // Last 50ms
        // Should have some spikes in history
        // (exact number depends on refractory period and adaptation)
    }
    
    #[test]
    fn test_randomized_neurons() {
        let neuron1 = SpikingNeuron::new_randomized(0);
        let neuron2 = SpikingNeuron::new_randomized(1);
        
        // Neurons should have different parameters
        assert_ne!(neuron1.noise_amplitude, neuron2.noise_amplitude);
        // Thresholds might be the same by chance, but very unlikely
    }
}