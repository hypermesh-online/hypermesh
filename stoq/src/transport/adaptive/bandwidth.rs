// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::{debug, trace};

#[derive(Debug, Clone)]
pub struct BandwidthSample {
    pub bytes: u64,
    pub duration: Duration,
    pub timestamp: Instant,
}

pub struct EwmaBandwidthEstimator {
    alpha: f64,
    current_estimate_bps: f64,
    samples: VecDeque<BandwidthSample>,
    max_samples: usize,
}

impl EwmaBandwidthEstimator {
    pub fn new(alpha: f64, max_samples: usize) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            current_estimate_bps: 0.0,
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    pub fn add_sample(&mut self, bytes: u64, duration: Duration) {
        let secs = duration.as_secs_f64();
        if secs <= 0.0 || bytes == 0 {
            debug!("Skipping zero-duration or zero-byte bandwidth sample");
            return;
        }

        let sample_bps = (bytes as f64 * 8.0) / secs;

        if self.current_estimate_bps <= 0.0 {
            self.current_estimate_bps = sample_bps;
        } else {
            self.current_estimate_bps =
                self.alpha * sample_bps + (1.0 - self.alpha) * self.current_estimate_bps;
        }

        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(BandwidthSample {
            bytes,
            duration,
            timestamp: Instant::now(),
        });

        trace!(
            "EWMA bandwidth: sample={:.2} Mbps, estimate={:.2} Mbps",
            sample_bps / 1_000_000.0,
            self.current_estimate_bps / 1_000_000.0,
        );
    }

    pub fn estimate_bps(&self) -> f64 {
        self.current_estimate_bps
    }

    pub fn estimate_gbps(&self) -> f64 {
        self.current_estimate_bps / 1_000_000_000.0
    }

    pub fn reset(&mut self) {
        self.current_estimate_bps = 0.0;
        self.samples.clear();
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ewma_convergence() {
        let mut est = EwmaBandwidthEstimator::new(0.125, 20);

        let bytes_per_sample: u64 = 125_000_000;
        let dur = Duration::from_secs(1);
        for _ in 0..10 {
            est.add_sample(bytes_per_sample, dur);
        }

        let estimate_gbps = est.estimate_gbps();
        assert!(
            (estimate_gbps - 1.0).abs() < 0.35,
            "expected ~1.0 Gbps, got {estimate_gbps:.4}"
        );
    }

    #[test]
    fn test_ewma_sample_windowing() {
        let mut est = EwmaBandwidthEstimator::new(0.125, 5);

        for i in 0..10u64 {
            est.add_sample(1_000_000 * (i + 1), Duration::from_millis(100));
        }

        assert_eq!(est.sample_count(), 5);
    }

    #[test]
    fn test_ewma_zero_duration() {
        let mut est = EwmaBandwidthEstimator::new(0.125, 20);

        est.add_sample(1_000_000, Duration::ZERO);
        assert_eq!(est.sample_count(), 0);
        assert!((est.estimate_bps() - 0.0).abs() < f64::EPSILON);

        est.add_sample(0, Duration::from_secs(1));
        assert_eq!(est.sample_count(), 0);
    }
}
