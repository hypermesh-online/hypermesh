//! Competitive Dynamics and Winner-Take-All Mechanisms
//!
//! Implements lateral inhibition and competition between neurons for pattern recognition
//! and feature selection in the Dynamic Similarity Reservoir.

use anyhow::Result;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, trace};

/// Competition result from winner-take-all dynamics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionResult {
    pub winner_id: usize,
    pub winning_similarity: f64,
    pub confidence: f64,
    pub suppressed_neurons: Vec<usize>,
    pub competition_strength: f64,
}

/// Winner-take-all competition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinnerTakeAll {
    /// Competition radius - how far competition extends
    pub competition_radius: f64,
    /// Inhibition strength - how strong the suppression is
    pub inhibition_strength: f64,
    /// Minimum threshold for competition participation
    pub competition_threshold: f64,
    /// Learning rate for competitive adaptation
    pub adaptation_rate: f64,
}

impl Default for WinnerTakeAll {
    fn default() -> Self {
        Self {
            competition_radius: 0.1,
            inhibition_strength: 0.5,
            competition_threshold: 0.1,
            adaptation_rate: 0.01,
        }
    }
}

/// Competitive dynamics engine
pub struct CompetitiveDynamics {
    wta: WinnerTakeAll,
    /// Neuron position in feature space for spatial competition
    neuron_positions: HashMap<usize, Vec<f64>>,
    /// Historical competition results for learning
    competition_history: Vec<CompetitionResult>,
    /// Adaptation matrix for competitive learning
    competition_weights: HashMap<(usize, usize), f64>,
    /// Inhibitory connections between competing neurons
    lateral_inhibition: HashMap<usize, Vec<(usize, f64)>>,
}

impl CompetitiveDynamics {
    pub fn new(inhibition_strength: f64, competition_radius: f64) -> Result<Self> {
        let wta = WinnerTakeAll {
            inhibition_strength,
            competition_radius,
            ..Default::default()
        };
        
        Ok(Self {
            wta,
            neuron_positions: HashMap::new(),
            competition_history: Vec::with_capacity(10000),
            competition_weights: HashMap::new(),
            lateral_inhibition: HashMap::new(),
        })
    }
    
    /// Apply competitive dynamics to network output
    pub async fn apply_competition(&mut self, network_output: &[f64]) -> Result<CompetitionResult> {
        // Find neurons above competition threshold
        let candidates: Vec<(usize, f64)> = network_output.iter()
            .enumerate()
            .filter(|(_, &activity)| activity >= self.wta.competition_threshold)
            .map(|(id, &activity)| (id, activity))
            .collect();
        
        if candidates.is_empty() {
            return Ok(CompetitionResult {
                winner_id: 0,
                winning_similarity: 0.0,
                confidence: 0.0,
                suppressed_neurons: Vec::new(),
                competition_strength: 0.0,
            });
        }
        
        // Apply lateral inhibition
        let inhibited_activities = self.apply_lateral_inhibition(&candidates).await?;
        
        // Find winner after inhibition
        let (winner_id, winning_activity) = inhibited_activities.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .cloned()
            .unwrap_or((0, 0.0));
        
        // Calculate competition confidence
        let confidence = self.calculate_confidence(&inhibited_activities, winner_id);
        
        // Identify suppressed neurons
        let suppressed_neurons = self.find_suppressed_neurons(&candidates, &inhibited_activities);
        
        // Calculate competition strength
        let competition_strength = self.calculate_competition_strength(&candidates);
        
        let result = CompetitionResult {
            winner_id,
            winning_similarity: winning_activity,
            confidence,
            suppressed_neurons,
            competition_strength,
        };
        
        // Update competition history
        self.competition_history.push(result.clone());
        if self.competition_history.len() > 10000 {
            self.competition_history.remove(0);
        }
        
        // Adapt competition weights
        self.adapt_competition_weights(&result).await?;
        
        debug!("Competition result: winner={}, similarity={:.3}, confidence={:.3}", 
               winner_id, winning_activity, confidence);
        
        Ok(result)
    }
    
