// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Streaming Shard Reconstruction (R13)
//!
//! Reconstruct assets incrementally as shards arrive rather than
//! buffering all k shards upfront. For large assets split across
//! multiple Reed-Solomon groups, each group is reconstructed and
//! flushed independently to bound memory usage.
//!
//! # Architecture
//!
//! A `StreamingReconstructor` accepts shards via [`add_shard`]. When
//! a group has accumulated its k data shards, it is reconstructed
//! immediately and the completed bytes are moved to the output buffer.
//! This lets the caller consume completed groups while later groups
//! are still arriving.

use crate::assets::pipeline::sharding::{Shard, ShardingConfig, Sharder};
use crate::assets::pipeline::PipelineError;
use std::collections::HashMap;

/// Progress state returned by [`StreamingReconstructor::add_shard`].
#[derive(Debug, Clone, PartialEq)]
pub enum ReconstructionState {
    /// More shards are needed. `progress` is 0.0-1.0 fraction of
    /// minimum required shards received so far.
    Partial { progress: f64 },
    /// All groups have been reconstructed.
    Complete { data: Vec<u8> },
}

/// Tracks shards for a single RS group and reconstructs when ready.
struct GroupBuffer {
    /// RS configuration for this group.
    config: ShardingConfig,
    /// Shards received so far, keyed by shard index within the group.
    shards: HashMap<usize, Shard>,
    /// Whether this group has been reconstructed already.
    reconstructed: bool,
    /// Reconstructed bytes (populated once k shards arrive).
    output: Option<Vec<u8>>,
}

impl GroupBuffer {
    fn new(config: ShardingConfig) -> Self {
        Self {
            config,
            shards: HashMap::new(),
            reconstructed: false,
            output: None,
        }
    }

    /// Add a shard to this group buffer. Returns true if the group
    /// just became reconstructable (has k shards).
    fn add(&mut self, local_index: usize, shard: Shard) -> bool {
        if self.reconstructed {
            return false;
        }
        self.shards.insert(local_index, shard);
        self.shards.len() >= self.config.data_shards && !self.reconstructed
    }

    /// Reconstruct this group from buffered shards.
    fn reconstruct(&mut self) -> Result<Vec<u8>, PipelineError> {
        if self.reconstructed {
            return self.output.clone().ok_or_else(|| {
                PipelineError::ShardingFailed("group already consumed".to_string())
            });
        }

        let sharder = Sharder::new(self.config.clone())?;
        let shard_vec: Vec<Shard> = self.shards.values().cloned().collect();
        let data = sharder.reconstruct(&shard_vec)?;

        self.reconstructed = true;
        self.output = Some(data.clone());
        // Free shard buffers since we have the output now.
        self.shards.clear();
        Ok(data)
    }
}

/// Streaming reconstructor that processes shards incrementally.
///
/// For single-group assets (most common case with RS 10+4), this
/// buffers k shards then reconstructs. For multi-group assets it
/// reconstructs each group independently and concatenates results.
pub struct StreamingReconstructor {
    /// Number of RS groups the asset is split into.
    group_count: usize,
    /// Per-group RS configuration.
    group_config: ShardingConfig,
    /// Per-group shard buffers.
    groups: Vec<GroupBuffer>,
    /// Total shards expected across all groups.
    total_shards: usize,
    /// Total shards received so far.
    received_count: usize,
}

impl StreamingReconstructor {
    /// Create a new streaming reconstructor.
    ///
    /// # Arguments
    /// * `group_count` - Number of RS groups (1 for typical single-group assets)
    /// * `group_config` - RS parameters for each group
    pub fn new(group_count: usize, group_config: ShardingConfig) -> Self {
        let total_shards = group_count * group_config.total_shards();
        let groups = (0..group_count)
            .map(|_| GroupBuffer::new(group_config.clone()))
            .collect();

        Self {
            group_count,
            group_config,
            groups,
            total_shards,
            received_count: 0,
        }
    }

    /// Create a reconstructor for a single RS group (most common case).
    pub fn single_group(config: ShardingConfig) -> Self {
        Self::new(1, config)
    }

    /// Add a shard and check reconstruction progress.
    ///
    /// The `Shard` must carry correct `ShardMetadata` (especially
    /// `index`, `is_parity`, and `original_size` on the last data
    /// shard) for Reed-Solomon reconstruction to strip padding.
    ///
    /// For multi-group assets, `group_index` selects which RS group
    /// this shard belongs to. For single-group assets, pass 0.
    pub fn add_shard(
        &mut self,
        group_index: usize,
        shard: Shard,
    ) -> Result<ReconstructionState, PipelineError> {
        if group_index >= self.group_count {
            return Err(PipelineError::ShardingFailed(format!(
                "group index {group_index} exceeds group count {}",
                self.group_count
            )));
        }

        let local_idx = shard.metadata.index;
        self.received_count += 1;

        let group = &mut self.groups[group_index];
        let ready = group.add(local_idx, shard);

        // If this group just became ready, reconstruct it eagerly.
        if ready {
            group.reconstruct()?;
        }

        // Check if ALL groups are reconstructed.
        let all_done = self.groups.iter().all(|g| g.reconstructed);
        if all_done {
            let mut combined = Vec::new();
            for g in &self.groups {
                if let Some(ref out) = g.output {
                    combined.extend_from_slice(out);
                }
            }
            return Ok(ReconstructionState::Complete { data: combined });
        }

        // Calculate progress as fraction of minimum required shards received.
        let min_required = self.group_count * self.group_config.data_shards;
        let progress = if min_required > 0 {
            (self.received_count as f64 / min_required as f64).min(1.0)
        } else {
            1.0
        };

        Ok(ReconstructionState::Partial { progress })
    }

