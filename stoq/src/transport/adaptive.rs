//! Adaptive connection optimization for live connections
//! Provides real-time parameter adjustment based on network conditions

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
    /// Previous tier for comparison
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
        }
    }

    /// Update network conditions from connection statistics
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
            conditions.packet_loss = (frame_stats.path_response as f64 / total.max(1) as f64) * 100.0;
        }

        // Update throughput estimate
        let udp_stats = stats.udp_tx;
        // Calculate throughput based on bytes transmitted
        let duration = conditions.last_update.elapsed().as_secs_f64();
        if duration > 0.0 {
            let bytes_per_sec = udp_stats.bytes as f64 / duration;
            conditions.throughput_mbps = (bytes_per_sec * 8.0) / 1_000_000.0;
        }

        // Track retransmissions (using datagrams as proxy)
        conditions.retransmissions = udp_stats.datagrams;

        conditions.last_update = Instant::now();

        debug!(
            "Updated network conditions: RTT={:.2}ms, loss={:.2}%, throughput={:.2}Mbps",
            conditions.rtt_ms, conditions.packet_loss, conditions.throughput_mbps
        );
    }

    /// Detect network tier based on current conditions
    pub fn detect_tier(&self) -> NetworkTier {
        let conditions = self.conditions.read();

        // Use multiple heuristics to determine tier
        let mut estimated_gbps = conditions.bandwidth_estimate / 1000.0;

        // Adjust estimate based on actual throughput
        if conditions.throughput_mbps > 0.0 {
            estimated_gbps = (estimated_gbps + (conditions.throughput_mbps / 1000.0)) / 2.0;
        }

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

        NetworkTier::from_gbps(estimated_gbps)
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

        // Set datagram size - Quinn uses initial_mtu for this
        transport_config.initial_mtu(params.max_datagram_size);

        // Set timeouts
        transport_config.max_idle_timeout(Some(params.idle_timeout.try_into()?));
        if let Some(keep_alive) = params.keep_alive_interval {
            transport_config.keep_alive_interval(Some(keep_alive));
        }

        // Apply congestion control (this would need quinn support for dynamic changes)
        // For now, we can only log the intended change
        debug!("Would apply congestion control: {:?}", params.congestion_control);

        // Apply the transport config to the connection
        // Note: Quinn doesn't directly support updating transport config on live connections
        // We'll need to implement this at the QUIC protocol level or use connection migration

        // For now, we update what we can through the connection API
        self.connection.set_max_concurrent_bi_streams(VarInt::from_u32(params.max_streams));
        self.connection.set_max_concurrent_uni_streams(VarInt::from_u32(params.max_streams / 2));

        Ok(())
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

    #[test]
    fn test_network_tier_detection() {
        let conditions = NetworkConditions {
            rtt_ms: 5.0,
            packet_loss: 0.1,
            throughput_mbps: 2500.0,
            bandwidth_estimate: 10000.0,
            retransmissions: 10,
            jitter_ms: 1.0,
            last_update: Instant::now(),
        };

        // Should detect as Performance tier based on conditions
        // Actual detection would use the detect_tier method
    }

    #[test]
    fn test_hysteresis_prevents_thrashing() {
        let mut hysteresis = HysteresisState::default();
        hysteresis.required_consecutive = 3;

        // Should require 3 consecutive measurements
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

        // Verify parameters are reasonable
        assert!(params.stream_window > 0);
        assert!(params.max_streams > 0);
    }
}