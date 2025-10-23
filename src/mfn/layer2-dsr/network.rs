//! Neural Network Topology and Connectivity
//!
//! Implements the network structure connecting spiking neurons with synaptic connections,
//! including topology management, connection patterns, and network dynamics.

use crate::spiking::{SpikingNeuron, SpikeEvent, NeuronPopulation};
use anyhow::Result;
use rand::{Rng, thread_rng};
use rand_distr::{Distribution, Normal, Uniform};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};

/// Synaptic connection between neurons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynapticConnection {
    pub pre_neuron_id: usize,
    pub post_neuron_id: usize,
    pub weight: f64,
    pub delay_ms: f64,
    pub is_excitatory: bool,
    pub last_update: f64,
    /// Connection strength history for plasticity
    pub weight_history: Vec<(f64, f64)>, // (timestamp, weight)
    /// Unique connection identifier
    pub id: u64,
}

impl SynapticConnection {
    pub fn new(pre_id: usize, post_id: usize, is_excitatory: bool) -> Self {
        let mut rng = thread_rng();
        let weight = if is_excitatory {
            // Excitatory weights: 0.1 to 1.0
            rng.gen::<f64>() * 0.9 + 0.1
        } else {
            // Inhibitory weights: -1.0 to -0.1
            -(rng.gen::<f64>() * 0.9 + 0.1)
        };
        
        let delay = rng.gen::<f64>() * 5.0 + 0.5; // 0.5-5.5ms delay
        let id = rng.gen::<u64>();
        
        Self {
            pre_neuron_id: pre_id,
            post_neuron_id: post_id,
            weight,
            delay_ms: delay,
            is_excitatory,
            last_update: 0.0,
            weight_history: Vec::with_capacity(1000),
            id,
        }
    }
    
    /// Create connection with specific weight
    pub fn with_weight(pre_id: usize, post_id: usize, weight: f64) -> Self {
        let mut rng = thread_rng();
        let is_excitatory = weight > 0.0;
        let delay = rng.gen::<f64>() * 5.0 + 0.5;
        let id = rng.gen::<u64>();
        
        Self {
            pre_neuron_id: pre_id,
            post_neuron_id: post_id,
            weight,
            delay_ms: delay,
            is_excitatory,
            last_update: 0.0,
            weight_history: Vec::with_capacity(1000),
            id,
        }
    }
    
    /// Update connection weight with history tracking
    pub fn update_weight(&mut self, new_weight: f64, timestamp: f64) {
        self.weight_history.push((timestamp, self.weight));
        if self.weight_history.len() > 1000 {
            self.weight_history.remove(0);
        }
        
        self.weight = new_weight;
        self.last_update = timestamp;
        self.is_excitatory = new_weight > 0.0;
    }
    
    /// Get connection strength (absolute weight)
    pub fn get_strength(&self) -> f64 {
        self.weight.abs()
    }
    
    /// Check if connection is valid (has reasonable parameters)
    pub fn is_valid(&self) -> bool {
        self.weight.is_finite() && 
        self.delay_ms > 0.0 && 
        self.delay_ms < 100.0 && // Max 100ms delay
        self.pre_neuron_id != self.post_neuron_id
    }
}

/// Network topology patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkTopology {
    /// Random connections with specified probability
    Random { connection_probability: f64 },
    /// Small-world network (Watts-Strogatz)
    SmallWorld { k_neighbors: usize, rewiring_probability: f64 },
    /// Scale-free network (Barabási-Albert)
    ScaleFree { m_edges: usize },
    /// Layered feed-forward network
    LayeredFeedForward { layers: Vec<usize> },
    /// Grid topology with local connections
    Grid2D { width: usize, height: usize, connection_radius: f64 },
    /// Custom topology from connection matrix
    Custom { connections: Vec<SynapticConnection> },
}

/// Neural network with spiking neurons and synaptic connections
pub struct NeuralNetwork {
    neurons: NeuronPopulation,
    connections: HashMap<u64, SynapticConnection>,
    topology: NetworkTopology,
    