    /// Number of shards received so far.
    pub fn received_count(&self) -> usize {
        self.received_count
    }

    /// Number of groups that have been fully reconstructed.
    pub fn completed_groups(&self) -> usize {
        self.groups.iter().filter(|g| g.reconstructed).count()
    }

    /// Total shards expected across all groups.
    pub fn total_shards(&self) -> usize {
        self.total_shards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create shards from data using the given config.
    fn make_shards(data: &[u8], config: ShardingConfig) -> Vec<Shard> {
        let sharder = Sharder::new(config).expect("test: create sharder");
        let (shards, _) = sharder.shard(data).expect("test: shard data");
        shards
    }

    #[test]
    fn test_partial_progress() {
        let config = ShardingConfig {
            data_shards: 4,
            parity_shards: 2,
            target_shard_size: 1024,
        };
        let data = vec![0xABu8; 200];
        let shards = make_shards(&data, config.clone());

        let mut recon = StreamingReconstructor::single_group(config);

        // Feed first 2 data shards -> should be Partial.
        for i in 0..2 {
            let state = recon
                .add_shard(0, shards[i].clone())
                .expect("test: add shard");
            match state {
                ReconstructionState::Partial { progress } => {
                    assert!(progress > 0.0);
                    assert!(progress < 1.0);
                }
                ReconstructionState::Complete { .. } => {
                    unreachable!("should not be complete with only 2 of 4 data shards");
                }
            }
        }
        assert_eq!(recon.received_count(), 2);
    }

    #[test]
    fn test_full_reconstruction_single_group() {
        let config = ShardingConfig {
            data_shards: 4,
            parity_shards: 2,
            target_shard_size: 1024,
        };
        let data = vec![0xCDu8; 200];
        let shards = make_shards(&data, config.clone());

        let mut recon = StreamingReconstructor::single_group(config);

        // Feed all 6 shards; should complete when 4th data shard arrives.
        let mut completed = false;
        for shard in &shards {
            let state = recon
                .add_shard(0, shard.clone())
                .expect("test: add shard");
            if let ReconstructionState::Complete { data: output } = state {
                assert_eq!(output, data);
                completed = true;
                break;
            }
        }
        assert!(completed, "reconstruction should complete");
        assert_eq!(recon.completed_groups(), 1);
    }

    #[test]
    fn test_reconstruction_with_parity_only() {
        // Use a mix of data and parity shards to verify RS recovery works.
        let config = ShardingConfig {
            data_shards: 4,
            parity_shards: 2,
            target_shard_size: 1024,
        };
        let data = vec![0xEFu8; 300];
        let shards = make_shards(&data, config.clone());

        let mut recon = StreamingReconstructor::single_group(config);

        // Feed shards 0,1,4,5 (2 data + 2 parity = 4 total >= k=4).
        let indices = [0, 1, 4, 5];
        let mut completed = false;
        for &idx in &indices {
            let state = recon
                .add_shard(0, shards[idx].clone())
                .expect("test: add shard");
            if let ReconstructionState::Complete { data: output } = state {
                assert_eq!(output, data);
                completed = true;
                break;
            }
        }
        assert!(completed, "should reconstruct from 2 data + 2 parity");
    }

    #[test]
    fn test_multi_group_reconstruction() {
        let config = ShardingConfig {
            data_shards: 4,
            parity_shards: 2,
            target_shard_size: 1024,
        };
        let group_count = 2;

        // Create data for two separate groups.
        let data_g0 = vec![0xAAu8; 200];
        let data_g1 = vec![0xBBu8; 200];

        let shards_g0 = make_shards(&data_g0, config.clone());
        let shards_g1 = make_shards(&data_g1, config.clone());

        let mut recon = StreamingReconstructor::new(group_count, config.clone());

        // Feed group 0 shards.
        for shard in &shards_g0 {
            let _ = recon
                .add_shard(0, shard.clone())
                .expect("test: add g0 shard");
        }
        assert_eq!(recon.completed_groups(), 1);

        // Feed group 1 shards.
        let mut final_state = None;
        for shard in &shards_g1 {
            let state = recon
                .add_shard(1, shard.clone())
                .expect("test: add g1 shard");
            final_state = Some(state);
        }
        assert_eq!(recon.completed_groups(), 2);

        // The final add should have returned Complete with both groups concatenated.
        match final_state {
            Some(ReconstructionState::Complete { data: output }) => {
                let mut expected = data_g0;
                expected.extend_from_slice(&data_g1);
                assert_eq!(output, expected);
            }
            _ => unreachable!("expected Complete after all groups reconstructed"),
        }
    }
}