    /// Apply lateral inhibition between competing neurons
    async fn apply_lateral_inhibition(&mut self, 
        candidates: &[(usize, f64)]
    ) -> Result<Vec<(usize, f64)>> {
        let mut inhibited = Vec::new();
        
        for &(neuron_id, activity) in candidates {
            let mut total_inhibition = 0.0;
            
            // Calculate inhibition from other active neurons
            for &(other_id, other_activity) in candidates {
                if neuron_id != other_id {
                    let inhibition = self.calculate_inhibition(neuron_id, other_id, other_activity).await;
                    total_inhibition += inhibition;
                }
            }
            
            // Apply inhibition (but don't let activity go negative)
            let inhibited_activity = (activity - total_inhibition).max(0.0);
            inhibited.push((neuron_id, inhibited_activity));
        }
        
        Ok(inhibited)
    }
    
    /// Calculate inhibition strength between two neurons
    async fn calculate_inhibition(&mut self, 
        target_neuron: usize, 
        source_neuron: usize, 
        source_activity: f64
    ) -> f64 {
        // Get or create neuron positions in feature space
        let distance = self.get_neuron_distance(target_neuron, source_neuron);
        
        // Distance-based inhibition (closer = stronger inhibition)
        let distance_factor = if distance <= self.wta.competition_radius {
            1.0 - (distance / self.wta.competition_radius)
        } else {
            0.0
        };
        
        // Get learned inhibition weight
        let learned_weight = self.competition_weights
            .get(&(source_neuron, target_neuron))
            .cloned()
            .unwrap_or(1.0);
        
        // Calculate total inhibition
        let inhibition = source_activity * distance_factor * learned_weight * self.wta.inhibition_strength;
        
        trace!("Inhibition from {} to {}: {:.3} (distance={:.3}, weight={:.3})",
               source_neuron, target_neuron, inhibition, distance, learned_weight);
        
        inhibition
    }
    
