// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::collections::VecDeque;

pub struct LossBasedAdjuster {
    loss_window: VecDeque<f64>,
    window_size: usize,
    downgrade_threshold: f64,
    upgrade_threshold: f64,
}

impl LossBasedAdjuster {
    pub fn new(window_size: usize, downgrade_threshold: f64, upgrade_threshold: f64) -> Self {
        Self {
            loss_window: VecDeque::with_capacity(window_size),
            window_size: window_size.max(1),
            downgrade_threshold,
            upgrade_threshold,
        }
    }

    pub fn record_loss(&mut self, loss_pct: f64) {
        if self.loss_window.len() >= self.window_size {
            self.loss_window.pop_front();
        }
        self.loss_window.push_back(loss_pct);
    }

    pub fn should_downgrade(&self) -> bool {
        if self.loss_window.is_empty() {
            return false;
        }
        self.average_loss() > self.downgrade_threshold
    }

    pub fn should_upgrade(&self) -> bool {
        if self.loss_window.is_empty() {
            return false;
        }
        self.average_loss() < self.upgrade_threshold
    }

    pub fn average_loss(&self) -> f64 {
        if self.loss_window.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.loss_window.iter().sum();
        sum / self.loss_window.len() as f64
    }

    pub fn reset(&mut self) {
        self.loss_window.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loss_downgrade_threshold() {
        let mut adj = LossBasedAdjuster::new(10, 5.0, 0.5);

        for _ in 0..10 {
            adj.record_loss(7.0);
        }

        assert!(adj.should_downgrade(), "average 7% > threshold 5%");
        assert!(!adj.should_upgrade());
    }

    #[test]
    fn test_loss_upgrade_threshold() {
        let mut adj = LossBasedAdjuster::new(10, 5.0, 0.5);

        for _ in 0..10 {
            adj.record_loss(0.1);
        }

        assert!(adj.should_upgrade(), "average 0.1% < threshold 0.5%");
        assert!(!adj.should_downgrade());
    }

    #[test]
    fn test_loss_mixed() {
        let mut adj = LossBasedAdjuster::new(10, 5.0, 0.5);

        for _ in 0..5 {
            adj.record_loss(1.0);
        }
        for _ in 0..5 {
            adj.record_loss(5.0);
        }

        let avg = adj.average_loss();
        assert!(
            (avg - 3.0).abs() < 0.01,
            "expected 3.0, got {avg}"
        );
        assert!(!adj.should_downgrade());
        assert!(!adj.should_upgrade());
    }
}
