// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Adaptive connection optimization for live connections
//! Provides real-time parameter adjustment based on network conditions

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use parking_lot::RwLock;
use quinn::{Connection as QuinnConnection, TransportConfig, VarInt};
use tracing::{debug, info, warn, trace};
use tokio::time::interval;

use super::{NetworkTier, CongestionControl};

/// Network condition metrics for adaptation decisions
#[derive(Debug, Clone)]
pub struct NetworkConditions {
    /// Round-trip time in milliseconds
    pub rtt_ms: f64,
    /// Packet loss percentage (0-100)
    pub packet_loss: f64,
    /// Current throughput in Mbps
    pub throughput_mbps: f64,
    /// Bandwidth estimate in Mbps
    pub bandwidth_estimate: f64,
    /// Number of retransmissions
    pub retransmissions: u64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Last update timestamp
    pub last_update: Instant,
}

impl Default for NetworkConditions {
    fn default() -> Self {
        Self {
            rtt_ms: 0.0,
            packet_loss: 0.0,
            throughput_mbps: 0.0,
            bandwidth_estimate: 1000.0, // Default 1 Gbps
            retransmissions: 0,
            jitter_ms: 0.0,
            last_update: Instant::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// EWMA Bandwidth Estimation
// ---------------------------------------------------------------------------

/// A single bandwidth measurement sample
#[derive(Debug, Clone)]
pub struct BandwidthSample {
    /// Bytes transferred during measurement
    pub bytes: u64,
    /// Duration of the measurement window
    pub duration: Duration,
    /// When this sample was recorded
    pub timestamp: Instant,
}

/// Exponentially-weighted moving average bandwidth estimator.
///
/// Produces a smoothed bandwidth estimate from discrete transfer samples,
/// dampening transient spikes and dips so that tier detection remains stable.
pub struct EwmaBandwidthEstimator {
    /// Smoothing factor (0.0 – 1.0). Higher values weight recent samples more.
    alpha: f64,
    /// Current smoothed estimate in bits per second
    current_estimate_bps: f64,
    /// Recent samples (bounded by `max_samples`)
    samples: VecDeque<BandwidthSample>,
    /// Maximum number of retained samples
    max_samples: usize,
}

impl EwmaBandwidthEstimator {
    /// Create a new estimator.
    ///
    /// * `alpha`       – EWMA smoothing factor (default 0.125)
    /// * `max_samples` – rolling window size (default 20)
    pub fn new(alpha: f64, max_samples: usize) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            current_estimate_bps: 0.0,
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    /// Record a transfer measurement. Samples with zero duration are skipped.
    pub fn add_sample(&mut self, bytes: u64, duration: Duration) {
        let secs = duration.as_secs_f64();
        if secs <= 0.0 || bytes == 0 {
            debug!("Skipping zero-duration or zero-byte bandwidth sample");
            return;
        }

        let sample_bps = (bytes as f64 * 8.0) / secs;

        // EWMA: estimate = alpha * sample + (1 - alpha) * previous
        if self.current_estimate_bps <= 0.0 {
            // First valid sample seeds the estimate directly
            self.current_estimate_bps = sample_bps;
        } else {
            self.current_estimate_bps =
                self.alpha * sample_bps + (1.0 - self.alpha) * self.current_estimate_bps;
        }

        // Maintain sliding window
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

    /// Current smoothed estimate in bits per second
    pub fn estimate_bps(&self) -> f64 {
        self.current_estimate_bps
    }

    /// Current smoothed estimate in gigabits per second
    pub fn estimate_gbps(&self) -> f64 {
        self.current_estimate_bps / 1_000_000_000.0
    }

    /// Reset all state
    pub fn reset(&mut self) {
        self.current_estimate_bps = 0.0;
        self.samples.clear();
    }

    /// Number of samples currently retained
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}

// ---------------------------------------------------------------------------
// MTU Path Discovery
// ---------------------------------------------------------------------------

/// Current state of the MTU binary-search probe
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MtuProbeState {
    /// Actively searching for the path MTU
    Searching,
    /// Path MTU has been confirmed at `current_mtu`
    Confirmed,
    /// Probing failed; fell back to minimum MTU
    Failed,
}

/// Binary-search MTU discovery.
///
/// Starts at `min_mtu`, probes upward in binary-search fashion. On success
/// the search range narrows upward; on failure it narrows downward. Converges
/// when the search range collapses or reaches the ceiling.
pub struct MtuDiscovery {
    /// Best confirmed MTU so far
    current_mtu: u16,
    /// Minimum allowed MTU (QUIC floor = 1200)
    min_mtu: u16,
    /// Maximum allowed MTU (jumbo frame ceiling = 9000)
    max_mtu: u16,
    /// High end of current search range
    search_high: u16,
    /// Low end of current search range
    search_low: u16,
    /// Current probe state
    probe_state: MtuProbeState,
    /// Timestamp of last probe attempt
    last_probe: Instant,
    /// Minimum interval between probes
    probe_interval: Duration,
}

impl MtuDiscovery {
    /// Create a new MTU discovery instance.
    ///
    /// * `min_mtu`        – floor (typically 1200)
    /// * `max_mtu`        – ceiling (typically 9000)
    /// * `probe_interval` – minimum time between probes
    pub fn new(min_mtu: u16, max_mtu: u16, probe_interval: Duration) -> Self {
        let min = min_mtu.max(1200);
        let max = max_mtu.max(min);
        Self {
            current_mtu: min,
            min_mtu: min,
            max_mtu: max,
            search_low: min,
            search_high: max,
            probe_state: MtuProbeState::Searching,
            last_probe: Instant::now() - probe_interval, // allow immediate first probe
            probe_interval,
        }
    }

    /// Returns `true` when it is time to send the next probe packet.
    pub fn should_probe(&self) -> bool {
        self.probe_state == MtuProbeState::Searching
            && self.last_probe.elapsed() >= self.probe_interval
    }

    /// Returns the MTU size the next probe should attempt.
    pub fn next_probe_size(&self) -> u16 {
        // Midpoint of the current search range
        let mid = self.search_low + (self.search_high - self.search_low) / 2;
        mid.max(self.min_mtu)
    }

    /// Report that a probe at `mtu` succeeded.
    pub fn probe_succeeded(&mut self, mtu: u16) {
        self.last_probe = Instant::now();
        self.current_mtu = mtu;
        self.search_low = mtu;

        if mtu >= self.max_mtu {
            self.current_mtu = self.max_mtu;
            self.probe_state = MtuProbeState::Confirmed;
            info!("MTU discovery confirmed at {} bytes", self.current_mtu);
        } else if self.search_high - self.search_low <= 1 {
            self.probe_state = MtuProbeState::Confirmed;
            info!("MTU discovery converged at {} bytes", self.current_mtu);
        } else {
            debug!("MTU probe succeeded at {}, searching [{}, {}]", mtu, self.search_low, self.search_high);
        }
    }

    /// Report that the current probe failed.
    pub fn probe_failed(&mut self) {
        self.last_probe = Instant::now();
        let mid = self.search_low + (self.search_high - self.search_low) / 2;
        self.search_high = mid;

        if self.search_high <= self.search_low || self.search_high <= self.min_mtu {
            self.current_mtu = self.min_mtu;
            self.probe_state = MtuProbeState::Failed;
            info!("MTU discovery failed, fell back to minimum {} bytes", self.min_mtu);
        } else {
            debug!("MTU probe failed, narrowing to [{}, {}]", self.search_low, self.search_high);
        }
    }

    /// Current best-known path MTU
    pub fn current_mtu(&self) -> u16 {
        self.current_mtu
    }

    /// Current probe state
    pub fn state(&self) -> &MtuProbeState {
        &self.probe_state
    }

    /// Reset to initial searching state
    pub fn reset(&mut self) {
        self.current_mtu = self.min_mtu;
        self.search_low = self.min_mtu;
        self.search_high = self.max_mtu;
        self.probe_state = MtuProbeState::Searching;
        self.last_probe = Instant::now() - self.probe_interval;
    }
}

// ---------------------------------------------------------------------------
// Loss-Based Tier Adjuster
// ---------------------------------------------------------------------------

/// Adjusts tier recommendations based on a sliding window of loss-rate
/// observations. When average loss exceeds `downgrade_threshold` the
/// connection should step down one tier; when it falls below
/// `upgrade_threshold` it may step up.
pub struct LossBasedAdjuster {
    /// Rolling window of loss percentages
    loss_window: VecDeque<f64>,
    /// Maximum window length
    window_size: usize,
    /// Average loss % above which a downgrade is recommended
    downgrade_threshold: f64,
    /// Average loss % below which an upgrade is recommended
    upgrade_threshold: f64,
}

impl LossBasedAdjuster {
    /// Create a new adjuster.
    ///
    /// * `window_size`          – number of samples to average over (default 10)
    /// * `downgrade_threshold`  – loss % triggering downgrade (default 5.0)
    /// * `upgrade_threshold`    – loss % permitting upgrade  (default 0.5)
    pub fn new(window_size: usize, downgrade_threshold: f64, upgrade_threshold: f64) -> Self {
        Self {
            loss_window: VecDeque::with_capacity(window_size),
            window_size: window_size.max(1),
            downgrade_threshold,
            upgrade_threshold,
        }
    }

    /// Record a loss observation (0.0 – 100.0 percent)
    pub fn record_loss(&mut self, loss_pct: f64) {
        if self.loss_window.len() >= self.window_size {
            self.loss_window.pop_front();
        }
        self.loss_window.push_back(loss_pct);
    }

    /// `true` when average loss exceeds the downgrade threshold
    pub fn should_downgrade(&self) -> bool {
        if self.loss_window.is_empty() {
            return false;
        }
        self.average_loss() > self.downgrade_threshold
    }

    /// `true` when average loss is below the upgrade threshold
    pub fn should_upgrade(&self) -> bool {
        if self.loss_window.is_empty() {
            return false;
        }
        self.average_loss() < self.upgrade_threshold
    }

    /// Windowed average loss percentage
    pub fn average_loss(&self) -> f64 {
        if self.loss_window.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.loss_window.iter().sum();
        sum / self.loss_window.len() as f64
    }

    /// Clear all recorded observations
    pub fn reset(&mut self) {
        self.loss_window.clear();
    }
}

// ---------------------------------------------------------------------------
// Congestion-control tier mapping
// ---------------------------------------------------------------------------

/// Returns the recommended congestion control algorithm for the given tier.
///
/// Since Quinn does not support changing CC on live connections this is intended
/// for configuring *new* connections.
pub fn congestion_control_for_tier(tier: &NetworkTier) -> CongestionControl {
    match tier {
        NetworkTier::Performance { .. }
        | NetworkTier::Enterprise { .. }
        | NetworkTier::DataCenter { .. } => CongestionControl::Bbr2,
        NetworkTier::Standard { .. } | NetworkTier::Home { .. } => CongestionControl::Cubic,
        NetworkTier::Slow { .. } => CongestionControl::NewReno,
    }
}

/// Adaptive connection state for live parameter updates
pub struct AdaptiveConnection {
    /// The underlying QUIC connection
    connection: Arc<QuinnConnection>,
    /// Current network tier
    current_tier: Arc<RwLock<NetworkTier>>,
    /// Network condition metrics
    conditions: Arc<RwLock<NetworkConditions>>,
    /// Adaptation enabled flag
    adaptation_enabled: AtomicBool,
    /// Last adaptation time
    last_adaptation: Arc<RwLock<Instant>>,
    /// Adaptation counter
    adaptation_count: AtomicU64,
    /// Connection-specific parameters
    parameters: Arc<RwLock<ConnectionParameters>>,
    /// Hysteresis state to prevent thrashing
    hysteresis: Arc<RwLock<HysteresisState>>,
    /// EWMA bandwidth estimator for smoothed throughput measurements
    bandwidth_estimator: Arc<RwLock<EwmaBandwidthEstimator>>,
    /// MTU path discovery state
    mtu_discovery: Arc<RwLock<MtuDiscovery>>,
    /// Loss-based tier adjustment
    loss_adjuster: Arc<RwLock<LossBasedAdjuster>>,
}

/// Connection-specific parameters that can be adjusted
#[derive(Debug, Clone)]
pub struct ConnectionParameters {
    /// Maximum stream window size
    pub stream_window: u64,
    /// Maximum connection window size
    pub connection_window: u64,
    /// Maximum concurrent streams
    pub max_streams: u32,
    /// Maximum datagram size
    pub max_datagram_size: u16,
    /// Keep-alive interval
    pub keep_alive_interval: Option<Duration>,
    /// Idle timeout
    pub idle_timeout: Duration,
    /// Congestion control algorithm
    pub congestion_control: CongestionControl,
    /// Send buffer size
    pub send_buffer_size: usize,
    /// Receive buffer size
    pub receive_buffer_size: usize,
}

impl Default for ConnectionParameters {
    fn default() -> Self {
        Self {
            stream_window: 16 * 1024 * 1024, // 16MB
            connection_window: 32 * 1024 * 1024, // 32MB
            max_streams: 100,
            max_datagram_size: 1500,
            keep_alive_interval: Some(Duration::from_secs(30)),
            idle_timeout: Duration::from_secs(120),
            congestion_control: CongestionControl::Bbr2,
            send_buffer_size: 8 * 1024 * 1024,
            receive_buffer_size: 8 * 1024 * 1024,
        }
    }
}

/// Hysteresis state to prevent parameter thrashing
#[derive(Debug, Clone)]
struct HysteresisState {
    /// Number of consecutive measurements in same direction
    consecutive_count: u32,
    /// Previous tier for comparison (used in hysteresis decision logic)
    #[allow(dead_code)]
    previous_tier: Option<NetworkTier>,
    /// Timestamp of last tier change
    last_tier_change: Instant,
    /// Minimum time between tier changes
    min_tier_stability: Duration,
    /// Required consecutive measurements for change
    required_consecutive: u32,
}

impl Default for HysteresisState {
    fn default() -> Self {
        Self {
            consecutive_count: 0,
            previous_tier: None,
            last_tier_change: Instant::now(),
            min_tier_stability: Duration::from_secs(5), // 5 second minimum
            required_consecutive: 3, // 3 consecutive measurements
        }
    }
}

impl AdaptiveConnection {
    /// Create a new adaptive connection wrapper
    pub fn new(connection: Arc<QuinnConnection>) -> Self {
        Self::with_config(connection, 0.125, 20, 30, 10, 5.0, 0.5)
    }

    /// Create a new adaptive connection with explicit tuning parameters.
    ///
    /// * `ewma_alpha`           – EWMA smoothing factor
    /// * `ewma_max_samples`     – EWMA sample window size
    /// * `mtu_probe_interval_s` – seconds between MTU probes
    /// * `loss_window_size`     – loss observation window length
    /// * `loss_downgrade_pct`   – average loss % triggering downgrade
    /// * `loss_upgrade_pct`     – average loss % permitting upgrade
    pub fn with_config(
        connection: Arc<QuinnConnection>,
        ewma_alpha: f64,
        ewma_max_samples: usize,
        mtu_probe_interval_s: u64,
        loss_window_size: usize,
        loss_downgrade_pct: f64,
        loss_upgrade_pct: f64,
    ) -> Self {
        let initial_tier = NetworkTier::Standard { gbps: 1.0 };

        Self {
            connection,
            current_tier: Arc::new(RwLock::new(initial_tier)),
            conditions: Arc::new(RwLock::new(NetworkConditions::default())),
            adaptation_enabled: AtomicBool::new(true),
            last_adaptation: Arc::new(RwLock::new(Instant::now())),
            adaptation_count: AtomicU64::new(0),
            parameters: Arc::new(RwLock::new(ConnectionParameters::default())),
            hysteresis: Arc::new(RwLock::new(HysteresisState::default())),
            bandwidth_estimator: Arc::new(RwLock::new(
                EwmaBandwidthEstimator::new(ewma_alpha, ewma_max_samples),
            )),
            mtu_discovery: Arc::new(RwLock::new(
                MtuDiscovery::new(1200, 9000, Duration::from_secs(mtu_probe_interval_s)),
            )),
            loss_adjuster: Arc::new(RwLock::new(
                LossBasedAdjuster::new(loss_window_size, loss_downgrade_pct, loss_upgrade_pct),
            )),
        }
    }

    /// Update network conditions from connection statistics.
    ///
    /// This also feeds the EWMA bandwidth estimator and loss adjuster so
    /// that `detect_tier()` operates on smoothed data.
    pub fn update_conditions(&self) {
        let stats = self.connection.stats();
        let mut conditions = self.conditions.write();

        // Update RTT from path statistics
        let path = stats.path;
        conditions.rtt_ms = path.rtt.as_millis() as f64;

        // Calculate jitter as RTT variance
        if conditions.rtt_ms > 0.0 {
            let prev_rtt = conditions.rtt_ms;
            conditions.jitter_ms = (path.rtt.as_millis() as f64 - prev_rtt).abs();
        }

        // Update packet loss from frame statistics
        let frame_stats = stats.frame_tx;
        let total = frame_stats.acks + frame_stats.stream;
        if total > 0 {
            // Use retransmits as a proxy for loss
            conditions.packet_loss =
                (frame_stats.path_response as f64 / total.max(1) as f64) * 100.0;
        }

        // Update throughput estimate
        let udp_stats = stats.udp_tx;
        let elapsed = conditions.last_update.elapsed();
        let duration_secs = elapsed.as_secs_f64();
        if duration_secs > 0.0 {
            let bytes_per_sec = udp_stats.bytes as f64 / duration_secs;
            conditions.throughput_mbps = (bytes_per_sec * 8.0) / 1_000_000.0;

            // Feed EWMA estimator with the raw transfer observation
            self.bandwidth_estimator
                .write()
                .add_sample(udp_stats.bytes, elapsed);
        }

        // Feed loss adjuster
        self.loss_adjuster.write().record_loss(conditions.packet_loss);

        // Track retransmissions (using datagrams as proxy)
        conditions.retransmissions = udp_stats.datagrams;

        // Update the EWMA-smoothed bandwidth estimate in conditions
        let smoothed_mbps = self.bandwidth_estimator.read().estimate_bps() / 1_000_000.0;
        if smoothed_mbps > 0.0 {
            conditions.bandwidth_estimate = smoothed_mbps;
        }

        conditions.last_update = Instant::now();

        debug!(
            "Updated network conditions: RTT={:.2}ms, loss={:.2}%, throughput={:.2}Mbps, \
             ewma={:.2}Mbps",
            conditions.rtt_ms,
            conditions.packet_loss,
            conditions.throughput_mbps,
            smoothed_mbps,
        );
    }

    /// Detect network tier based on current conditions.
    ///
    /// Tier detection uses the EWMA-smoothed bandwidth estimate rather than
    /// raw throughput samples, and the loss-based adjuster can cap or boost
    /// the result by one tier level.
    pub fn detect_tier(&self) -> NetworkTier {
        let conditions = self.conditions.read();

        // Prefer EWMA-smoothed estimate when available
        let ewma_gbps = self.bandwidth_estimator.read().estimate_gbps();
        let mut estimated_gbps = if ewma_gbps > 0.0 {
            ewma_gbps
        } else {
            // Fallback: blend stored estimate with raw throughput
            let base = conditions.bandwidth_estimate / 1000.0;
            if conditions.throughput_mbps > 0.0 {
                (base + (conditions.throughput_mbps / 1000.0)) / 2.0
            } else {
                base
            }
        };

        // Penalize for high latency
        if conditions.rtt_ms > 100.0 {
            estimated_gbps *= 0.5; // Satellite/intercontinental
        } else if conditions.rtt_ms > 50.0 {
            estimated_gbps *= 0.7; // WAN
        } else if conditions.rtt_ms > 20.0 {
            estimated_gbps *= 0.9; // Metro
        }

        // Penalize for packet loss
        if conditions.packet_loss > 5.0 {
            estimated_gbps *= 0.3;
        } else if conditions.packet_loss > 2.0 {
            estimated_gbps *= 0.5;
        } else if conditions.packet_loss > 0.5 {
            estimated_gbps *= 0.8;
        }

        // Penalize for high jitter
        if conditions.jitter_ms > 20.0 {
            estimated_gbps *= 0.7;
        }

        let mut tier = NetworkTier::from_gbps(estimated_gbps);

        // Apply loss-adjuster influence: cap or boost by one tier level
        let loss = self.loss_adjuster.read();
        if loss.should_downgrade() {
            tier = Self::tier_step_down(&tier);
            debug!("Loss adjuster capped tier down (avg loss {:.1}%)", loss.average_loss());
        } else if loss.should_upgrade() {
            tier = Self::tier_step_up(&tier);
            debug!("Loss adjuster boosted tier up (avg loss {:.1}%)", loss.average_loss());
        }

        tier
    }

    /// Step a tier down by one level (for loss-adjuster downgrade)
    fn tier_step_down(tier: &NetworkTier) -> NetworkTier {
        match tier {
            NetworkTier::DataCenter { .. } => NetworkTier::Enterprise { gbps: 10.0 },
            NetworkTier::Enterprise { .. } => NetworkTier::Performance { gbps: 2.5 },
            NetworkTier::Performance { .. } => NetworkTier::Standard { gbps: 1.0 },
            NetworkTier::Standard { .. } => NetworkTier::Home { mbps: 100.0 },
            NetworkTier::Home { .. } | NetworkTier::Slow { .. } => NetworkTier::Slow { mbps: 10.0 },
        }
    }

    /// Step a tier up by one level (for loss-adjuster upgrade)
    fn tier_step_up(tier: &NetworkTier) -> NetworkTier {
        match tier {
            NetworkTier::Slow { .. } => NetworkTier::Home { mbps: 100.0 },
            NetworkTier::Home { .. } => NetworkTier::Standard { gbps: 1.0 },
            NetworkTier::Standard { .. } => NetworkTier::Performance { gbps: 2.5 },
            NetworkTier::Performance { .. } => NetworkTier::Enterprise { gbps: 10.0 },
            NetworkTier::Enterprise { .. } | NetworkTier::DataCenter { .. } => {
                NetworkTier::DataCenter { gbps: 25.0 }
            }
        }
    }

    /// Check if adaptation should trigger based on hysteresis
    fn should_adapt(&self, new_tier: &NetworkTier) -> bool {
        let mut hysteresis = self.hysteresis.write();
        let current_tier = self.current_tier.read();

        // Check if tier is different
        let tier_changed = !Self::tiers_equal(&*current_tier, new_tier);

        if !tier_changed {
            // Reset consecutive count if tier is stable
            hysteresis.consecutive_count = 0;
            return false;
        }

        // Check minimum stability time
        if hysteresis.last_tier_change.elapsed() < hysteresis.min_tier_stability {
            trace!("Skipping adaptation: minimum stability time not met");
            return false;
        }

        // Increment consecutive count
        hysteresis.consecutive_count += 1;

        // Check if we have enough consecutive measurements
        if hysteresis.consecutive_count >= hysteresis.required_consecutive {
            hysteresis.consecutive_count = 0;
            hysteresis.last_tier_change = Instant::now();
            true
        } else {
            trace!(
                "Hysteresis: {}/{} consecutive measurements for tier change",
                hysteresis.consecutive_count, hysteresis.required_consecutive
            );
            false
        }
    }

    /// Compare two network tiers for equality
    fn tiers_equal(a: &NetworkTier, b: &NetworkTier) -> bool {
        std::mem::discriminant(a) == std::mem::discriminant(b)
    }

    /// Adapt connection parameters based on network conditions
    pub async fn adapt(&self) -> Result<(), anyhow::Error> {
        if !self.adaptation_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Update conditions from connection stats
        self.update_conditions();

        // Detect current network tier
        let detected_tier = self.detect_tier();

        // Check hysteresis before adapting
        if !self.should_adapt(&detected_tier) {
            return Ok(());
        }

        // Update tier
        {
            let mut current = self.current_tier.write();
            *current = detected_tier.clone();
        }

        // Apply tier-specific parameters
        self.apply_tier_parameters(&detected_tier)?;

        // Update adaptation metadata
        *self.last_adaptation.write() = Instant::now();
        self.adaptation_count.fetch_add(1, Ordering::Relaxed);

        info!(
            "Connection adapted to {:?} (adaptation #{})",
            detected_tier,
            self.adaptation_count.load(Ordering::Relaxed)
        );

        Ok(())
    }

    /// Apply tier-specific parameters to the connection
    fn apply_tier_parameters(&self, tier: &NetworkTier) -> Result<(), anyhow::Error> {
        let mut params = self.parameters.write();

        match tier {
            NetworkTier::Slow { mbps } => {
                // Conservative parameters for slow networks
                params.stream_window = 256 * 1024; // 256KB
                params.connection_window = 512 * 1024; // 512KB
                params.max_streams = 10;
                params.max_datagram_size = 1200;
                params.keep_alive_interval = Some(Duration::from_secs(60));
                params.idle_timeout = Duration::from_secs(300);
                params.congestion_control = CongestionControl::NewReno;
                params.send_buffer_size = 128 * 1024;
                params.receive_buffer_size = 128 * 1024;

                debug!("Applied slow network parameters ({}Mbps)", mbps);
            }
            NetworkTier::Home { mbps } => {
                // Home broadband parameters
                params.stream_window = 2 * 1024 * 1024; // 2MB
                params.connection_window = 4 * 1024 * 1024; // 4MB
                params.max_streams = 50;
                params.max_datagram_size = 1500;
                params.keep_alive_interval = Some(Duration::from_secs(45));
                params.idle_timeout = Duration::from_secs(180);
                params.congestion_control = CongestionControl::Cubic;
                params.send_buffer_size = 1024 * 1024;
                params.receive_buffer_size = 1024 * 1024;

                debug!("Applied home network parameters ({}Mbps)", mbps);
            }
            NetworkTier::Standard { gbps } => {
                // Standard gigabit parameters
                params.stream_window = 8 * 1024 * 1024; // 8MB
                params.connection_window = 16 * 1024 * 1024; // 16MB
                params.max_streams = 100;
                params.max_datagram_size = 9000; // Jumbo frames
                params.keep_alive_interval = Some(Duration::from_secs(30));
                params.idle_timeout = Duration::from_secs(120);
                params.congestion_control = CongestionControl::Bbr2;
                params.send_buffer_size = 4 * 1024 * 1024;
                params.receive_buffer_size = 4 * 1024 * 1024;

                debug!("Applied standard gigabit parameters ({}Gbps)", gbps);
            }
            NetworkTier::Performance { gbps } => {
                // Performance network parameters
                params.stream_window = 16 * 1024 * 1024; // 16MB
                params.connection_window = 32 * 1024 * 1024; // 32MB
                params.max_streams = 200;
                params.max_datagram_size = 9000;
                params.keep_alive_interval = Some(Duration::from_secs(20));
                params.idle_timeout = Duration::from_secs(90);
                params.congestion_control = CongestionControl::Bbr2;
                params.send_buffer_size = 8 * 1024 * 1024;
                params.receive_buffer_size = 8 * 1024 * 1024;

                debug!("Applied performance network parameters ({}Gbps)", gbps);
            }
            NetworkTier::Enterprise { gbps } | NetworkTier::DataCenter { gbps } => {
                // Maximum performance parameters
                params.stream_window = 32 * 1024 * 1024; // 32MB
                params.connection_window = 64 * 1024 * 1024; // 64MB
                params.max_streams = 1000;
                params.max_datagram_size = 9000;
                params.keep_alive_interval = Some(Duration::from_secs(10));
                params.idle_timeout = Duration::from_secs(60);
                params.congestion_control = CongestionControl::Bbr2;
                params.send_buffer_size = 16 * 1024 * 1024;
                params.receive_buffer_size = 16 * 1024 * 1024;

                debug!("Applied data center parameters ({}Gbps)", gbps);
            }
        }

        // Apply parameters to the actual connection
        self.apply_to_connection(&params)?;

        Ok(())
    }

    /// Apply parameters to the underlying QUIC connection
    fn apply_to_connection(&self, params: &ConnectionParameters) -> Result<(), anyhow::Error> {
        // Create new transport config with updated parameters
        let mut transport_config = TransportConfig::default();

        // Set flow control windows
        transport_config.stream_receive_window(VarInt::from_u64(params.stream_window)?);
        transport_config.receive_window(VarInt::from_u64(params.connection_window)?);

        // Set stream limits
        transport_config.max_concurrent_bidi_streams(VarInt::from_u32(params.max_streams));
        transport_config.max_concurrent_uni_streams(VarInt::from_u32(params.max_streams / 2));

        // Use MTU from discovery when it has a confirmed or in-progress result;
        // otherwise fall back to the tier's datagram size.
        let discovered_mtu = self.mtu_discovery.read().current_mtu();
        let mtu = if discovered_mtu > params.max_datagram_size {
            discovered_mtu
        } else {
            params.max_datagram_size
        };
        transport_config.initial_mtu(mtu);

        // Set timeouts
        transport_config.max_idle_timeout(Some(params.idle_timeout.try_into()?));
        if let Some(keep_alive) = params.keep_alive_interval {
            transport_config.keep_alive_interval(Some(keep_alive));
        }

        // Congestion control cannot be changed on live connections; log intent
        // and expose via `recommended_congestion_control()` for new connections.
        debug!(
            "Congestion control for tier: {:?} (applies to new connections only)",
            params.congestion_control
        );

        // Update what Quinn allows on a live connection
        self.connection.set_max_concurrent_bi_streams(VarInt::from_u32(params.max_streams));
        self.connection.set_max_concurrent_uni_streams(VarInt::from_u32(params.max_streams / 2));

        Ok(())
    }

    /// Returns the congestion control algorithm recommended for the current
    /// detected tier. Useful when creating new connections that should match
    /// the observed network conditions.
    pub fn recommended_congestion_control(&self) -> CongestionControl {
        let tier = self.current_tier.read();
        congestion_control_for_tier(&tier)
    }

    /// Borrow the EWMA bandwidth estimator (read-only)
    pub fn bandwidth_estimator(&self) -> parking_lot::RwLockReadGuard<'_, EwmaBandwidthEstimator> {
        self.bandwidth_estimator.read()
    }

    /// Borrow the MTU discovery state (read-only)
    pub fn mtu_discovery(&self) -> parking_lot::RwLockReadGuard<'_, MtuDiscovery> {
        self.mtu_discovery.read()
    }

    /// Borrow the loss adjuster (read-only)
    pub fn loss_adjuster(&self) -> parking_lot::RwLockReadGuard<'_, LossBasedAdjuster> {
        self.loss_adjuster.read()
    }

    /// Enable or disable adaptation
    pub fn set_adaptation_enabled(&self, enabled: bool) {
        self.adaptation_enabled.store(enabled, Ordering::Relaxed);
        if enabled {
            info!("Adaptive optimization enabled for connection");
        } else {
            info!("Adaptive optimization disabled for connection");
        }
    }

    /// Get current network tier
    pub fn current_tier(&self) -> NetworkTier {
        self.current_tier.read().clone()
    }

    /// Get current network conditions
    pub fn conditions(&self) -> NetworkConditions {
        self.conditions.read().clone()
    }

    /// Get current connection parameters
    pub fn parameters(&self) -> ConnectionParameters {
        self.parameters.read().clone()
    }

    /// Get adaptation statistics
    pub fn adaptation_stats(&self) -> AdaptationStats {
        AdaptationStats {
            adaptation_count: self.adaptation_count.load(Ordering::Relaxed),
            last_adaptation: *self.last_adaptation.read(),
            current_tier: self.current_tier.read().clone(),
            enabled: self.adaptation_enabled.load(Ordering::Relaxed),
        }
    }

    /// Force immediate adaptation (bypasses hysteresis)
    pub async fn force_adapt(&self) -> Result<(), anyhow::Error> {
        // Clear hysteresis state
        {
            let mut hysteresis = self.hysteresis.write();
            hysteresis.consecutive_count = hysteresis.required_consecutive;
        }

        // Run adaptation
        self.adapt().await
    }
}

/// Statistics about connection adaptation
#[derive(Debug, Clone)]
pub struct AdaptationStats {
    pub adaptation_count: u64,
    pub last_adaptation: Instant,
    pub current_tier: NetworkTier,
    pub enabled: bool,
}

/// Adaptation manager for all connections
pub struct AdaptationManager {
    /// All adaptive connections
    connections: Arc<dashmap::DashMap<String, Arc<AdaptiveConnection>>>,
    /// Global adaptation enabled flag
    enabled: AtomicBool,
    /// Adaptation interval
    adaptation_interval: Duration,
}

impl AdaptationManager {
    /// Create new adaptation manager
    pub fn new(adaptation_interval: Duration) -> Self {
        Self {
            connections: Arc::new(dashmap::DashMap::new()),
            enabled: AtomicBool::new(true),
            adaptation_interval,
        }
    }

