// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Matrix state serialization
//!
//! Handles serialization of matrix coordinates, neighbor lists, and distance caches
//! in multiple formats with compression and versioning support.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::matrix::coordinate::MatrixCoordinate;
use super::{PersistenceError, PersistenceResult};

/// Current format version for backward compatibility
const CURRENT_VERSION: u32 = 1;

/// Serialization format options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SerializationFormat {
    /// Binary format using bincode (fastest)
    Bincode,
    /// JSON format (human-readable)
    Json,
    /// MessagePack format (compact)
    MessagePack,
}

/// Complete matrix state for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixState {
    /// Format version
    pub version: u32,
    /// Node's primary coordinate
    pub coordinate: MatrixCoordinate,
    /// Neighbor node IDs and their coordinates
    pub neighbors: HashMap<String, MatrixCoordinate>,
    /// Cached distance calculations
    pub distance_cache: HashMap<(String, String), f64>,
    /// Timestamp of state snapshot
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Node metadata
    pub metadata: HashMap<String, String>,
}

impl MatrixState {
    /// Create a new matrix state
    pub fn new(coordinate: MatrixCoordinate) -> Self {
        Self {
            version: CURRENT_VERSION,
            coordinate,
            neighbors: HashMap::new(),
            distance_cache: HashMap::new(),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add a neighbor
    pub fn add_neighbor(&mut self, node_id: String, coordinate: MatrixCoordinate) {
        self.neighbors.insert(node_id, coordinate);
    }

    /// Cache a distance calculation
    pub fn cache_distance(&mut self, from: String, to: String, distance: f64) {
        self.distance_cache.insert((from, to), distance);
    }

    /// Get size estimate in bytes
    pub fn size_estimate(&self) -> usize {
        // Rough estimate: coordinate (24) + each neighbor (24 + id_len) + cache entries (16 + key_lens)
        let base = 24 + 8; // coordinate + version + timestamp
        let neighbors_size: usize = self.neighbors.iter()
            .map(|(id, _)| 24 + id.len())
            .sum();
        let cache_size: usize = self.distance_cache.iter()
            .map(|((from, to), _)| 16 + from.len() + to.len())
            .sum();
        let metadata_size: usize = self.metadata.iter()
            .map(|(k, v)| k.len() + v.len())
            .sum();

        base + neighbors_size + cache_size + metadata_size
    }
}

/// Handles matrix state serialization and deserialization
pub struct MatrixStateSerializer {
    format: SerializationFormat,
    compress: bool,
    compression_level: i32,
}

impl MatrixStateSerializer {
    /// Create a new serializer
    pub fn new(format: SerializationFormat, compress: bool) -> Self {
        Self {
            format,
            compress,
            compression_level: 3, // Default zstd compression level
        }
    }

    /// Set compression level (1-22 for zstd)
    pub fn with_compression_level(mut self, level: i32) -> Self {
        self.compression_level = level.clamp(1, 22);
        self
    }

    /// Serialize matrix state to bytes
    pub fn serialize(&self, state: &MatrixState) -> PersistenceResult<Vec<u8>> {
        debug!("Serializing matrix state with format {:?}", self.format);

        // Serialize based on format
        let serialized = match self.format {
            SerializationFormat::Bincode => {
                bincode::serialize(state)
                    .map_err(|e| PersistenceError::Serialization(e.to_string()))?
            }
            SerializationFormat::Json => {
                serde_json::to_vec(state)
                    .map_err(|e| PersistenceError::Serialization(e.to_string()))?
            }
            SerializationFormat::MessagePack => {
                rmp_serde::to_vec(state)
                    .map_err(|e| PersistenceError::Serialization(e.to_string()))?
            }
        };

        // Apply compression if enabled
        if self.compress {
            self.compress_data(&serialized)
        } else {
            Ok(serialized)
        }
    }

    /// Deserialize matrix state from bytes
    pub fn deserialize(&self, data: &[u8]) -> PersistenceResult<MatrixState> {
        debug!("Deserializing matrix state");

        // Decompress if needed
        let decompressed = if self.compress {
            self.decompress_data(data)?
        } else {
            data.to_vec()
        };

        // Deserialize based on format
        let state: MatrixState = match self.format {
            SerializationFormat::Bincode => {
                bincode::deserialize(&decompressed)
                    .map_err(|e| PersistenceError::Deserialization(e.to_string()))?
            }
            SerializationFormat::Json => {
                serde_json::from_slice(&decompressed)
                    .map_err(|e| PersistenceError::Deserialization(e.to_string()))?
            }
            SerializationFormat::MessagePack => {
                rmp_serde::from_slice(&decompressed)
                    .map_err(|e| PersistenceError::Deserialization(e.to_string()))?
            }
        };

        // Validate version
        if state.version > CURRENT_VERSION {
            return Err(PersistenceError::VersionMismatch {
                expected: CURRENT_VERSION,
                actual: state.version,
            });
        }

        Ok(state)
    }

    /// Serialize only changed data (incremental)
    pub fn serialize_incremental(
        &self,
        current: &MatrixState,
        previous: &MatrixState,
    ) -> PersistenceResult<Vec<u8>> {
        // Create a delta state with only changes
        let mut delta = MatrixState::new(current.coordinate.clone());
        delta.timestamp = current.timestamp;

        // Find new/changed neighbors
        for (id, coord) in &current.neighbors {
            if previous.neighbors.get(id) != Some(coord) {
                delta.neighbors.insert(id.clone(), coord.clone());
            }
        }

        // Find new distance cache entries
        for (key, value) in &current.distance_cache {
            if previous.distance_cache.get(key) != Some(value) {
                delta.distance_cache.insert(key.clone(), *value);
            }
        }

        // Find changed metadata
        for (key, value) in &current.metadata {
            if previous.metadata.get(key) != Some(value) {
                delta.metadata.insert(key.clone(), value.clone());
            }
        }

        info!("Incremental update: {} neighbors, {} distances, {} metadata",
              delta.neighbors.len(), delta.distance_cache.len(), delta.metadata.len());

        self.serialize(&delta)
    }

    /// Compress data using zstd
    fn compress_data(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        zstd::encode_all(data, self.compression_level)
            .map_err(|e| PersistenceError::Compression(e.to_string()))
    }

    /// Decompress data using zstd
    fn decompress_data(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        zstd::decode_all(data)
            .map_err(|e| PersistenceError::Decompression(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_state_creation() {
        let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
        let state = MatrixState::new(coord.clone());

        assert_eq!(state.version, CURRENT_VERSION);
        assert_eq!(state.coordinate, coord);
        assert!(state.neighbors.is_empty());
        assert!(state.distance_cache.is_empty());
    }

    #[test]
    fn test_add_neighbor() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let mut state = MatrixState::new(coord);

        let neighbor_coord = MatrixCoordinate::new(1, 1, 1).unwrap();
        state.add_neighbor("node1".to_string(), neighbor_coord.clone());

        assert_eq!(state.neighbors.len(), 1);
        assert_eq!(state.neighbors.get("node1"), Some(&neighbor_coord));
    }

    #[test]
    fn test_cache_distance() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let mut state = MatrixState::new(coord);

        state.cache_distance("node1".to_string(), "node2".to_string(), 10.5);

        assert_eq!(state.distance_cache.len(), 1);
        assert_eq!(
            state.distance_cache.get(&("node1".to_string(), "node2".to_string())),
            Some(&10.5)
        );
    }

    #[test]
    fn test_bincode_serialization() {
        let coord = MatrixCoordinate::new(5, 10, 15).unwrap();
        let mut state = MatrixState::new(coord);
        state.add_neighbor("node1".to_string(), MatrixCoordinate::new(1, 2, 3).unwrap());
        state.cache_distance("a".to_string(), "b".to_string(), 42.0);

        let serializer = MatrixStateSerializer::new(SerializationFormat::Bincode, false);
        let serialized = serializer.serialize(&state).unwrap();
        let deserialized = serializer.deserialize(&serialized).unwrap();

        assert_eq!(deserialized.coordinate, state.coordinate);
        assert_eq!(deserialized.neighbors.len(), state.neighbors.len());
        assert_eq!(deserialized.distance_cache.len(), state.distance_cache.len());
    }

    #[test]
    fn test_json_serialization() {
        let coord = MatrixCoordinate::new(5, 10, 15).unwrap();
        let state = MatrixState::new(coord);

        let serializer = MatrixStateSerializer::new(SerializationFormat::Json, false);
        let serialized = serializer.serialize(&state).unwrap();
        let deserialized = serializer.deserialize(&serialized).unwrap();

        assert_eq!(deserialized.coordinate, state.coordinate);
        assert_eq!(deserialized.version, state.version);
    }

    #[test]
    fn test_messagepack_serialization() {
        let coord = MatrixCoordinate::new(5, 10, 15).unwrap();
        let state = MatrixState::new(coord);

        let serializer = MatrixStateSerializer::new(SerializationFormat::MessagePack, false);
        let serialized = serializer.serialize(&state).unwrap();
        let deserialized = serializer.deserialize(&serialized).unwrap();

        assert_eq!(deserialized.coordinate, state.coordinate);
    }

    #[test]
    fn test_compression() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let mut state = MatrixState::new(coord);

        // Add lots of data to make compression worthwhile
        for i in 0..100 {
            let neighbor = MatrixCoordinate::new(i, i * 2, i * 3).unwrap();
            state.add_neighbor(format!("node{}", i), neighbor);
        }

        let uncompressed = MatrixStateSerializer::new(SerializationFormat::Bincode, false);
        let compressed = MatrixStateSerializer::new(SerializationFormat::Bincode, true);

        let uncompressed_data = uncompressed.serialize(&state).unwrap();
        let compressed_data = compressed.serialize(&state).unwrap();

        // Compression should reduce size
        assert!(compressed_data.len() < uncompressed_data.len());

        // Should still deserialize correctly
        let deserialized = compressed.deserialize(&compressed_data).unwrap();
        assert_eq!(deserialized.neighbors.len(), 100);
    }

    #[test]
    fn test_incremental_serialization() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let mut previous = MatrixState::new(coord.clone());
        previous.add_neighbor("node1".to_string(), MatrixCoordinate::new(1, 1, 1).unwrap());

        let mut current = previous.clone();
        current.add_neighbor("node2".to_string(), MatrixCoordinate::new(2, 2, 2).unwrap());
        current.cache_distance("a".to_string(), "b".to_string(), 10.0);

        let serializer = MatrixStateSerializer::new(SerializationFormat::Bincode, false);
        let delta = serializer.serialize_incremental(&current, &previous).unwrap();

        // Delta should be smaller than full serialization
        let full = serializer.serialize(&current).unwrap();
        assert!(delta.len() < full.len());
    }

    #[test]
    fn test_size_estimate() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let mut state = MatrixState::new(coord);

        let initial_size = state.size_estimate();

        state.add_neighbor("node1".to_string(), MatrixCoordinate::new(1, 1, 1).unwrap());
        let after_neighbor = state.size_estimate();
        assert!(after_neighbor > initial_size);

        state.cache_distance("a".to_string(), "b".to_string(), 10.0);
        let after_cache = state.size_estimate();
        assert!(after_cache > after_neighbor);
    }

    #[test]
    fn test_version_validation() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let mut state = MatrixState::new(coord);
        state.version = CURRENT_VERSION + 1; // Future version

        let serializer = MatrixStateSerializer::new(SerializationFormat::Json, false);
        let serialized = serializer.serialize(&state).unwrap();

        let result = serializer.deserialize(&serialized);
        assert!(result.is_err());

        if let Err(PersistenceError::VersionMismatch { expected, actual }) = result {
            assert_eq!(expected, CURRENT_VERSION);
            assert_eq!(actual, CURRENT_VERSION + 1);
        } else {
            panic!("Expected version mismatch error");
        }
    }
}