// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Shard Map Data Structures
//!
//! Complete mapping of content → shards → matrix positions with replica tracking.

use crate::assets::storage::Hash;
use crate::matrix::MatrixCoordinate;
use serde::{Deserialize, Serialize};

/// Location of a shard in the matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardLocation {
    /// Matrix position where shard is stored
    pub position: MatrixCoordinate,

    /// Distance metric to requester (set during optimization)
    pub distance: f64,

    /// Replica priority (higher = preferred)
    pub priority: u32,

    /// Node health score (0.0 to 1.0)
    pub health_score: f64,

    /// Estimated latency to this location (milliseconds)
    pub estimated_latency_ms: u64,
}

impl ShardLocation {
    /// Create a new shard location with default values
    pub fn new(position: MatrixCoordinate, health_score: f64) -> Self {
        Self {
            position,
            distance: 0.0,
            priority: 100,
            health_score,
            estimated_latency_ms: 0,
        }
    }

    /// Create with full details
    pub fn with_details(
        position: MatrixCoordinate,
        distance: f64,
        priority: u32,
        health_score: f64,
        estimated_latency_ms: u64,
    ) -> Self {
        Self {
            position,
            distance,
            priority,
            health_score,
            estimated_latency_ms,
        }
    }

    /// Calculate distance to a target position
    pub fn distance_to(&self, target: &MatrixCoordinate) -> f64 {
        let dx = (self.position.x - target.x) as f64;
        let dy = (self.position.y - target.y) as f64;
        let dz = (self.position.z - target.z) as f64;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Calculate suitability score (combines distance, priority, health)
    pub fn suitability_score(&self) -> f64 {
        let distance_factor = 1.0 / (1.0 + self.distance);
        let priority_factor = self.priority as f64 / 100.0;
        let health_factor = self.health_score;

        // Weighted combination: 40% distance, 30% priority, 30% health
        0.4 * distance_factor + 0.3 * priority_factor + 0.3 * health_factor
    }

    /// Check if this location is suitable for retrieval
    pub fn is_suitable(&self) -> bool {
        self.health_score >= 0.5 // At least 50% health
    }
}

/// Entry in the shard map for a single shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMapEntry {
    /// Hash of this shard
    pub shard_hash: Hash,

    /// All known locations where this shard is stored
    pub locations: Vec<ShardLocation>,
}

impl ShardMapEntry {
    /// Create a new shard map entry
    pub fn new(shard_hash: Hash, locations: Vec<ShardLocation>) -> Self {
        Self {
            shard_hash,
            locations,
        }
    }

    /// Add a location to this shard
    pub fn add_location(&mut self, location: ShardLocation) {
        self.locations.push(location);
    }

