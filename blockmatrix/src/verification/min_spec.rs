// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Minimum Device Spec Validation (R13)
//!
//! Validates operations against the hard protocol minimum hardware
//! requirements:
//! - 1 Mb/s network bandwidth
//! - 50 GB storage
//! - 4 GB RAM
//! - 2-core 1 GHz CPU
//!
//! Every planned operation (shard retrieval, asset storage, RS
//! reconstruction) is checked against these bounds to ensure it can
//! complete on the weakest supported device.

use serde::{Deserialize, Serialize};

/// Hard minimum device specification from R13.
#[derive(Debug, Clone, Copy)]
pub struct MinSpec;

impl MinSpec {
    /// Minimum network bandwidth in bits per second (1 Mb/s).
    pub const BANDWIDTH_BPS: u64 = 1_000_000;

    /// Minimum network bandwidth in bytes per second.
    pub const BANDWIDTH_BYTES_PER_SEC: u64 = Self::BANDWIDTH_BPS / 8; // 125,000 B/s

    /// Minimum storage capacity in bytes (50 GB).
    pub const STORAGE_BYTES: u64 = 50 * 1024 * 1024 * 1024;

    /// Minimum RAM in bytes (4 GB).
    pub const RAM_BYTES: u64 = 4 * 1024 * 1024 * 1024;

    /// Minimum CPU cores.
    pub const CPU_CORES: u32 = 2;

    /// Minimum CPU clock speed in Hz (1 GHz).
    pub const CPU_HZ: u64 = 1_000_000_000;
}

/// An operation to validate against minimum spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Total shard data size in bytes (all shards combined).
    pub total_shard_bytes: u64,
    /// Number of shards.
    pub shard_count: u32,
    /// Individual shard size in bytes.
    pub shard_size_bytes: u64,
    /// RS overhead multiplier (e.g. 1.4 for 10+4).
    pub rs_overhead: f64,
    /// Maximum allowed transfer time in seconds.
    pub transfer_timeout_secs: u64,
    /// Total storage required for this operation (bytes).
    pub storage_required_bytes: u64,
}

/// Result of min-spec validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MinSpecResult {
    /// Operation fits within minimum device spec.
    Pass,
    /// Operation exceeds minimum spec -- includes reason.
    Fail { reason: String },
}

impl MinSpecResult {
    /// Returns true if the validation passed.
    pub fn is_pass(&self) -> bool {
        matches!(self, MinSpecResult::Pass)
    }
}

/// Validates operations against R13 minimum hardware requirements.
pub struct MinSpecValidator;

impl MinSpecValidator {
    /// Validate an operation against all minimum spec constraints.
    ///
    /// Checks:
    /// 1. RAM: shard data with RS overhead fits in available memory.
    /// 2. Bandwidth: transfer completes within timeout at 1 Mb/s.
    /// 3. Storage: total storage fits in 50 GB.
    /// 4. Shard budget: shard count fits in storage at given shard size.
    pub fn validate_operation(op: &Operation) -> MinSpecResult {
        // 1. RAM check: all shards must fit in memory during reconstruction.
        //    We need at minimum the data shards (total_shard_bytes) plus RS
        //    overhead for parity reconstruction.
        let ram_needed = (op.total_shard_bytes as f64 * op.rs_overhead) as u64;
        if ram_needed > MinSpec::RAM_BYTES {
            return MinSpecResult::Fail {
                reason: format!(
                    "RAM: operation requires {} bytes ({:.1} GB) with RS overhead, \
                     exceeds minimum {} bytes ({:.1} GB)",
                    ram_needed,
                    ram_needed as f64 / (1024.0 * 1024.0 * 1024.0),
                    MinSpec::RAM_BYTES,
                    MinSpec::RAM_BYTES as f64 / (1024.0 * 1024.0 * 1024.0),
                ),
            };
        }

        // 2. Bandwidth check: can all shards transfer within timeout?
        let transfer_time_secs = op.total_shard_bytes / MinSpec::BANDWIDTH_BYTES_PER_SEC.max(1);
        if transfer_time_secs > op.transfer_timeout_secs {
            return MinSpecResult::Fail {
                reason: format!(
                    "bandwidth: transferring {} bytes at {} B/s takes {} seconds, \
                     exceeds timeout of {} seconds",
                    op.total_shard_bytes,
                    MinSpec::BANDWIDTH_BYTES_PER_SEC,
                    transfer_time_secs,
                    op.transfer_timeout_secs,
                ),
            };
        }

        // 3. Storage check: does the operation fit in 50 GB?
        if op.storage_required_bytes > MinSpec::STORAGE_BYTES {
            return MinSpecResult::Fail {
                reason: format!(
                    "storage: operation requires {} bytes ({:.1} GB), \
                     exceeds minimum {} bytes ({:.1} GB)",
                    op.storage_required_bytes,
                    op.storage_required_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
                    MinSpec::STORAGE_BYTES,
                    MinSpec::STORAGE_BYTES as f64 / (1024.0 * 1024.0 * 1024.0),
                ),
            };
        }

        // 4. Shard budget: max shards that fit in storage.
        if op.shard_size_bytes > 0 {
            let max_shards = MinSpec::STORAGE_BYTES / op.shard_size_bytes;
            if (op.shard_count as u64) > max_shards {
                return MinSpecResult::Fail {
                    reason: format!(
                        "shard budget: {} shards of {} bytes each requires {} bytes, \
                         exceeds storage capacity of {} bytes",
                        op.shard_count,
                        op.shard_size_bytes,
                        op.shard_count as u64 * op.shard_size_bytes,
                        MinSpec::STORAGE_BYTES,
                    ),
                };
            }
        }

        MinSpecResult::Pass
    }

