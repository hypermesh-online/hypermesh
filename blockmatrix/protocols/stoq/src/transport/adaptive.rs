//! Adaptive bandwidth detection and configuration
//!
//! This module implements real-time network capability detection and automatically
//! adjusts QUIC transport configuration to optimize performance for different
//! network tiers: 100 Mbps, 1 Gbps, and 2.5+ Gbps.

use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};
use anyhow::Result;

use super::TransportConfig;

/// Network tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkTier {
    /// Less than 100 Mbps - basic tier with conservative settings
    Basic,
    /// 100 Mbps to 1 Gbps - standard tier with optimized settings
    Standard,
    /// 1 Gbps to 2.5 Gbps - high performance tier
    HighPerformance,
    /// Above 2.5 Gbps - ultra-high performance tier
    UltraHigh,
}

impl NetworkTier {
    /// Get the tier based on measured throughput in Mbps
    pub fn from_throughput_mbps(throughput: f64) -> Self {
        if throughput >= 2500.0 {
            Self::UltraHigh
        } else if throughput >= 1000.0 {
            Self::HighPerformance
        } else if throughput >= 100.0 {
            Self::Standard
        } else {
            Self::Basic
        }
    }

    /// Get the tier description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Basic => "Basic (< 100 Mbps)",
            Self::Standard => "Standard (100 Mbps - 1 Gbps)",
            Self::HighPerformance => "High Performance (1-2.5 Gbps)",
            Self::UltraHigh => "Ultra High (> 2.5 Gbps)",
        }
    }

    /// Get optimal configuration for this tier
    pub fn get_config(&self) -> TierConfig {
        match self {
            Self::Basic => TierConfig {
                max_concurrent_streams: 100,
                send_buffer_size: 64 * 1024,      // 64KB
                receive_buffer_size: 64 * 1024,   // 64KB
                max_idle_timeout: Duration::from_secs(30),
                keep_alive_interval: Duration::from_secs(10),
                max_concurrent_connections: 100,
            },
            Self::Standard => TierConfig {
                max_concurrent_streams: 1000,
                send_buffer_size: 1 * 1024 * 1024,    // 1MB
                receive_buffer_size: 1 * 1024 * 1024, // 1MB
                max_idle_timeout: Duration::from_secs(60),
                keep_alive_interval: Duration::from_secs(15),
                max_concurrent_connections: 1000,
            },
            Self::HighPerformance => TierConfig {
                max_concurrent_streams: 10000,
                send_buffer_size: 4 * 1024 * 1024,    // 4MB
                receive_buffer_size: 4 * 1024 * 1024, // 4MB
                max_idle_timeout: Duration::from_secs(120),
                keep_alive_interval: Duration::from_secs(20),
                max_concurrent_connections: 10000,
            },
            Self::UltraHigh => TierConfig {
                max_concurrent_streams: 100000,
                send_buffer_size: 16 * 1024 * 1024,   // 16MB
                receive_buffer_size: 16 * 1024 * 1024, // 16MB
                max_idle_timeout: Duration::from_secs(300),
                keep_alive_interval: Duration::from_secs(30),
                max_concurrent_connections: 100000,
            },
        }
    }
}

/// Configuration parameters for a network tier
#[derive(Debug, Clone)]
pub struct TierConfig {
    pub max_concurrent_streams: u32,
    pub send_buffer_size: usize,
    pub receive_buffer_size: usize,
    pub max_idle_timeout: Duration,
    pub keep_alive_interval: Duration,
    pub max_concurrent_connections: u32,
}

/// Measurement sample for bandwidth detection
#[derive(Debug, Clone)]
pub struct BandwidthSample {
    pub timestamp: Instant,
    pub bytes_transferred: u64,
    pub duration: Duration,
    pub throughput_mbps: f64,
}