    /// Get or compute distance between neurons in feature space
    fn get_neuron_distance(&mut self, neuron1: usize, neuron2: usize) -> f64 {
        // Initialize positions if not present
        if !self.neuron_positions.contains_key(&neuron1) {
            self.initialize_neuron_position(neuron1);
        }
        if !self.neuron_positions.contains_key(&neuron2) {
            self.initialize_neuron_position(neuron2);
        }
        
        let pos1 = self.neuron_positions.get(&neuron1).unwrap();
        let pos2 = self.neuron_positions.get(&neuron2).unwrap();
        
        // Euclidean distance in feature space
        pos1.iter().zip(pos2.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
    }
    
    /// Initialize random position for neuron in feature space
    fn initialize_neuron_position(&mut self, neuron_id: usize) {
        let mut rng = thread_rng();
        let dimensions = 10; // 10D feature space
        
        let position: Vec<f64> = (0..dimensions)
            .map(|_| rng.gen::<f64>() * 2.0 - 1.0) // [-1, 1] range
            .collect();
        
        self.neuron_positions.insert(neuron_id, position);
    }
    
    /// Calculate confidence in competition result
    fn calculate_confidence(&self, activities: &[(usize, f64)], winner_id: usize) -> f64 {
        if activities.len() < 2 {
            return 1.0;
        }
        
        let mut sorted_activities: Vec<f64> = activities.iter()
            .map(|(_, activity)| *activity)
            .collect();
        sorted_activities.sort_by(|a, b| b.partial_cmp(a).unwrap());
        
        let winner_activity = activities.iter()
            .find(|(id, _)| *id == winner_id)
            .map(|(_, activity)| *activity)
            .unwrap_or(0.0);
        
        if sorted_activities.len() >= 2 {
            let second_best = sorted_activities[1];
            let margin = winner_activity - second_best;
            let max_possible = winner_activity + second_best;
            
            if max_possible > 0.0 {
                (margin / max_possible).max(0.0).min(1.0)
            } else {
                0.5
            }
        } else {
            1.0
        }
    }
    
    /// Find neurons that were suppressed by competition
    fn find_suppressed_neurons(&self, 
        original: &[(usize, f64)], 
        inhibited: &[(usize, f64)]
    ) -> Vec<usize> {
        let original_map: HashMap<usize, f64> = original.iter().cloned().collect();
        let inhibited_map: HashMap<usize, f64> = inhibited.iter().cloned().collect();
        
        let mut suppressed = Vec::new();
        
        for (&neuron_id, &original_activity) in &original_map {
            let inhibited_activity = inhibited_map.get(&neuron_id).cloned().unwrap_or(0.0);
            
            // Significant suppression (>50% reduction)
            if original_activity > 0.0 && 
               (original_activity - inhibited_activity) / original_activity > 0.5 {
                suppressed.push(neuron_id);
            }
        }
        
        suppressed
    }
    
    /// Calculate overall competition strength
    fn calculate_competition_strength(&self, candidates: &[(usize, f64)]) -> f64 {
        if candidates.len() < 2 {
            return 0.0;
        }
        
        let total_activity: f64 = candidates.iter().map(|(_, activity)| activity).sum();
        let mean_activity = total_activity / candidates.len() as f64;
        
        // Variance in activities (higher variance = stronger competition)
        let variance: f64 = candidates.iter()
            .map(|(_, activity)| (activity - mean_activity).powi(2))
            .sum::<f64>() / candidates.len() as f64;
        
        variance.sqrt() / (mean_activity + 1e-6) // Coefficient of variation
    }
    
    /// Adapt competition weights based on results
    async fn adapt_competition_weights(&mut self, result: &CompetitionResult) -> Result<()> {
        let learning_rate = self.wta.adaptation_rate;
        let winner_id = result.winner_id;
        
        // Strengthen inhibition from winner to suppressed neurons
        for &suppressed_id in &result.suppressed_neurons {
            let key = (winner_id, suppressed_id);
            let current_weight = self.competition_weights.get(&key).cloned().unwrap_or(1.0);
            let new_weight = current_weight + learning_rate * result.confidence;
            self.competition_weights.insert(key, new_weight.min(2.0)); // Cap at 2.0
        }
        
        // Weaken inhibition from suppressed neurons to winner
        for &suppressed_id in &result.suppressed_neurons {
            let key = (suppressed_id, winner_id);
            let current_weight = self.competition_weights.get(&key).cloned().unwrap_or(1.0);
            let new_weight = current_weight - learning_rate * 0.5;
            self.competition_weights.insert(key, new_weight.max(0.1)); // Minimum 0.1
        }
        
        trace!("Adapted {} competition weights", result.suppressed_neurons.len() * 2);
        Ok(())
    }
    
    /// Update neuron positions based on input patterns (competitive learning)
    pub async fn update_neuron_positions(&mut self, 
        input_pattern: &[f64], 
        winner_id: usize
    ) -> Result<()> {
        if !self.neuron_positions.contains_key(&winner_id) {
            self.initialize_neuron_position(winner_id);
        }
        
        let learning_rate = self.wta.adaptation_rate;
        let position = self.neuron_positions.get_mut(&winner_id).unwrap();
        
        // Move winner position toward input pattern
        for (i, &input_val) in input_pattern.iter().enumerate() {
            if i < position.len() {
                position[i] += learning_rate * (input_val - position[i]);
            }
        }
        
        // Normalize position to unit length
        let length: f64 = position.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        if length > 0.0 {
            for pos in position.iter_mut() {
                *pos /= length;
            }
        }
        
        Ok(())
    }
    
    /// Get competition statistics
    pub fn get_competition_stats(&self) -> CompetitionStats {
        if self.competition_history.is_empty() {
            return CompetitionStats::default();
        }
        
        let recent_count = 100.min(self.competition_history.len());
        let recent_results = &self.competition_history[self.competition_history.len() - recent_count..];
        
        let avg_confidence = recent_results.iter()
            .map(|r| r.confidence)
            .sum::<f64>() / recent_count as f64;
        
        let avg_suppression = recent_results.iter()
            .map(|r| r.suppressed_neurons.len())
            .sum::<usize>() as f64 / recent_count as f64;
        
        let avg_competition_strength = recent_results.iter()
            .map(|r| r.competition_strength)
            .sum::<f64>() / recent_count as f64;
        
        // Winner diversity (entropy of winner distribution)
        let mut winner_counts: HashMap<usize, usize> = HashMap::new();
        for result in recent_results {
            *winner_counts.entry(result.winner_id).or_insert(0) += 1;
        }
        
        let winner_entropy = if winner_counts.len() > 1 {
            winner_counts.values()
                .map(|&count| {
                    let p = count as f64 / recent_count as f64;
                    -p * p.log2()
                })
                .sum()
        } else {
            0.0
        };
        
        CompetitionStats {
            total_competitions: self.competition_history.len(),
            average_confidence: avg_confidence,
            average_suppression: avg_suppression,
            average_competition_strength: avg_competition_strength,
            winner_diversity: winner_entropy,
            unique_winners: winner_counts.len(),
            learned_weights: self.competition_weights.len(),
        }
    }
    
    /// Reset competition history and weights
    pub fn reset_competition(&mut self) {
        self.competition_history.clear();
        self.competition_weights.clear();
        self.neuron_positions.clear();
        self.lateral_inhibition.clear();
    }
    
    /// Set competition parameters
    pub fn set_parameters(&mut self, 
        inhibition_strength: f64, 
        competition_radius: f64,
        adaptation_rate: f64
    ) {
        self.wta.inhibition_strength = inhibition_strength;
        self.wta.competition_radius = competition_radius;
        self.wta.adaptation_rate = adaptation_rate;
    }
    
    /// Get current competition parameters
    pub fn get_parameters(&self) -> &WinnerTakeAll {
        &self.wta
    }
    
    /// Create structured lateral inhibition connections
    pub fn create_lateral_inhibition_topology(&mut self, neuron_count: usize, topology: InhibitionTopology) {
        self.lateral_inhibition.clear();
        
        match topology {
            InhibitionTopology::Global => {
                // Every neuron inhibits every other neuron
                for i in 0..neuron_count {
                    let mut connections = Vec::new();
                    for j in 0..neuron_count {
                        if i != j {
                            connections.push((j, self.wta.inhibition_strength));
                        }
                    }
                    self.lateral_inhibition.insert(i, connections);
                }
            },
            InhibitionTopology::LocalRadius { radius } => {
                // Local inhibition within radius
                for i in 0..neuron_count {
                    let mut connections = Vec::new();
                    for j in 0..neuron_count {
                        if i != j {
                            let distance = self.get_neuron_distance(i, j);
                            if distance <= radius {
                                let strength = self.wta.inhibition_strength * (1.0 - distance / radius);
                                connections.push((j, strength));
                            }
                        }
                    }
                    self.lateral_inhibition.insert(i, connections);
                }
            },
            InhibitionTopology::Random { probability } => {
                let mut rng = thread_rng();
                for i in 0..neuron_count {
                    let mut connections = Vec::new();
                    for j in 0..neuron_count {
                        if i != j && rng.gen::<f64>() < probability {
                            let strength = self.wta.inhibition_strength * (0.5 + rng.gen::<f64>() * 0.5);
                            connections.push((j, strength));
                        }
                    }
                    self.lateral_inhibition.insert(i, connections);
                }
            },
        }
        
        debug!("Created lateral inhibition topology: {:?}", topology);
    }
}

/// Competition statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionStats {
    pub total_competitions: usize,
    pub average_confidence: f64,
    pub average_suppression: f64,
    pub average_competition_strength: f64,
    pub winner_diversity: f64,
    pub unique_winners: usize,
    pub learned_weights: usize,
}

