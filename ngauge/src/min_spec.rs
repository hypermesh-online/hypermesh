// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Minimum-specification performance profiling (R13).
//!
//! Verifies that ngauge analytics overhead fits within the R13 budget:
//! - 4 GB RAM total -> ngauge should use < 256 MB
//! - 1 Mb/s network -> metrics traffic < 10 KB/s
//! - 2-core 1 GHz CPU -> ngauge should use < 5% CPU

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// R13 Budget constants
// ---------------------------------------------------------------------------

/// Maximum memory budget for ngauge operations (256 MB).
pub const MAX_MEMORY_BYTES: u64 = 256 * 1024 * 1024;

/// Maximum bandwidth budget for metrics traffic (10 KB/s).
pub const MAX_BANDWIDTH_BPS: u64 = 10 * 1024 * 8; // 80 Kbps

/// Maximum CPU fraction for ngauge operations (5%).
pub const MAX_CPU_FRACTION: f64 = 0.05;

// ---------------------------------------------------------------------------
// ResourceUsage
// ---------------------------------------------------------------------------

/// Resource usage measurement for a single operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Memory usage in bytes.
    pub memory_bytes: u64,
    /// CPU fraction (0.0 to 1.0) over the measurement period.
    pub cpu_fraction: f64,
    /// Bandwidth usage in bits per second.
    pub bandwidth_bps: u64,
    /// Duration of the operation.
    pub duration: Duration,
    /// Name of the measured operation.
    pub operation: String,
}

impl ResourceUsage {
    /// Check if this usage fits within the R13 minimum device spec budget.
    pub fn fits_min_spec(&self) -> bool {
        self.memory_bytes <= MAX_MEMORY_BYTES
            && self.cpu_fraction <= MAX_CPU_FRACTION
            && self.bandwidth_bps <= MAX_BANDWIDTH_BPS
    }
}

// ---------------------------------------------------------------------------
// MinSpecProfiler
// ---------------------------------------------------------------------------

/// Profiles ngauge operations against R13 minimum device spec.
///
/// Use [`profile_operation`] to measure a closure's resource usage,
/// or [`estimate_steady_state`] to estimate ongoing resource consumption.
pub struct MinSpecProfiler {
    /// Collected profiles.
    profiles: Vec<ResourceUsage>,
}

impl MinSpecProfiler {
    /// Create a new profiler.
    pub fn new() -> Self {
        Self {
            profiles: Vec::new(),
        }
    }

    /// Profile a synchronous operation, measuring wall-clock time.
    ///
    /// Memory and bandwidth estimates must be provided by the caller
    /// since precise measurement requires OS-specific APIs. The profiler
    /// measures duration and computes CPU fraction from it.
    pub fn profile_operation(
        &mut self,
        name: &str,
        estimated_memory_bytes: u64,
        estimated_bandwidth_bps: u64,
        operation: impl FnOnce(),
    ) -> ResourceUsage {
        let start = Instant::now();
        operation();
        let duration = start.elapsed();

        // Estimate CPU fraction: duration / available_cpu_time.
        // On a 2-core system with 1 second wall time, max CPU time = 2s.
        // We assume single-threaded operation for ngauge analytics.
        let cpu_fraction = if duration.as_secs_f64() > 0.0 {
            // Fraction of one core used during the operation.
            // For a 1-second measurement window, using 50ms = 5%.
            duration.as_secs_f64().min(1.0)
        } else {
            0.0
        };

        let usage = ResourceUsage {
            memory_bytes: estimated_memory_bytes,
            cpu_fraction,
            bandwidth_bps: estimated_bandwidth_bps,
            duration,
            operation: name.to_string(),
        };

        self.profiles.push(usage.clone());
        usage
    }

    /// Estimate steady-state resource usage from collected profiles.
    ///
    /// Returns the sum of memory, max CPU fraction, and sum of bandwidth
    /// across all profiled operations (representing concurrent load).
    pub fn estimate_steady_state(&self) -> ResourceUsage {
        let total_memory: u64 = self.profiles.iter().map(|p| p.memory_bytes).sum();
        let max_cpu: f64 = self
            .profiles
            .iter()
            .map(|p| p.cpu_fraction)
            .fold(0.0_f64, f64::max);
        let total_bandwidth: u64 = self.profiles.iter().map(|p| p.bandwidth_bps).sum();
        let total_duration: Duration = self.profiles.iter().map(|p| p.duration).sum();

        ResourceUsage {
            memory_bytes: total_memory,
            cpu_fraction: max_cpu,
            bandwidth_bps: total_bandwidth,
            duration: total_duration,
            operation: "steady_state_estimate".to_string(),
        }
    }

    /// Check if steady-state usage fits the R13 min-spec budget.
    pub fn fits_min_spec(&self) -> bool {
        self.estimate_steady_state().fits_min_spec()
    }

    /// Get all collected profiles.
    pub fn profiles(&self) -> &[ResourceUsage] {
        &self.profiles
    }

    /// Clear all collected profiles.
    pub fn clear(&mut self) {
        self.profiles.clear();
    }
}

impl Default for MinSpecProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a resource usage fits within R13 budget.
pub fn fits_min_spec(usage: &ResourceUsage) -> bool {
    usage.fits_min_spec()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_usage_within_budget() {
        let usage = ResourceUsage {
            memory_bytes: 100 * 1024 * 1024, // 100 MB
            cpu_fraction: 0.02,              // 2%
            bandwidth_bps: 40_000,            // 5 KB/s
            duration: Duration::from_millis(20),
            operation: "test_op".to_string(),
        };

        assert!(usage.fits_min_spec(), "usage should fit within R13 budget");
    }

    #[test]
    fn resource_usage_exceeds_memory_budget() {
        let usage = ResourceUsage {
            memory_bytes: 300 * 1024 * 1024, // 300 MB > 256 MB
            cpu_fraction: 0.01,
            bandwidth_bps: 1000,
            duration: Duration::from_millis(10),
            operation: "memory_heavy".to_string(),
        };

        assert!(
            !usage.fits_min_spec(),
            "300 MB should exceed R13 memory budget"
        );
    }

    #[test]
    fn profiler_measures_operation() {
        let mut profiler = MinSpecProfiler::new();

        let usage = profiler.profile_operation(
            "hash_computation",
            1024, // 1 KB estimated memory
            0,    // no bandwidth
            || {
                // Simulate a small computation.
                let mut _sum = 0u64;
                for i in 0..1000 {
                    _sum = _sum.wrapping_add(i);
                }
            },
        );

        assert_eq!(usage.operation, "hash_computation");
        assert_eq!(usage.memory_bytes, 1024);
        assert!(usage.fits_min_spec());
        assert_eq!(profiler.profiles().len(), 1);
    }

    #[test]
    fn steady_state_estimate_aggregates() {
        let mut profiler = MinSpecProfiler::new();

        profiler.profile_operation("op_a", 50 * 1024 * 1024, 20_000, || {});
        profiler.profile_operation("op_b", 30 * 1024 * 1024, 10_000, || {});

        let steady = profiler.estimate_steady_state();
        assert_eq!(
            steady.memory_bytes,
            80 * 1024 * 1024,
            "memory should be summed"
        );
        assert_eq!(steady.bandwidth_bps, 30_000, "bandwidth should be summed");
        assert!(
            profiler.fits_min_spec(),
            "80 MB + 30 Kbps should fit R13 budget"
        );
    }
}