/// Adaptive bandwidth detector
pub struct AdaptiveBandwidthDetector {
    /// Current detected network tier
    current_tier: Arc<RwLock<NetworkTier>>,
    /// History of bandwidth measurements
    samples: Arc<RwLock<Vec<BandwidthSample>>>,
    /// Last configuration update time
    last_update: Arc<RwLock<Instant>>,
    /// Detection parameters
    config: DetectorConfig,
}

/// Configuration for the adaptive detector
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    /// Minimum interval between tier updates
    pub update_interval: Duration,
    /// Number of samples to keep for analysis
    pub sample_window_size: usize,
    /// Minimum transfer size to consider for measurement
    pub min_transfer_bytes: u64,
    /// Tier change hysteresis factor (prevents oscillation)
    pub hysteresis_factor: f64,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            update_interval: Duration::from_secs(30),
            sample_window_size: 20,
            min_transfer_bytes: 1024 * 1024, // 1MB minimum
            hysteresis_factor: 0.2, // 20% hysteresis
        }
    }
}

impl AdaptiveBandwidthDetector {
    /// Create a new adaptive bandwidth detector
    pub fn new() -> Self {
        Self::with_config(DetectorConfig::default())
    }

    /// Create a new detector with custom configuration
    pub fn with_config(config: DetectorConfig) -> Self {
        Self {
            current_tier: Arc::new(RwLock::new(NetworkTier::Standard)), // Start with standard tier
            samples: Arc::new(RwLock::new(Vec::new())),
            last_update: Arc::new(RwLock::new(Instant::now())),
            config,
        }
    }

    /// Record a data transfer for bandwidth analysis
    pub fn record_transfer(&self, bytes: u64, duration: Duration) {
        // Only consider significant transfers
        if bytes < self.config.min_transfer_bytes {
            return;
        }

        let throughput_mbps = (bytes as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000.0);

        let sample = BandwidthSample {
            timestamp: Instant::now(),
            bytes_transferred: bytes,
            duration,
            throughput_mbps,
        };

        // Add sample and maintain window size
        {
            let mut samples = self.samples.write();
            samples.push(sample);

            // Keep only recent samples
            if samples.len() > self.config.sample_window_size {
                let excess = samples.len() - self.config.sample_window_size;
                samples.drain(0..excess);
            }
        }

        // Check if we should update the tier
        self.maybe_update_tier();
    }

    /// Get the current detected network tier
    pub fn current_tier(&self) -> NetworkTier {
        *self.current_tier.read()
    }

    /// Get the current tier configuration
    pub fn current_config(&self) -> TierConfig {
        self.current_tier().get_config()
    }

    /// Force a tier analysis and potential update
    pub fn force_update(&self) -> bool {
        self.analyze_and_update_tier(true)
    }

    /// Check if tier should be updated based on recent measurements
    fn maybe_update_tier(&self) {
        let last_update = *self.last_update.read();
        if last_update.elapsed() >= self.config.update_interval {
            self.analyze_and_update_tier(false);
        }
    }