    /// Register a connection for adaptive optimization
    pub fn register_connection(&self, id: String, connection: Arc<QuinnConnection>) {
        let adaptive = Arc::new(AdaptiveConnection::new(connection));
        self.connections.insert(id, adaptive);
    }

    /// Unregister a connection
    pub fn unregister_connection(&self, id: &str) {
        self.connections.remove(id);
    }

    /// Start the adaptation loop
    pub async fn start(self: Arc<Self>) {
        let mut ticker = interval(self.adaptation_interval);

        loop {
            ticker.tick().await;

            if !self.enabled.load(Ordering::Relaxed) {
                continue;
            }

            // Adapt all connections
            for entry in self.connections.iter() {
                let connection = entry.value().clone();

                // Spawn adaptation as a separate task to avoid blocking
                tokio::spawn(async move {
                    if let Err(e) = connection.adapt().await {
                        warn!("Failed to adapt connection: {}", e);
                    }
                });
            }
        }
    }

    /// Get an adaptive connection by ID
    pub fn get_connection(&self, id: &str) -> Option<Arc<AdaptiveConnection>> {
        self.connections.get(id).map(|entry| entry.clone())
    }

    /// Enable or disable global adaptation
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Get all connection IDs
    pub fn connection_ids(&self) -> Vec<String> {
        self.connections.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get adaptation statistics for all connections
    pub fn all_stats(&self) -> Vec<(String, AdaptationStats)> {
        self.connections
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().adaptation_stats()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Existing tests (preserved)
    // -----------------------------------------------------------------------

    #[test]
    fn test_network_tier_detection() {
        let _conditions = NetworkConditions {
            rtt_ms: 5.0,
            packet_loss: 0.1,
            throughput_mbps: 2500.0,
            bandwidth_estimate: 10000.0,
            retransmissions: 10,
            jitter_ms: 1.0,
            last_update: Instant::now(),
        };
        // Should detect as Performance tier based on conditions
    }

    #[test]
    fn test_hysteresis_prevents_thrashing() {
        let mut hysteresis = HysteresisState::default();
        hysteresis.required_consecutive = 3;
        assert_eq!(hysteresis.consecutive_count, 0);
    }

    #[test]
    fn test_parameter_application() {
        let params = ConnectionParameters {
            stream_window: 16 * 1024 * 1024,
            connection_window: 32 * 1024 * 1024,
            max_streams: 100,
            max_datagram_size: 9000,
            keep_alive_interval: Some(Duration::from_secs(30)),
            idle_timeout: Duration::from_secs(120),
            congestion_control: CongestionControl::Bbr2,
            send_buffer_size: 8 * 1024 * 1024,
            receive_buffer_size: 8 * 1024 * 1024,
        };
        assert!(params.stream_window > 0);
        assert!(params.max_streams > 0);
    }

    // -----------------------------------------------------------------------
    // EWMA Bandwidth Estimator
    // -----------------------------------------------------------------------

    #[test]
    fn test_ewma_convergence() {
        let mut est = EwmaBandwidthEstimator::new(0.125, 20);

        // Feed 10 samples at ~1 Gbps (125 MB in 1 second each)
        let bytes_per_sample: u64 = 125_000_000;
        let dur = Duration::from_secs(1);
        for _ in 0..10 {
            est.add_sample(bytes_per_sample, dur);
        }

        let estimate_gbps = est.estimate_gbps();
        // Converging toward 1.0 Gbps; after 10 samples with alpha=0.125
        // the estimate should be within 30% of target.
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

        // Window is 5, so only last 5 samples retained
        assert_eq!(est.sample_count(), 5);
    }

    #[test]
    fn test_ewma_zero_duration() {
        let mut est = EwmaBandwidthEstimator::new(0.125, 20);

        // Zero-duration sample must not panic and must not change estimate
        est.add_sample(1_000_000, Duration::ZERO);
        assert_eq!(est.sample_count(), 0);
        assert!((est.estimate_bps() - 0.0).abs() < f64::EPSILON);

        // Zero-byte sample also skipped
        est.add_sample(0, Duration::from_secs(1));
        assert_eq!(est.sample_count(), 0);
    }

    // -----------------------------------------------------------------------
    // MTU Discovery
    // -----------------------------------------------------------------------

    #[test]
    fn test_mtu_probe_search_confirm() {
        let mut mtu = MtuDiscovery::new(1200, 9000, Duration::from_millis(0));

        // Repeatedly succeed until confirmed at max
        for _ in 0..20 {
            if *mtu.state() == MtuProbeState::Confirmed {
                break;
            }
            let probe = mtu.next_probe_size();
            mtu.probe_succeeded(probe);
        }

        assert_eq!(*mtu.state(), MtuProbeState::Confirmed);
        // Final MTU should be at or near max
        assert!(
            mtu.current_mtu() >= 8900,
            "expected near-max MTU, got {}",
            mtu.current_mtu()
        );
    }

    #[test]
    fn test_mtu_probe_search_fail() {
        let mut mtu = MtuDiscovery::new(1200, 9000, Duration::from_millis(0));

        // Repeatedly fail until we settle at min
        for _ in 0..20 {
            if *mtu.state() == MtuProbeState::Failed {
                break;
            }
            mtu.probe_failed();
        }

        assert_eq!(*mtu.state(), MtuProbeState::Failed);
        assert_eq!(mtu.current_mtu(), 1200);
    }

    #[test]
    fn test_mtu_probe_binary_search() {
        let mut mtu = MtuDiscovery::new(1200, 9000, Duration::from_millis(0));

        // Succeed at the first midpoint
        let first_mid = mtu.next_probe_size();
        mtu.probe_succeeded(first_mid);
        assert!(mtu.current_mtu() >= 1200);

        // Now fail — search should narrow downward from the high side
        let second_mid = mtu.next_probe_size();
        assert!(second_mid > first_mid, "next probe should be higher");
        mtu.probe_failed();

        // The search range narrowed; next probe should be between first_mid and second_mid
        let third_probe = mtu.next_probe_size();
        assert!(
            third_probe >= first_mid && third_probe <= second_mid,
            "binary search should narrow: {} in [{}, {}]",
            third_probe, first_mid, second_mid
        );
    }

    // -----------------------------------------------------------------------
    // Loss-Based Adjuster
    // -----------------------------------------------------------------------

    #[test]
    fn test_loss_downgrade_threshold() {
        let mut adj = LossBasedAdjuster::new(10, 5.0, 0.5);

        // Record 10 samples all above 5%
        for _ in 0..10 {
            adj.record_loss(7.0);
        }

        assert!(adj.should_downgrade(), "average 7% > threshold 5%");
        assert!(!adj.should_upgrade());
    }

    #[test]
    fn test_loss_upgrade_threshold() {
        let mut adj = LossBasedAdjuster::new(10, 5.0, 0.5);

        // Record 10 samples all below 0.5%
        for _ in 0..10 {
            adj.record_loss(0.1);
        }

        assert!(adj.should_upgrade(), "average 0.1% < threshold 0.5%");
        assert!(!adj.should_downgrade());
    }

    #[test]
    fn test_loss_mixed() {
        let mut adj = LossBasedAdjuster::new(10, 5.0, 0.5);

        // Mix of high and low — average = 3.0%
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
        // 3.0 is between thresholds: neither downgrade nor upgrade
        assert!(!adj.should_downgrade());
        assert!(!adj.should_upgrade());
    }

    // -----------------------------------------------------------------------
    // Congestion control tier mapping
    // -----------------------------------------------------------------------

    #[test]
    fn test_cc_tier_mapping() {
        assert!(matches!(
            congestion_control_for_tier(&NetworkTier::DataCenter { gbps: 40.0 }),
            CongestionControl::Bbr2
        ));
        assert!(matches!(
            congestion_control_for_tier(&NetworkTier::Standard { gbps: 1.0 }),
            CongestionControl::Cubic
        ));
        assert!(matches!(
            congestion_control_for_tier(&NetworkTier::Slow { mbps: 10.0 }),
            CongestionControl::NewReno
        ));
    }

    // -----------------------------------------------------------------------
    // Integrated: detect_tier with EWMA + loss adjuster
    // -----------------------------------------------------------------------

    #[test]
    fn test_adaptive_tier_with_ewma() {
        // We cannot construct a real QuinnConnection in unit tests, so we
        // exercise the sub-components directly through the same logic path.

        // 1. Seed EWMA at ~2.5 Gbps (Performance tier boundary)
        let mut est = EwmaBandwidthEstimator::new(0.125, 20);
        let bytes = 312_500_000u64; // 2.5 Gbps = 312.5 MB/s
        let dur = Duration::from_secs(1);
        for _ in 0..20 {
            est.add_sample(bytes, dur);
        }
        let gbps = est.estimate_gbps();
        assert!(
            gbps > 2.0 && gbps < 3.0,
            "expected ~2.5 Gbps, got {gbps:.3}"
        );

        // 2. Tier from EWMA alone should be Performance
        let tier = NetworkTier::from_gbps(gbps);
        assert!(
            matches!(tier, NetworkTier::Performance { .. }),
            "expected Performance, got {tier:?}"
        );

        // 3. Loss adjuster with high loss should recommend downgrade
        let mut adj = LossBasedAdjuster::new(10, 5.0, 0.5);
        for _ in 0..10 {
            adj.record_loss(8.0);
        }
        assert!(adj.should_downgrade());

        // Simulate the tier_step_down call from detect_tier
        let downgraded = AdaptiveConnection::tier_step_down(&tier);
        assert!(
            matches!(downgraded, NetworkTier::Standard { .. }),
            "expected Standard after downgrade, got {downgraded:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Tier stepping helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_tier_step_down_floor() {
        // Stepping down from Slow should stay Slow
        let slow = NetworkTier::Slow { mbps: 10.0 };
        let result = AdaptiveConnection::tier_step_down(&slow);
        assert!(matches!(result, NetworkTier::Slow { .. }));
    }

    #[test]
    fn test_tier_step_up_ceiling() {
        // Stepping up from DataCenter should stay DataCenter
        let dc = NetworkTier::DataCenter { gbps: 40.0 };
        let result = AdaptiveConnection::tier_step_up(&dc);
        assert!(matches!(result, NetworkTier::DataCenter { .. }));
    }
}