    /// Connection lookup tables for efficient processing
    incoming_connections: HashMap<usize, Vec<u64>>, // neuron_id -> connection_ids
    outgoing_connections: HashMap<usize, Vec<u64>>, // neuron_id -> connection_ids
    
    /// Delayed spike queue for synaptic transmission
    spike_queue: Vec<(f64, SpikeEvent, u64)>, // (arrival_time, spike, connection_id)
    
    /// Network statistics
    total_spikes: u64,
    last_activity_time: f64,
    
    /// Excitation/inhibition ratio (typically 4:1 in cortex)
    excitatory_ratio: f64,
}

impl NeuralNetwork {
    /// Create new neural network with specified topology
    pub async fn new(neuron_count: usize, avg_connections: usize) -> Result<Self> {
        info!("Creating neural network with {} neurons, ~{} connections per neuron", 
              neuron_count, avg_connections);
        
        let neurons = NeuronPopulation::new(neuron_count, true); // Randomized neurons
        let topology = NetworkTopology::Random { 
            connection_probability: avg_connections as f64 / neuron_count as f64 
        };
        
        let mut network = Self {
            neurons,
            connections: HashMap::new(),
            topology,
            incoming_connections: HashMap::new(),
            outgoing_connections: HashMap::new(),
            spike_queue: Vec::new(),
            total_spikes: 0,
            last_activity_time: 0.0,
            excitatory_ratio: 0.8, // 80% excitatory, 20% inhibitory
        };
        
        network.generate_connections().await?;
        network.build_connection_lookup_tables();
        
        info!("Neural network created with {} connections", network.connections.len());
        Ok(network)
    }
    
    /// Create network with specific topology
    pub async fn with_topology(topology: NetworkTopology) -> Result<Self> {
        let neuron_count = match &topology {
            NetworkTopology::Random { .. } => 1000,
            NetworkTopology::SmallWorld { .. } => 1000,
            NetworkTopology::ScaleFree { .. } => 1000,
            NetworkTopology::LayeredFeedForward { layers } => layers.iter().sum(),
            NetworkTopology::Grid2D { width, height, .. } => width * height,
            NetworkTopology::Custom { connections } => {
                let max_id = connections.iter()
                    .map(|c| c.pre_neuron_id.max(c.post_neuron_id))
                    .max()
                    .unwrap_or(0);
                max_id + 1
            }
        };
        
        let neurons = NeuronPopulation::new(neuron_count, true);
        
        let mut network = Self {
            neurons,
            connections: HashMap::new(),
            topology,
            incoming_connections: HashMap::new(),
            outgoing_connections: HashMap::new(),
            spike_queue: Vec::new(),
            total_spikes: 0,
            last_activity_time: 0.0,
            excitatory_ratio: 0.8,
        };
        
        network.generate_connections().await?;
        network.build_connection_lookup_tables();
        
        Ok(network)
    }
    
    /// Process input and propagate through network
    pub async fn process_input(&mut self, 
        input_pattern: &[f64], 
        context: Option<&[f64]>
    ) -> Result<Vec<f64>> {
        let start_time = self.neurons.get_current_time();
        
        // Prepare input currents
        let mut input_currents = vec![0.0; self.neurons.size()];
        
        // Map input pattern to input currents
        self.apply_input_pattern(&mut input_currents, input_pattern, context);
        
        // Run network simulation for sufficient time to see response
        let simulation_duration = 50.0; // 50ms simulation
        let mut output_spikes = Vec::new();
        
        for _ in 0..500 { // 500 steps of 0.1ms each
            // Update all neurons
            let new_spikes = self.neurons.update_population(&input_currents);
            
            // Process new spikes through synaptic connections
            self.process_spikes(&new_spikes);
            
            // Update delayed spike queue and apply delayed inputs
            self.update_spike_queue(&mut input_currents);
            
            // Collect output spikes for analysis
            output_spikes.extend(new_spikes);
            
            // Reset input currents for next iteration (only external input persists)
            for i in 0..input_currents.len() {
                // Keep only external input, zero synaptic contributions
                if i < input_pattern.len() {
                    input_currents[i] = input_pattern[i] * 10.0; // Scale factor
                } else {
                    input_currents[i] = 0.0;
                }
            }
        }
        
        // Convert spike trains to output vector
        let output = self.compute_network_output(&output_spikes, simulation_duration);
        
        self.total_spikes += output_spikes.len() as u64;
        self.last_activity_time = self.neurons.get_current_time();
        
        debug!("Network processed input: {} spikes generated", output_spikes.len());
        Ok(output)
    }
    