    /// Analyze recent samples and update tier if needed
    fn analyze_and_update_tier(&self, force: bool) -> bool {
        let samples = self.samples.read();

        // Need enough samples for analysis
        if samples.len() < 3 && !force {
            return false;
        }

        // Calculate recent average throughput
        let recent_samples: Vec<_> = samples.iter()
            .filter(|s| s.timestamp.elapsed() < Duration::from_secs(120)) // Last 2 minutes
            .collect();

        if recent_samples.is_empty() {
            return false;
        }

        // Calculate weighted average (more recent samples have higher weight)
        let total_weight: f64 = recent_samples.iter()
            .map(|s| {
                let age_factor = 1.0 - (s.timestamp.elapsed().as_secs_f64() / 120.0).min(1.0);
                age_factor.max(0.1) // Minimum weight of 10%
            })
            .sum();

        let weighted_throughput: f64 = recent_samples.iter()
            .map(|s| {
                let age_factor = 1.0 - (s.timestamp.elapsed().as_secs_f64() / 120.0).min(1.0);
                let weight = age_factor.max(0.1);
                s.throughput_mbps * weight
            })
            .sum();

        let avg_throughput = weighted_throughput / total_weight;
        let detected_tier = NetworkTier::from_throughput_mbps(avg_throughput);

        // Apply hysteresis to prevent oscillation
        let current_tier = *self.current_tier.read();
        let should_update = if detected_tier != current_tier {
            match (current_tier, detected_tier) {
                // Moving up a tier - require higher confidence
                (NetworkTier::Basic, NetworkTier::Standard) => {
                    avg_throughput >= 100.0 * (1.0 + self.config.hysteresis_factor)
                },
                (NetworkTier::Standard, NetworkTier::HighPerformance) => {
                    avg_throughput >= 1000.0 * (1.0 + self.config.hysteresis_factor)
                },
                (NetworkTier::HighPerformance, NetworkTier::UltraHigh) => {
                    avg_throughput >= 2500.0 * (1.0 + self.config.hysteresis_factor)
                },
                // Moving down a tier - require lower confidence
                (NetworkTier::Standard, NetworkTier::Basic) => {
                    avg_throughput <= 100.0 * (1.0 - self.config.hysteresis_factor)
                },
                (NetworkTier::HighPerformance, NetworkTier::Standard) => {
                    avg_throughput <= 1000.0 * (1.0 - self.config.hysteresis_factor)
                },
                (NetworkTier::UltraHigh, NetworkTier::HighPerformance) => {
                    avg_throughput <= 2500.0 * (1.0 - self.config.hysteresis_factor)
                },
                // Large jumps (e.g., Basic to HighPerformance) - always update
                _ => true,
            }
        } else {
            false
        };

        if should_update || force {
            *self.current_tier.write() = detected_tier;
            *self.last_update.write() = Instant::now();

            info!(
                "Network tier updated: {} -> {} (avg throughput: {:.1} Mbps)",
                current_tier.description(),
                detected_tier.description(),
                avg_throughput
            );

            debug!(
                "Tier update based on {} samples, weighted average: {:.1} Mbps",
                recent_samples.len(),
                avg_throughput
            );

            return true;
        }

        false
    }

    /// Get statistics about recent measurements
    pub fn get_stats(&self) -> DetectorStats {
        let samples = self.samples.read();
        let current_tier = *self.current_tier.read();

        if samples.is_empty() {
            return DetectorStats {
                current_tier,
                sample_count: 0,
                avg_throughput_mbps: 0.0,
                min_throughput_mbps: 0.0,
                max_throughput_mbps: 0.0,
                last_measurement_secs: None,
            };
        }

        let throughputs: Vec<f64> = samples.iter().map(|s| s.throughput_mbps).collect();
        let avg_throughput = throughputs.iter().sum::<f64>() / throughputs.len() as f64;
        let min_throughput = throughputs.iter().copied().fold(f64::INFINITY, f64::min);
        let max_throughput = throughputs.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        DetectorStats {
            current_tier,
            sample_count: samples.len(),
            avg_throughput_mbps: avg_throughput,
            min_throughput_mbps: min_throughput,
            max_throughput_mbps: max_throughput,
            last_measurement_secs: samples.last().map(|s| s.timestamp.elapsed().as_secs_f64()),
        }
    }

    /// Apply tier configuration to transport config
    pub fn apply_to_transport_config(&self, transport_config: &mut TransportConfig) {
        let tier_config = self.current_config();

        transport_config.max_concurrent_streams = tier_config.max_concurrent_streams;
        transport_config.send_buffer_size = tier_config.send_buffer_size;
        transport_config.receive_buffer_size = tier_config.receive_buffer_size;
        transport_config.max_idle_timeout = tier_config.max_idle_timeout;

        if let Some(max_connections) = transport_config.max_connections.as_mut() {
            *max_connections = tier_config.max_concurrent_connections;
        }

        debug!(
            "Applied {} configuration to transport",
            self.current_tier().description()
        );
    }
}