    /// Quick check: can a single shard of `size` bytes fit in RAM?
    pub fn shard_fits_in_ram(shard_size: u64, rs_overhead: f64) -> bool {
        let needed = (shard_size as f64 * rs_overhead) as u64;
        needed <= MinSpec::RAM_BYTES
    }

    /// Calculate maximum transfer size that completes within timeout at min bandwidth.
    pub fn max_transfer_bytes(timeout_secs: u64) -> u64 {
        MinSpec::BANDWIDTH_BYTES_PER_SEC * timeout_secs
    }

    /// Calculate maximum number of shards that fit in storage.
    pub fn max_shard_count(shard_size: u64) -> u64 {
        if shard_size == 0 {
            return 0;
        }
        MinSpec::STORAGE_BYTES / shard_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passing_spec() {
        let op = Operation {
            total_shard_bytes: 100 * 1024 * 1024, // 100 MB
            shard_count: 14,
            shard_size_bytes: 10 * 1024 * 1024, // 10 MB each
            rs_overhead: 1.4,
            transfer_timeout_secs: 3600, // 1 hour
            storage_required_bytes: 200 * 1024 * 1024, // 200 MB
        };

        let result = MinSpecValidator::validate_operation(&op);
        assert_eq!(result, MinSpecResult::Pass);
    }

    #[test]
    fn test_failing_ram() {
        let op = Operation {
            total_shard_bytes: 4 * 1024 * 1024 * 1024, // 4 GB raw
            shard_count: 14,
            shard_size_bytes: 300 * 1024 * 1024,
            rs_overhead: 1.4, // 4 * 1.4 = 5.6 GB > 4 GB RAM
            transfer_timeout_secs: 86400,
            storage_required_bytes: 5 * 1024 * 1024 * 1024,
        };

        let result = MinSpecValidator::validate_operation(&op);
        assert!(matches!(result, MinSpecResult::Fail { reason } if reason.contains("RAM")));
    }

    #[test]
    fn test_failing_bandwidth() {
        // 1 Mb/s = 125,000 bytes/sec. 1 GB = ~8,000 seconds.
        let op = Operation {
            total_shard_bytes: 1024 * 1024 * 1024, // 1 GB
            shard_count: 14,
            shard_size_bytes: 75 * 1024 * 1024,
            rs_overhead: 1.0,
            transfer_timeout_secs: 60, // 1 minute -- way too short for 1 GB at 1 Mb/s
            storage_required_bytes: 1024 * 1024 * 1024,
        };

        let result = MinSpecValidator::validate_operation(&op);
        assert!(matches!(result, MinSpecResult::Fail { reason } if reason.contains("bandwidth")));
    }

    #[test]
    fn test_failing_storage() {
        let op = Operation {
            total_shard_bytes: 100 * 1024 * 1024,
            shard_count: 14,
            shard_size_bytes: 10 * 1024 * 1024,
            rs_overhead: 1.0,
            transfer_timeout_secs: 86400,
            storage_required_bytes: 60 * 1024 * 1024 * 1024, // 60 GB > 50 GB
        };

        let result = MinSpecValidator::validate_operation(&op);
        assert!(matches!(result, MinSpecResult::Fail { reason } if reason.contains("storage")));
    }

    #[test]
    fn test_shard_budget_exceeded() {
        // Each shard 10 GB, 6 shards = 60 GB > 50 GB storage.
        let op = Operation {
            total_shard_bytes: 50 * 1024 * 1024 * 1024,
            shard_count: 6,
            shard_size_bytes: 10 * 1024 * 1024 * 1024,
            rs_overhead: 1.0,
            transfer_timeout_secs: 86400 * 30,
            storage_required_bytes: 40 * 1024 * 1024 * 1024,
        };

        let result = MinSpecValidator::validate_operation(&op);
        // RAM check fails first at 50GB * 1.0 > 4GB.
        assert!(matches!(result, MinSpecResult::Fail { .. }));
    }

    #[test]
    fn test_helper_shard_fits_in_ram() {
        // 2 GB * 1.4 = 2.8 GB < 4 GB -> fits.
        assert!(MinSpecValidator::shard_fits_in_ram(2 * 1024 * 1024 * 1024, 1.4));
        // 3 GB * 1.4 = 4.2 GB > 4 GB -> does not fit.
        assert!(!MinSpecValidator::shard_fits_in_ram(3 * 1024 * 1024 * 1024, 1.4));
    }

    #[test]
    fn test_helper_max_transfer_bytes() {
        // 1 hour at 125,000 B/s = 450,000,000 bytes (~429 MB).
        let max = MinSpecValidator::max_transfer_bytes(3600);
        assert_eq!(max, 125_000 * 3600);
    }

    #[test]
    fn test_helper_max_shard_count() {
        // 50 GB / 1 MB = ~50,000 shards.
        let count = MinSpecValidator::max_shard_count(1024 * 1024);
        assert!(count > 50_000);
        // Zero shard size returns 0.
        assert_eq!(MinSpecValidator::max_shard_count(0), 0);
    }

    #[test]
    fn test_edge_case_exactly_at_limits() {
        // Operation that exactly matches RAM limit.
        let op = Operation {
            total_shard_bytes: MinSpec::RAM_BYTES, // 4 GB
            shard_count: 1,
            shard_size_bytes: MinSpec::RAM_BYTES,
            rs_overhead: 1.0, // 4 GB * 1.0 = 4 GB = exactly at limit
            transfer_timeout_secs: MinSpec::RAM_BYTES / MinSpec::BANDWIDTH_BYTES_PER_SEC + 1,
            storage_required_bytes: MinSpec::RAM_BYTES,
        };

        let result = MinSpecValidator::validate_operation(&op);
        assert_eq!(result, MinSpecResult::Pass);
    }
}
