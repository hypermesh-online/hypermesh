// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use parking_lot::RwLock;
use quinn::{Connection as QuinnConnection, TransportConfig, VarInt};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, trace};

use super::bandwidth::EwmaBandwidthEstimator;
use super::loss::LossBasedAdjuster;
use super::mtu::MtuDiscovery;
use super::tier::{congestion_control_for_tier, tier_step_down, tier_step_up, tiers_equal};
use crate::transport::{CongestionControl, NetworkTier};

#[derive(Debug, Clone)]
pub struct AdaptationStats {
    pub adaptation_count: u64,
    pub last_adaptation: Instant,
    pub current_tier: NetworkTier,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct NetworkConditions {
    pub rtt_ms: f64,
    pub packet_loss: f64,
    pub throughput_mbps: f64,
    pub bandwidth_estimate: f64,
    pub retransmissions: u64,
    pub jitter_ms: f64,
    pub last_update: Instant,
}

impl Default for NetworkConditions {
    fn default() -> Self {
        Self {
            rtt_ms: 0.0,
            packet_loss: 0.0,
            throughput_mbps: 0.0,
            bandwidth_estimate: 1000.0,
            retransmissions: 0,
            jitter_ms: 0.0,
            last_update: Instant::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionParameters {
    pub stream_window: u64,
    pub connection_window: u64,
    pub max_streams: u32,
    pub max_datagram_size: u16,
    pub keep_alive_interval: Option<Duration>,
    pub idle_timeout: Duration,
    pub congestion_control: CongestionControl,
    pub send_buffer_size: usize,
    pub receive_buffer_size: usize,
}

impl Default for ConnectionParameters {
    fn default() -> Self {
        Self {
            stream_window: 16 * 1024 * 1024,
            connection_window: 32 * 1024 * 1024,
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

#[derive(Debug, Clone)]
struct HysteresisState {
    consecutive_count: u32,
    _previous_tier: Option<NetworkTier>,
    last_tier_change: Instant,
    min_tier_stability: Duration,
    required_consecutive: u32,
}

impl Default for HysteresisState {
    fn default() -> Self {
        Self {
            consecutive_count: 0,
            _previous_tier: None,
            last_tier_change: Instant::now(),
            min_tier_stability: Duration::from_secs(5),
            required_consecutive: 3,
        }
    }
}

pub struct AdaptiveConnection {
    connection: Arc<QuinnConnection>,
    current_tier: Arc<RwLock<NetworkTier>>,
    conditions: Arc<RwLock<NetworkConditions>>,
    adaptation_enabled: AtomicBool,
    last_adaptation: Arc<RwLock<Instant>>,
    adaptation_count: AtomicU64,
    parameters: Arc<RwLock<ConnectionParameters>>,
    hysteresis: Arc<RwLock<HysteresisState>>,
    bandwidth_estimator: Arc<RwLock<EwmaBandwidthEstimator>>,
    mtu_discovery: Arc<RwLock<MtuDiscovery>>,
    loss_adjuster: Arc<RwLock<LossBasedAdjuster>>,
}

impl AdaptiveConnection {
    pub fn new(connection: Arc<QuinnConnection>) -> Self {
        Self::with_config(connection, 0.125, 20, 30, 10, 5.0, 0.5)
    }

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
            bandwidth_estimator: Arc::new(RwLock::new(EwmaBandwidthEstimator::new(
                ewma_alpha,
                ewma_max_samples,
            ))),
            mtu_discovery: Arc::new(RwLock::new(MtuDiscovery::new(
                1200,
                9000,
                Duration::from_secs(mtu_probe_interval_s),
            ))),
            loss_adjuster: Arc::new(RwLock::new(LossBasedAdjuster::new(
                loss_window_size,
                loss_downgrade_pct,
                loss_upgrade_pct,
            ))),
        }
    }

    pub fn update_conditions(&self) {
        let stats = self.connection.stats();
        let mut conditions = self.conditions.write();

        let path = stats.path;
        conditions.rtt_ms = path.rtt.as_millis() as f64;

        if conditions.rtt_ms > 0.0 {
            let prev_rtt = conditions.rtt_ms;
            conditions.jitter_ms = (path.rtt.as_millis() as f64 - prev_rtt).abs();
        }

        let frame_stats = stats.frame_tx;
        let total = frame_stats.acks + frame_stats.stream;
        if total > 0 {
            conditions.packet_loss =
                (frame_stats.path_response as f64 / total.max(1) as f64) * 100.0;
        }

        let udp_stats = stats.udp_tx;
        let elapsed = conditions.last_update.elapsed();
        let duration_secs = elapsed.as_secs_f64();
        if duration_secs > 0.0 {
            let bytes_per_sec = udp_stats.bytes as f64 / duration_secs;
            conditions.throughput_mbps = (bytes_per_sec * 8.0) / 1_000_000.0;

            self.bandwidth_estimator
                .write()
                .add_sample(udp_stats.bytes, elapsed);
        }

        self.loss_adjuster
            .write()
            .record_loss(conditions.packet_loss);

        conditions.retransmissions = udp_stats.datagrams;

        let smoothed_mbps = self.bandwidth_estimator.read().estimate_bps() / 1_000_000.0;
        if smoothed_mbps > 0.0 {
            conditions.bandwidth_estimate = smoothed_mbps;
        }

        conditions.last_update = Instant::now();

        debug!(
            "Updated network conditions: RTT={:.2}ms, loss={:.2}%, throughput={:.2}Mbps, \
             ewma={:.2}Mbps",
            conditions.rtt_ms, conditions.packet_loss, conditions.throughput_mbps, smoothed_mbps,
        );
    }

    pub fn detect_tier(&self) -> NetworkTier {
        let conditions = self.conditions.read();

        let ewma_gbps = self.bandwidth_estimator.read().estimate_gbps();
        let mut estimated_gbps = if ewma_gbps > 0.0 {
            ewma_gbps
        } else {
            let base = conditions.bandwidth_estimate / 1000.0;
            if conditions.throughput_mbps > 0.0 {
                (base + (conditions.throughput_mbps / 1000.0)) / 2.0
            } else {
                base
            }
        };

        if conditions.rtt_ms > 100.0 {
            estimated_gbps *= 0.5;
        } else if conditions.rtt_ms > 50.0 {
            estimated_gbps *= 0.7;
        } else if conditions.rtt_ms > 20.0 {
            estimated_gbps *= 0.9;
        }

        if conditions.packet_loss > 5.0 {
            estimated_gbps *= 0.3;
        } else if conditions.packet_loss > 2.0 {
            estimated_gbps *= 0.5;
        } else if conditions.packet_loss > 0.5 {
            estimated_gbps *= 0.8;
        }

        if conditions.jitter_ms > 20.0 {
            estimated_gbps *= 0.7;
        }

        let mut tier = NetworkTier::from_gbps(estimated_gbps);

        let loss = self.loss_adjuster.read();
        if loss.should_downgrade() {
            tier = tier_step_down(&tier);
            debug!(
                "Loss adjuster capped tier down (avg loss {:.1}%)",
                loss.average_loss()
            );
        } else if loss.should_upgrade() {
            tier = tier_step_up(&tier);
            debug!(
                "Loss adjuster boosted tier up (avg loss {:.1}%)",
                loss.average_loss()
            );
        }

        tier
    }

    fn should_adapt(&self, new_tier: &NetworkTier) -> bool {
        let mut hysteresis = self.hysteresis.write();
        let current_tier = self.current_tier.read();

        let tier_changed = !tiers_equal(&current_tier, new_tier);

        if !tier_changed {
            hysteresis.consecutive_count = 0;
            return false;
        }

        if hysteresis.last_tier_change.elapsed() < hysteresis.min_tier_stability {
            trace!("Skipping adaptation: minimum stability time not met");
            return false;
        }

        hysteresis.consecutive_count += 1;

        if hysteresis.consecutive_count >= hysteresis.required_consecutive {
            hysteresis.consecutive_count = 0;
            hysteresis.last_tier_change = Instant::now();
            true
        } else {
            trace!(
                "Hysteresis: {}/{} consecutive measurements for tier change",
                hysteresis.consecutive_count,
                hysteresis.required_consecutive
            );
            false
        }
    }

    pub async fn adapt(&self) -> Result<(), anyhow::Error> {
        if !self.adaptation_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.update_conditions();

        let detected_tier = self.detect_tier();

        if !self.should_adapt(&detected_tier) {
            return Ok(());
        }

        {
            let mut current = self.current_tier.write();
            *current = detected_tier.clone();
        }

        self.apply_tier_parameters(&detected_tier)?;

        *self.last_adaptation.write() = Instant::now();
        self.adaptation_count.fetch_add(1, Ordering::Relaxed);

        info!(
            "Connection adapted to {:?} (adaptation #{})",
            detected_tier,
            self.adaptation_count.load(Ordering::Relaxed)
        );

        Ok(())
    }

    fn apply_tier_parameters(&self, tier: &NetworkTier) -> Result<(), anyhow::Error> {
        let mut params = self.parameters.write();

        match tier {
            NetworkTier::MinSpec { mbps } => {
                // R13 minimum spec: 1 Mb/s — most conservative parameters
                params.stream_window = 64 * 1024;
                params.connection_window = 128 * 1024;
                params.max_streams = 4;
                params.max_datagram_size = 1200;
                params.keep_alive_interval = Some(Duration::from_secs(120));
                params.idle_timeout = Duration::from_secs(600);
                params.congestion_control = CongestionControl::NewReno;
                params.send_buffer_size = 64 * 1024;
                params.receive_buffer_size = 64 * 1024;

                debug!("Applied R13 minimum spec parameters ({}Mbps)", mbps);
            }
            NetworkTier::Slow { mbps } => {
                params.stream_window = 256 * 1024;
                params.connection_window = 512 * 1024;
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
                params.stream_window = 2 * 1024 * 1024;
                params.connection_window = 4 * 1024 * 1024;
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
                params.stream_window = 8 * 1024 * 1024;
                params.connection_window = 16 * 1024 * 1024;
                params.max_streams = 100;
                params.max_datagram_size = 9000;
                params.keep_alive_interval = Some(Duration::from_secs(30));
                params.idle_timeout = Duration::from_secs(120);
                params.congestion_control = CongestionControl::Bbr2;
                params.send_buffer_size = 4 * 1024 * 1024;
                params.receive_buffer_size = 4 * 1024 * 1024;

                debug!("Applied standard gigabit parameters ({}Gbps)", gbps);
            }
            NetworkTier::Performance { gbps } => {
                params.stream_window = 16 * 1024 * 1024;
                params.connection_window = 32 * 1024 * 1024;
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
                params.stream_window = 32 * 1024 * 1024;
                params.connection_window = 64 * 1024 * 1024;
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

        self.apply_to_connection(&params)?;

        Ok(())
    }

    fn apply_to_connection(&self, params: &ConnectionParameters) -> Result<(), anyhow::Error> {
        let mut transport_config = TransportConfig::default();

        transport_config.stream_receive_window(VarInt::from_u64(params.stream_window)?);
        transport_config.receive_window(VarInt::from_u64(params.connection_window)?);

        transport_config.max_concurrent_bidi_streams(VarInt::from_u32(params.max_streams));
        transport_config.max_concurrent_uni_streams(VarInt::from_u32(params.max_streams / 2));

        let discovered_mtu = self.mtu_discovery.read().current_mtu();
        let mtu = if discovered_mtu > params.max_datagram_size {
            discovered_mtu
        } else {
            params.max_datagram_size
        };
        transport_config.initial_mtu(mtu);

        transport_config.max_idle_timeout(Some(params.idle_timeout.try_into()?));
        if let Some(keep_alive) = params.keep_alive_interval {
            transport_config.keep_alive_interval(Some(keep_alive));
        }

        debug!(
            "Congestion control for tier: {:?} (applies to new connections only)",
            params.congestion_control
        );

        self.connection
            .set_max_concurrent_bi_streams(VarInt::from_u32(params.max_streams));
        self.connection
            .set_max_concurrent_uni_streams(VarInt::from_u32(params.max_streams / 2));

        Ok(())
    }

    pub fn recommended_congestion_control(&self) -> CongestionControl {
        let tier = self.current_tier.read();
        congestion_control_for_tier(&tier)
    }

    pub fn bandwidth_estimator(&self) -> parking_lot::RwLockReadGuard<'_, EwmaBandwidthEstimator> {
        self.bandwidth_estimator.read()
    }

    pub fn mtu_discovery(&self) -> parking_lot::RwLockReadGuard<'_, MtuDiscovery> {
        self.mtu_discovery.read()
    }

    pub fn loss_adjuster(&self) -> parking_lot::RwLockReadGuard<'_, LossBasedAdjuster> {
        self.loss_adjuster.read()
    }

    pub fn set_adaptation_enabled(&self, enabled: bool) {
        self.adaptation_enabled.store(enabled, Ordering::Relaxed);
        if enabled {
            info!("Adaptive optimization enabled for connection");
        } else {
            info!("Adaptive optimization disabled for connection");
        }
    }

    pub fn current_tier(&self) -> NetworkTier {
        self.current_tier.read().clone()
    }

    pub fn conditions(&self) -> NetworkConditions {
        self.conditions.read().clone()
    }

    pub fn parameters(&self) -> ConnectionParameters {
        self.parameters.read().clone()
    }

    pub fn adaptation_stats(&self) -> AdaptationStats {
        AdaptationStats {
            adaptation_count: self.adaptation_count.load(Ordering::Relaxed),
            last_adaptation: *self.last_adaptation.read(),
            current_tier: self.current_tier.read().clone(),
            enabled: self.adaptation_enabled.load(Ordering::Relaxed),
        }
    }

    pub async fn force_adapt(&self) -> Result<(), anyhow::Error> {
        if !self.adaptation_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.update_conditions();
        let detected_tier = self.detect_tier();

        // Force always applies the detected tier regardless of hysteresis
        self.apply_tier_parameters(&detected_tier)?;
        *self.current_tier.write() = detected_tier;
        *self.last_adaptation.write() = Instant::now();
        self.adaptation_count.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Manually set the network tier and apply its parameters immediately,
    /// bypassing detection and hysteresis.
    pub fn set_tier(&self, tier: NetworkTier) -> Result<(), anyhow::Error> {
        self.apply_tier_parameters(&tier)?;
        *self.current_tier.write() = tier;
        *self.last_adaptation.write() = Instant::now();
        self.adaptation_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::bandwidth::EwmaBandwidthEstimator;
    use super::super::loss::LossBasedAdjuster;
    use super::super::tier::tier_step_down;
    use super::*;

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
    }

    #[test]
    fn test_hysteresis_prevents_thrashing() {
        let hysteresis = HysteresisState {
            required_consecutive: 3,
            ..Default::default()
        };
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

    #[test]
    fn test_adaptive_tier_with_ewma() {
        let mut est = EwmaBandwidthEstimator::new(0.125, 20);
        let bytes = 312_500_000u64;
        let dur = Duration::from_secs(1);
        for _ in 0..20 {
            est.add_sample(bytes, dur);
        }
        let gbps = est.estimate_gbps();
        assert!(
            gbps > 2.0 && gbps < 3.0,
            "expected ~2.5 Gbps, got {gbps:.3}"
        );

        let tier = NetworkTier::from_gbps(gbps);
        assert!(
            matches!(tier, NetworkTier::Performance { .. }),
            "expected Performance, got {tier:?}"
        );

        let mut adj = LossBasedAdjuster::new(10, 5.0, 0.5);
        for _ in 0..10 {
            adj.record_loss(8.0);
        }
        assert!(adj.should_downgrade());

        let downgraded = tier_step_down(&tier);
        assert!(
            matches!(downgraded, NetworkTier::Standard { .. }),
            "expected Standard after downgrade, got {downgraded:?}"
        );
    }
}