    /// Apply input pattern to network
    fn apply_input_pattern(&self, 
        input_currents: &mut [f64], 
        pattern: &[f64], 
        context: Option<&[f64]>
    ) {
        // Map input pattern to first N neurons
        for (i, &value) in pattern.iter().enumerate() {
            if i < input_currents.len() {
                input_currents[i] += value * 10.0; // Scale input
            }
        }
        
        // Add context if provided
        if let Some(ctx) = context {
            let context_start = pattern.len().min(input_currents.len() / 2);
            for (i, &value) in ctx.iter().enumerate() {
                let idx = context_start + i;
                if idx < input_currents.len() {
                    input_currents[idx] += value * 5.0; // Lower weight for context
                }
            }
        }
    }
    
    /// Process spikes and add to delay queue
    fn process_spikes(&mut self, spikes: &[SpikeEvent]) {
        for spike in spikes {
            if let Some(connection_ids) = self.outgoing_connections.get(&spike.neuron_id) {
                for &conn_id in connection_ids {
                    if let Some(connection) = self.connections.get(&conn_id) {
                        let arrival_time = spike.timestamp + connection.delay_ms;
                        self.spike_queue.push((arrival_time, spike.clone(), conn_id));
                    }
                }
            }
        }
        
        // Sort spike queue by arrival time
        self.spike_queue.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }
    
    /// Update spike queue and apply delayed synaptic inputs
    fn update_spike_queue(&mut self, input_currents: &mut [f64]) {
        let current_time = self.neurons.get_current_time();
        
        // Find spikes that should arrive now
        let mut i = 0;
        while i < self.spike_queue.len() {
            if self.spike_queue[i].0 <= current_time {
                let (_, spike, conn_id) = self.spike_queue.remove(i);
                
                if let Some(connection) = self.connections.get(&conn_id) {
                    // Apply synaptic current to post-synaptic neuron
                    if connection.post_neuron_id < input_currents.len() {
                        input_currents[connection.post_neuron_id] += connection.weight;
                    }
                }
            } else {
                i += 1;
            }
        }
    }
    
    /// Convert spike train to output vector
    fn compute_network_output(&self, spikes: &[SpikeEvent], duration: f64) -> Vec<f64> {
        let neuron_count = self.neurons.size();
        let mut firing_rates = vec![0.0; neuron_count];
        
        // Count spikes per neuron
        for spike in spikes {
            if spike.neuron_id < firing_rates.len() {
                firing_rates[spike.neuron_id] += 1.0;
            }
        }
        
        // Convert to firing rates (Hz)
        for rate in &mut firing_rates {
            *rate = (*rate / (duration / 1000.0)).min(100.0); // Cap at 100Hz
        }
        
        firing_rates
    }
    
    /// Generate connections based on topology
    async fn generate_connections(&mut self) -> Result<()> {
        match &self.topology {
            NetworkTopology::Random { connection_probability } => {
                self.generate_random_connections(*connection_probability).await?;
            },
            NetworkTopology::SmallWorld { k_neighbors, rewiring_probability } => {
                self.generate_small_world_connections(*k_neighbors, *rewiring_probability).await?;
            },
            NetworkTopology::ScaleFree { m_edges } => {
                self.generate_scale_free_connections(*m_edges).await?;
            },
            NetworkTopology::LayeredFeedForward { layers } => {
                self.generate_feedforward_connections(layers).await?;
            },
            NetworkTopology::Grid2D { width, height, connection_radius } => {
                self.generate_grid_connections(*width, *height, *connection_radius).await?;
            },
            NetworkTopology::Custom { connections } => {
                for conn in connections {
                    if conn.is_valid() {
                        self.connections.insert(conn.id, conn.clone());
                    }
                }
            },
        }
        
        info!("Generated {} connections for topology: {:?}", 
              self.connections.len(), self.topology);
        Ok(())
    }
    