impl Default for CompetitionStats {
    fn default() -> Self {
        Self {
            total_competitions: 0,
            average_confidence: 0.0,
            average_suppression: 0.0,
            average_competition_strength: 0.0,
            winner_diversity: 0.0,
            unique_winners: 0,
            learned_weights: 0,
        }
    }
}

/// Lateral inhibition topology patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InhibitionTopology {
    /// All neurons inhibit all other neurons
    Global,
    /// Local inhibition within specified radius
    LocalRadius { radius: f64 },
    /// Random inhibition connections with probability
    Random { probability: f64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_competition_creation() {
        let competition = CompetitiveDynamics::new(0.5, 0.1);
        assert!(competition.is_ok());
    }
    
    #[tokio::test]
    async fn test_winner_take_all() {
        let mut competition = CompetitiveDynamics::new(0.8, 0.2).unwrap();
        
        // Strong winner with weaker competitors
        let activities = vec![0.1, 0.9, 0.3, 0.2, 0.4];
        let result = competition.apply_competition(&activities).await;
        
        assert!(result.is_ok());
        let comp_result = result.unwrap();
        
        // Winner should be neuron 1 (highest activity)
        assert_eq!(comp_result.winner_id, 1);
        assert!(comp_result.confidence > 0.0);
        assert!(comp_result.winning_similarity > 0.0);
    }
    
    #[tokio::test]
    async fn test_lateral_inhibition() {
        let mut competition = CompetitiveDynamics::new(1.0, 0.5).unwrap();
        
        let candidates = vec![(0, 0.8), (1, 0.7), (2, 0.6)];
        let inhibited = competition.apply_lateral_inhibition(&candidates).await;
        
        assert!(inhibited.is_ok());
        let result = inhibited.unwrap();
        
        // All should have some activity (may be reduced)
        assert_eq!(result.len(), 3);
        for (_, activity) in result {
            assert!(activity >= 0.0);
        }
    }
    
    #[tokio::test]
    async fn test_neuron_position_update() {
        let mut competition = CompetitiveDynamics::new(0.5, 0.1).unwrap();
        
        let input_pattern = vec![0.5, 0.3, 0.8, 0.1];
        let winner_id = 5;
        
        let result = competition.update_neuron_positions(&input_pattern, winner_id).await;
        assert!(result.is_ok());
        
        // Position should be initialized and updated
        assert!(competition.neuron_positions.contains_key(&winner_id));
        let position = &competition.neuron_positions[&winner_id];
        
        // Position should be normalized (unit length)
        let length: f64 = position.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        assert!((length - 1.0).abs() < 1e-6);
    }
    
    #[test]
    fn test_competition_confidence() {
        let competition = CompetitiveDynamics::new(0.5, 0.1).unwrap();
        
        // Clear winner case
        let activities1 = vec![(0, 0.9), (1, 0.1), (2, 0.1)];
        let confidence1 = competition.calculate_confidence(&activities1, 0);
        
        // Close competition case  
        let activities2 = vec![(0, 0.5), (1, 0.48), (2, 0.1)];
        let confidence2 = competition.calculate_confidence(&activities2, 0);
        
        // Clear winner should have higher confidence
        assert!(confidence1 > confidence2);
        assert!(confidence1 > 0.5);
        assert!(confidence2 < 0.5);
    }
    
    #[test]
    fn test_lateral_inhibition_topology() {
        let mut competition = CompetitiveDynamics::new(0.5, 0.1).unwrap();
        
        // Test global inhibition
        competition.create_lateral_inhibition_topology(5, InhibitionTopology::Global);
        assert_eq!(competition.lateral_inhibition.len(), 5);
        
        // Each neuron should inhibit 4 others
        for connections in competition.lateral_inhibition.values() {
            assert_eq!(connections.len(), 4);
        }
        
        // Test random inhibition
        competition.create_lateral_inhibition_topology(10, InhibitionTopology::Random { probability: 0.3 });
        assert_eq!(competition.lateral_inhibition.len(), 10);
    }
    
    #[tokio::test]
    async fn test_competition_adaptation() {
        let mut competition = CompetitiveDynamics::new(0.5, 0.1).unwrap();
        
        let activities = vec![0.8, 0.3, 0.2, 0.1];
        let result1 = competition.apply_competition(&activities).await.unwrap();
        
        // Apply same pattern multiple times
        for _ in 0..10 {
            let _ = competition.apply_competition(&activities).await;
        }
        
        let stats = competition.get_competition_stats();
        assert!(stats.total_competitions > 10);
        assert!(stats.learned_weights > 0);
    }
}