/// Statistics about the adaptive bandwidth detector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorStats {
    pub current_tier: NetworkTier,
    pub sample_count: usize,
    pub avg_throughput_mbps: f64,
    pub min_throughput_mbps: f64,
    pub max_throughput_mbps: f64,
    /// Seconds since the last measurement
    pub last_measurement_secs: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_tier_classification() {
        assert_eq!(NetworkTier::from_throughput_mbps(50.0), NetworkTier::Basic);
        assert_eq!(NetworkTier::from_throughput_mbps(500.0), NetworkTier::Standard);
        assert_eq!(NetworkTier::from_throughput_mbps(1500.0), NetworkTier::HighPerformance);
        assert_eq!(NetworkTier::from_throughput_mbps(3000.0), NetworkTier::UltraHigh);
    }

    #[test]
    fn test_detector_creation() {
        let detector = AdaptiveBandwidthDetector::new();
        assert_eq!(detector.current_tier(), NetworkTier::Standard);

        let stats = detector.get_stats();
        assert_eq!(stats.sample_count, 0);
    }

    #[tokio::test]
    async fn test_bandwidth_measurement() {
        let detector = AdaptiveBandwidthDetector::new();

        // Simulate high bandwidth transfer
        let transfer_size = 10 * 1024 * 1024; // 10MB
        let transfer_time = Duration::from_millis(100); // 100ms = 800 Mbps

        detector.record_transfer(transfer_size, transfer_time);

        let stats = detector.get_stats();
        assert_eq!(stats.sample_count, 1);
        assert!(stats.avg_throughput_mbps > 700.0); // Should detect high bandwidth
    }

    #[tokio::test]
    async fn test_tier_progression() {
        let detector = AdaptiveBandwidthDetector::new();

        // Start with standard tier
        assert_eq!(detector.current_tier(), NetworkTier::Standard);

        // Simulate low bandwidth (50 Mbps)
        for _ in 0..5 {
            let transfer_size = 5 * 1024 * 1024; // 5MB
            let transfer_time = Duration::from_millis(800); // 50 Mbps
            detector.record_transfer(transfer_size, transfer_time);
        }
        detector.force_update();
        assert_eq!(detector.current_tier(), NetworkTier::Basic);

        // Simulate medium bandwidth (500 Mbps)
        for _ in 0..5 {
            let transfer_size = 10 * 1024 * 1024; // 10MB
            let transfer_time = Duration::from_millis(160); // 500 Mbps
            detector.record_transfer(transfer_size, transfer_time);
        }
        detector.force_update();
        assert_eq!(detector.current_tier(), NetworkTier::Standard);

        // Simulate high bandwidth (1.5 Gbps) with more samples to overwhelm low bandwidth history
        for _ in 0..15 {
            let transfer_size = 100 * 1024 * 1024; // 100MB
            let transfer_time = Duration::from_millis(533); // ~1.5 Gbps = 1573 Mbps
            detector.record_transfer(transfer_size, transfer_time);
        }
        detector.force_update();
        let stats = detector.get_stats();
        println!("Stats after high bandwidth: {:?}", stats);
        println!("Need >1200 Mbps for Standard->HighPerformance transition");
        // Should detect high performance tier (>1200 Mbps with 20% hysteresis)
        assert_eq!(detector.current_tier(), NetworkTier::HighPerformance);

        // Simulate ultra-high bandwidth (3.2 Gbps) - need to dominate the 20-sample window
        for _ in 0..15 {
            let transfer_size = 200 * 1024 * 1024; // 200MB
            let transfer_time = Duration::from_millis(500); // ~3.2 Gbps = 3276 Mbps
            detector.record_transfer(transfer_size, transfer_time);
        }
        detector.force_update();
        let final_stats = detector.get_stats();
        println!("Stats after ultra-high bandwidth: {:?}", final_stats);
        println!("Need >3000 Mbps for HighPerformance->UltraHigh transition");
        assert_eq!(detector.current_tier(), NetworkTier::UltraHigh);

        println!("✅ Adaptive bandwidth detection tier progression test passed!");
        println!("   Basic -> Standard -> HighPerformance -> UltraHigh");
    }
}