    /// Get best location based on suitability score
    pub fn get_best_location(&self) -> Option<&ShardLocation> {
        self.locations
            .iter()
            .filter(|loc| loc.is_suitable())
            .max_by(|a, b| {
                a.suitability_score()
                    .partial_cmp(&b.suitability_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Get all suitable locations sorted by suitability
    pub fn get_sorted_locations(&self) -> Vec<&ShardLocation> {
        let mut suitable: Vec<&ShardLocation> = self
            .locations
            .iter()
            .filter(|loc| loc.is_suitable())
            .collect();

        suitable.sort_by(|a, b| {
            b.suitability_score()
                .partial_cmp(&a.suitability_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        suitable
    }

    /// Optimize locations for a specific target position
    pub fn optimize_for_target(&mut self, target: &MatrixCoordinate) {
        // Update distance for all locations
        for location in &mut self.locations {
            location.distance = location.distance_to(target);
        }

        // Sort by suitability
        self.locations.sort_by(|a, b| {
            b.suitability_score()
                .partial_cmp(&a.suitability_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Check if shard has any suitable replicas
    pub fn has_suitable_replicas(&self) -> bool {
        self.locations.iter().any(|loc| loc.is_suitable())
    }
}

/// Complete shard map for content retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteShardMap {
    /// All shard entries in order
    pub entries: Vec<ShardMapEntry>,

    /// Total unique positions used
    pub total_positions: usize,

    /// Average replicas per shard
    pub avg_replicas: f64,
}

impl CompleteShardMap {
    /// Create a new empty shard map
    pub fn new() -> Self {
        Self {
            entries: vec![],
            total_positions: 0,
            avg_replicas: 0.0,
        }
    }

    /// Create from shard entries
    pub fn from_entries(entries: Vec<ShardMapEntry>) -> Self {
        let mut map = Self {
            entries,
            total_positions: 0,
            avg_replicas: 0.0,
        };
        map.recalculate_stats();
        map
    }

    /// Add a shard entry
    pub fn add_entry(&mut self, entry: ShardMapEntry) {
        self.entries.push(entry);
        self.recalculate_stats();
    }

    /// Recalculate statistics
    fn recalculate_stats(&mut self) {
        if self.entries.is_empty() {
            self.total_positions = 0;
            self.avg_replicas = 0.0;
            return;
        }

        // Count unique positions
        let mut positions = std::collections::HashSet::new();
        let mut total_replicas = 0;

        for entry in &self.entries {
            total_replicas += entry.locations.len();
            for location in &entry.locations {
                positions.insert(format!(
                    "{},{},{}",
                    location.position.x, location.position.y, location.position.z
                ));
            }
        }

        self.total_positions = positions.len();
        self.avg_replicas = total_replicas as f64 / self.entries.len() as f64;
    }

    /// Optimize entire map for a target position
    pub fn optimize_for_target(&mut self, target: &MatrixCoordinate) {
        for entry in &mut self.entries {
            entry.optimize_for_target(target);
        }
    }

    /// Get shard entry by index
    pub fn get_entry(&self, index: usize) -> Option<&ShardMapEntry> {
        self.entries.get(index)
    }

    /// Get shard entry by hash
    pub fn find_entry(&self, shard_hash: &Hash) -> Option<&ShardMapEntry> {
        self.entries.iter().find(|e| &e.shard_hash == shard_hash)
    }

    /// Check if all shards have sufficient replicas
    pub fn has_sufficient_replicas(&self, min_replicas: usize) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.locations.len() >= min_replicas)
    }

    /// Get shards with insufficient replicas
    pub fn get_weak_shards(&self, min_replicas: usize) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.locations.len() < min_replicas)
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Estimate wire-format size in bytes using compact binary encoding.
    ///
    /// The wire format only transmits essential data: shard hashes and matrix
    /// positions. Derived metrics (health, latency, distance, priority) are
    /// calculated by the client based on its own position and are NOT
    /// transmitted, keeping the instruction payload minimal.
    pub fn estimate_size(&self) -> usize {
        // Per entry: shard_hash (32 bytes) + location count varint (1 byte)
        let entry_overhead = 33;
        // Per location: position x,y,z as 3 varints (avg 4 bytes each = 12)
        let location_size = 12;

        // Base: entry count varint (2 bytes)
        let mut total = 2;

        for entry in &self.entries {
            total += entry_overhead;
            total += entry.locations.len() * location_size;
        }

        total
    }
}

impl Default for CompleteShardMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_location_creation() {
        let pos = MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate");
        let location = ShardLocation::new(pos, 0.95);

        assert_eq!(location.position, pos);
        assert_eq!(location.health_score, 0.95);
        assert!(location.is_suitable());
    }

    #[test]
    fn test_distance_calculation() {
        let pos1 = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let pos2 = MatrixCoordinate::new(3, 4, 0).expect("test: valid coordinate");

        let location = ShardLocation::new(pos1, 1.0);
        let distance = location.distance_to(&pos2);

        assert_eq!(distance, 5.0); // 3-4-5 triangle
    }

    #[test]
    fn test_suitability_score() {
        let pos = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let mut location = ShardLocation::new(pos, 0.9);
        location.distance = 5.0;
        location.priority = 80;

        let score = location.suitability_score();
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn test_shard_map_entry() {
        let shard_hash = [1u8; 32];
        let positions = vec![
            MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(1, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(2, 0, 0).expect("test: valid coordinate"),
        ];

        let locations: Vec<ShardLocation> = positions
            .into_iter()
            .map(|pos| ShardLocation::new(pos, 0.9))
            .collect();

        let entry = ShardMapEntry::new(shard_hash, locations);
        assert_eq!(entry.locations.len(), 3);
        assert!(entry.has_suitable_replicas());
    }

    #[test]
    fn test_optimize_for_target() {
        let shard_hash = [1u8; 32];
        let positions = vec![
            MatrixCoordinate::new(10, 0, 0).expect("test: valid coordinate"),
            MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate"), // Nearest
            MatrixCoordinate::new(5, 0, 0).expect("test: valid coordinate"),
        ];

        let locations: Vec<ShardLocation> = positions
            .into_iter()
            .map(|pos| ShardLocation::new(pos, 1.0))
            .collect();

        let mut entry = ShardMapEntry::new(shard_hash, locations);

        // Optimize for origin
        let target = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        entry.optimize_for_target(&target);

        // First location should be nearest
        let best = entry.get_best_location().expect("test: expected success");
        assert_eq!(best.position.x, 0);
    }

    #[test]
    fn test_complete_shard_map() {
        let mut map = CompleteShardMap::new();

        // Add 3 shards with different replica counts
        for i in 0..3 {
            let shard_hash = [i as u8; 32];
            let locations = vec![
                ShardLocation::new(MatrixCoordinate::new(i as i64, 0, 0).expect("test: valid coordinate"), 1.0),
                ShardLocation::new(MatrixCoordinate::new(i as i64, 1, 0).expect("test: valid coordinate"), 1.0),
            ];
            let entry = ShardMapEntry::new(shard_hash, locations);
            map.add_entry(entry);
        }

        assert_eq!(map.entries.len(), 3);
        assert_eq!(map.avg_replicas, 2.0);
        assert!(map.has_sufficient_replicas(2));
        assert!(!map.has_sufficient_replicas(3));
    }

    #[test]
    fn test_estimate_size() {
        let mut map = CompleteShardMap::new();

        // Create typical Reed-Solomon 10+4 configuration
        for i in 0..14 {
            let shard_hash = [i as u8; 32];
            let locations = vec![
                ShardLocation::new(MatrixCoordinate::new(i as i64, 0, 0).expect("test: valid coordinate"), 1.0),
                ShardLocation::new(MatrixCoordinate::new(i as i64, 1, 0).expect("test: valid coordinate"), 0.9),
                ShardLocation::new(MatrixCoordinate::new(i as i64, 2, 0).expect("test: valid coordinate"), 0.85),
            ];
            let entry = ShardMapEntry::new(shard_hash, locations);
            map.add_entry(entry);
        }

        let size = map.estimate_size();
        println!("Shard map estimated size: {size} bytes");

        // Should be reasonable for 14 shards × 3 replicas
        assert!(size < 10000); // Less than 10KB
    }
}