    /// Generate random connections
    async fn generate_random_connections(&mut self, prob: f64) -> Result<()> {
        let mut rng = thread_rng();
        let neuron_count = self.neurons.size();
        
        for pre in 0..neuron_count {
            for post in 0..neuron_count {
                if pre != post && rng.gen::<f64>() < prob {
                    let is_excitatory = rng.gen::<f64>() < self.excitatory_ratio;
                    let connection = SynapticConnection::new(pre, post, is_excitatory);
                    self.connections.insert(connection.id, connection);
                }
            }
        }
        
        Ok(())
    }
    
    /// Generate small-world network (Watts-Strogatz model)
    async fn generate_small_world_connections(&mut self, k: usize, rewire_prob: f64) -> Result<()> {
        let mut rng = thread_rng();
        let n = self.neurons.size();
        
        // Start with regular ring lattice
        for i in 0..n {
            for j in 1..=k/2 {
                let post = (i + j) % n;
                let is_excitatory = rng.gen::<f64>() < self.excitatory_ratio;
                
                // Forward connection
                let conn1 = SynapticConnection::new(i, post, is_excitatory);
                self.connections.insert(conn1.id, conn1);
                
                // Backward connection
                let conn2 = SynapticConnection::new(post, i, is_excitatory);
                self.connections.insert(conn2.id, conn2);
            }
        }
        
        // Rewire connections
        let connection_ids: Vec<u64> = self.connections.keys().cloned().collect();
        for &conn_id in &connection_ids {
            if rng.gen::<f64>() < rewire_prob {
                if let Some(mut connection) = self.connections.remove(&conn_id) {
                    // Rewire to random target
                    let new_post = rng.gen_range(0..n);
                    if new_post != connection.pre_neuron_id {
                        connection.post_neuron_id = new_post;
                        self.connections.insert(connection.id, connection);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Generate scale-free network (Barabási-Albert model)
    async fn generate_scale_free_connections(&mut self, m: usize) -> Result<()> {
        let mut rng = thread_rng();
        let n = self.neurons.size();
        
        // Start with complete graph of m+1 nodes
        for i in 0..=(m.min(n-1)) {
            for j in (i+1)..=(m.min(n-1)) {
                let is_excitatory = rng.gen::<f64>() < self.excitatory_ratio;
                let conn1 = SynapticConnection::new(i, j, is_excitatory);
                let conn2 = SynapticConnection::new(j, i, is_excitatory);
                self.connections.insert(conn1.id, conn1);
                self.connections.insert(conn2.id, conn2);
            }
        }
        
        // Add remaining nodes with preferential attachment
        for new_node in (m+1)..n {
            let mut degree_sum: usize = self.connections.values()
                .filter(|c| c.post_neuron_id < new_node)
                .count();
            
            if degree_sum == 0 { degree_sum = 1; } // Avoid division by zero
            
            let mut added_connections = 0;
            let mut targets = HashSet::new();
            
            while added_connections < m && targets.len() < new_node {
                let target = rng.gen_range(0..new_node);
                if targets.insert(target) {
                    let target_degree = self.connections.values()
                        .filter(|c| c.post_neuron_id == target || c.pre_neuron_id == target)
                        .count();
                    
                    let attachment_prob = target_degree as f64 / degree_sum as f64;
                    
                    if rng.gen::<f64>() < attachment_prob {
                        let is_excitatory = rng.gen::<f64>() < self.excitatory_ratio;
                        let conn = SynapticConnection::new(new_node, target, is_excitatory);
                        self.connections.insert(conn.id, conn);
                        added_connections += 1;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Generate feedforward layered connections
    async fn generate_feedforward_connections(&mut self, layers: &[usize]) -> Result<()> {
        let mut rng = thread_rng();
        let mut neuron_offset = 0;
        
        for i in 0..(layers.len()-1) {
            let current_layer_size = layers[i];
            let next_layer_size = layers[i+1];
            let next_layer_offset = neuron_offset + current_layer_size;
            
            // Connect each neuron in current layer to neurons in next layer
            for pre in neuron_offset..(neuron_offset + current_layer_size) {
                // Connect to random subset of next layer
                let connection_count = rng.gen_range(1..=next_layer_size);
                let mut targets: Vec<usize> = (next_layer_offset..(next_layer_offset + next_layer_size)).collect();
                targets.shuffle(&mut rng);
                
                for &post in targets.iter().take(connection_count) {
                    let is_excitatory = rng.gen::<f64>() < self.excitatory_ratio;
                    let conn = SynapticConnection::new(pre, post, is_excitatory);
                    self.connections.insert(conn.id, conn);
                }
            }
            
            neuron_offset += current_layer_size;
        }
        
        Ok(())
    }
    
    /// Generate 2D grid connections
    async fn generate_grid_connections(&mut self, width: usize, height: usize, radius: f64) -> Result<()> {
        let mut rng = thread_rng();
        
        for i in 0..height {
            for j in 0..width {
                let neuron_id = i * width + j;
                
                // Connect to neighbors within radius
                for di in -(radius as i32)..=(radius as i32) {
                    for dj in -(radius as i32)..=(radius as i32) {
                        if di == 0 && dj == 0 { continue; }
                        
                        let ni = i as i32 + di;
                        let nj = j as i32 + dj;
                        
                        if ni >= 0 && ni < height as i32 && nj >= 0 && nj < width as i32 {
                            let distance = ((di * di + dj * dj) as f64).sqrt();
                            if distance <= radius {
                                let neighbor_id = (ni as usize) * width + (nj as usize);
                                let connection_prob = 1.0 - (distance / radius);
                                
                                if rng.gen::<f64>() < connection_prob {
                                    let is_excitatory = rng.gen::<f64>() < self.excitatory_ratio;
                                    let conn = SynapticConnection::new(neuron_id, neighbor_id, is_excitatory);
                                    self.connections.insert(conn.id, conn);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Build lookup tables for efficient connection processing
    fn build_connection_lookup_tables(&mut self) {
        self.incoming_connections.clear();
        self.outgoing_connections.clear();
        
        for (&conn_id, connection) in &self.connections {
            // Outgoing connections
            self.outgoing_connections
                .entry(connection.pre_neuron_id)
                .or_insert_with(Vec::new)
                .push(conn_id);
            
            // Incoming connections
            self.incoming_connections
                .entry(connection.post_neuron_id)
                .or_insert_with(Vec::new)
                .push(conn_id);
        }
        
        debug!("Built connection lookup tables for {} connections", self.connections.len());
    }
    
    /// Get network statistics
    pub fn get_network_stats(&self) -> NetworkStats {
        let total_connections = self.connections.len();
        let excitatory_connections = self.connections.values()
            .filter(|c| c.is_excitatory)
            .count();
        let inhibitory_connections = total_connections - excitatory_connections;
        
        let avg_weight = self.connections.values()
            .map(|c| c.weight.abs())
            .sum::<f64>() / total_connections as f64;
        
        let max_degree = self.outgoing_connections.values()
            .map(|v| v.len())
            .max()
            .unwrap_or(0);
        
        let avg_degree = self.outgoing_connections.values()
            .map(|v| v.len())
            .sum::<usize>() as f64 / self.neurons.size() as f64;
        
        NetworkStats {
            neuron_count: self.neurons.size(),
            connection_count: total_connections,
            excitatory_connections,
            inhibitory_connections,
            average_weight: avg_weight,
            average_degree: avg_degree,
            max_degree,
            total_spikes: self.total_spikes,
            last_activity_time: self.last_activity_time,
        }
    }
    
    /// Get specific connection
    pub fn get_connection(&self, conn_id: u64) -> Option<&SynapticConnection> {
        self.connections.get(&conn_id)
    }
    
    /// Update connection weight
    pub fn update_connection_weight(&mut self, conn_id: u64, new_weight: f64) -> Result<()> {
        if let Some(connection) = self.connections.get_mut(&conn_id) {
            let current_time = self.neurons.get_current_time();
            connection.update_weight(new_weight, current_time);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Connection {} not found", conn_id))
        }
    }
    
    /// Get neuron population reference
    pub fn get_neurons(&self) -> &NeuronPopulation {
        &self.neurons
    }
    
    /// Get mutable neuron population reference
    pub fn get_neurons_mut(&mut self) -> &mut NeuronPopulation {
        &mut self.neurons
    }
    
    /// Reset network state
    pub fn reset_network(&mut self) {
        self.neurons.reset_population();
        self.spike_queue.clear();
        self.total_spikes = 0;
        self.last_activity_time = 0.0;
    }
}

/// Network performance and connectivity statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub neuron_count: usize,
    pub connection_count: usize,
    pub excitatory_connections: usize,
    pub inhibitory_connections: usize,
    pub average_weight: f64,
    pub average_degree: f64,
    pub max_degree: usize,
    pub total_spikes: u64,
    pub last_activity_time: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_network_creation() {
        let network = NeuralNetwork::new(100, 10).await;
        assert!(network.is_ok());
        
        let network = network.unwrap();
        let stats = network.get_network_stats();
        assert_eq!(stats.neuron_count, 100);
        assert!(stats.connection_count > 0);
    }
    
    #[tokio::test]
    async fn test_network_processing() {
        let mut network = NeuralNetwork::new(50, 5).await.unwrap();
        
        let input = vec![0.5, 0.3, 0.8, 0.1, 0.9];
        let result = network.process_input(&input, None).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert_eq!(output.len(), 50);
        assert!(output.iter().all(|&x| x >= 0.0 && x <= 100.0));
    }
    
    #[tokio::test]
    async fn test_small_world_topology() {
        let topology = NetworkTopology::SmallWorld {
            k_neighbors: 4,
            rewiring_probability: 0.1,
        };
        
        let network = NeuralNetwork::with_topology(topology).await;
        assert!(network.is_ok());
        
        let network = network.unwrap();
        let stats = network.get_network_stats();
        assert!(stats.connection_count > 0);
    }
    
    #[tokio::test]
    async fn test_connection_updates() {
        let mut network = NeuralNetwork::new(10, 3).await.unwrap();
        let stats = network.get_network_stats();
        
        if stats.connection_count > 0 {
            let conn_id = network.connections.keys().next().cloned().unwrap();
            let original_weight = network.connections[&conn_id].weight;
            
            let new_weight = original_weight * 1.5;
            let result = network.update_connection_weight(conn_id, new_weight);
            assert!(result.is_ok());
            
            assert_eq!(network.connections[&conn_id].weight, new_weight);
        }
    }
    
    #[test]
    fn test_synaptic_connection() {
        let conn = SynapticConnection::new(0, 1, true);
        assert_eq!(conn.pre_neuron_id, 0);
        assert_eq!(conn.post_neuron_id, 1);
        assert!(conn.is_excitatory);
        assert!(conn.weight > 0.0);
        assert!(conn.is_valid());
    }
    
    #[tokio::test]
    async fn test_grid_topology() {
        let topology = NetworkTopology::Grid2D {
            width: 5,
            height: 4,
            connection_radius: 1.5,
        };
        
        let network = NeuralNetwork::with_topology(topology).await;
        assert!(network.is_ok());
        
        let network = network.unwrap();
        assert_eq!(network.neurons.size(), 20); // 5x4 grid
    }
}