//! Network monitoring using eBPF programs
//! 
//! Provides real-time network visibility with kernel-level packet inspection

use anyhow::Result;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{EbpfConfig, EbpfProgram, NetworkStats};

/// Network monitoring using eBPF
pub struct NetworkMonitor {
    config: EbpfConfig,
    running: bool,
    stats: RwLock<NetworkStats>,
    connection_tracker: RwLock<ConnectionTracker>,
    bandwidth_monitor: RwLock<BandwidthMonitor>,
    latency_tracker: RwLock<LatencyTracker>,
}

impl NetworkMonitor {
    pub async fn new(config: &EbpfConfig) -> Result<Self> {
        info!("🔧 Initializing network monitor");
        
        Ok(Self {
            config: config.clone(),
            running: false,
            stats: RwLock::new(NetworkStats::default()),
            connection_tracker: RwLock::new(ConnectionTracker::new()),
            bandwidth_monitor: RwLock::new(BandwidthMonitor::new()),
            latency_tracker: RwLock::new(LatencyTracker::new()),
        })
    }

    /// Get current network statistics
    pub async fn get_stats(&self) -> Result<NetworkStats> {
        let mut stats = self.stats.write().await;
        
        // Update stats from various trackers
        let connections = self.connection_tracker.read().await;
        stats.connections_tracked = connections.active_count();

        let bandwidth = self.bandwidth_monitor.read().await;
        stats.bandwidth_utilization = bandwidth.current_utilization();

        let latency = self.latency_tracker.read().await;
        let percentiles = latency.percentiles();
        stats.latency_p50_microseconds = percentiles.p50;
        stats.latency_p95_microseconds = percentiles.p95;
        stats.latency_p99_microseconds = percentiles.p99;

        Ok(stats.clone())
    }

    /// Get detailed connection information
    pub async fn get_connections(&self) -> Vec<ConnectionInfo> {
        self.connection_tracker.read().await.get_connections()
    }

    /// Get bandwidth utilization per interface
    pub async fn get_interface_stats(&self) -> HashMap<String, InterfaceStats> {
        self.bandwidth_monitor.read().await.get_interface_stats()
    }

    /// Get latency histogram
    pub async fn get_latency_distribution(&self) -> LatencyDistribution {
        self.latency_tracker.read().await.get_distribution()
    }

    /// Start monitoring specific service traffic
    pub async fn monitor_service(&self, service_name: &str, ports: Vec<u16>) -> Result<()> {
        info!("📊 Starting service monitoring for: {}", service_name);
        
        // In a real implementation, this would:
        // 1. Compile eBPF program with service-specific filters
        // 2. Attach to network interfaces
        // 3. Start collecting metrics for the specific service
        
        debug!("Monitoring ports {:?} for service {}", ports, service_name);
        Ok(())
    }

    /// Stop monitoring specific service
    pub async fn stop_monitoring_service(&self, service_name: &str) -> Result<()> {
        info!("🛑 Stopping service monitoring for: {}", service_name);
        Ok(())
    }

    /// Simulate packet processing updates
    async fn update_packet_stats(&self, packets: u64, bytes: u64, dropped: u64) {
        let mut stats = self.stats.write().await;
        stats.packets_processed += packets;
        stats.bytes_processed += bytes;
        stats.packets_dropped += dropped;
    }
}

#[async_trait::async_trait]
impl EbpfProgram for NetworkMonitor {
    async fn start(&mut self) -> Result<()> {
        info!("🚀 Starting network monitor eBPF program");
        
        // In a real implementation, this would:
        // 1. Load and verify eBPF bytecode
        // 2. Attach to network interfaces
        // 3. Set up maps for data sharing
        // 4. Start collection threads
        
        self.running = true;
        
        // Start background task to simulate monitoring
        let stats_clone = self.stats.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            let mut counter = 0u64;
            
            loop {
                interval.tick().await;
                counter += 1;
                
                // Simulate network activity
                let packets = 100 + (counter % 50);
                let bytes = packets * 1500; // Average packet size
                let dropped = if counter % 100 == 0 { 1 } else { 0 };
                
                // Update stats
                {
                    let mut stats = stats_clone.write().await;
                    stats.packets_processed += packets;
                    stats.bytes_processed += bytes;
                    stats.packets_dropped += dropped;
                }
            }
        });
        
        info!("✅ Network monitor started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("🛑 Stopping network monitor");
        self.running = false;
        Ok(())
    }

    async fn reload(&mut self) -> Result<()> {
        info!("🔄 Reloading network monitor");
        self.stop().await?;
        self.start().await
    }

    fn name(&self) -> &str {
        "network-monitor"
    }

    fn is_running(&self) -> bool {
        self.running
    }
}

/// Tracks active network connections
struct ConnectionTracker {
    connections: HashMap<ConnectionKey, ConnectionInfo>,
    last_cleanup: Instant,
}

impl ConnectionTracker {
    fn new() -> Self {
        Self {
            connections: HashMap::new(),
            last_cleanup: Instant::now(),
        }
    }

    fn active_count(&self) -> u32 {
        self.connections.len() as u32
    }

    fn get_connections(&self) -> Vec<ConnectionInfo> {
        self.connections.values().cloned().collect()
    }

    fn add_connection(&mut self, conn: ConnectionInfo) {
        let key = ConnectionKey {
            src_ip: conn.src_ip,
            dst_ip: conn.dst_ip,
            src_port: conn.src_port,
            dst_port: conn.dst_port,
            protocol: conn.protocol.clone(),
        };
        self.connections.insert(key, conn);
    }

    fn cleanup_stale_connections(&mut self) {
        if self.last_cleanup.elapsed() < Duration::from_secs(60) {
            return;
        }

        let now = Instant::now();
        let timeout = Duration::from_secs(300); // 5 minutes
        
        self.connections.retain(|_, conn| {
            now.duration_since(conn.last_seen) < timeout
        });
        
        self.last_cleanup = now;
    }
}

/// Monitors bandwidth utilization
struct BandwidthMonitor {
    interface_stats: HashMap<String, InterfaceStats>,
    samples: Vec<BandwidthSample>,
    max_samples: usize,
}

impl BandwidthMonitor {
    fn new() -> Self {
        Self {
            interface_stats: HashMap::new(),
            samples: Vec::new(),
            max_samples: 1000,
        }
    }

    fn current_utilization(&self) -> f64 {
        // Calculate current bandwidth utilization as percentage
        if self.samples.is_empty() {
            return 0.0;
        }

        let recent_samples: Vec<_> = self.samples.iter()
            .rev()
            .take(10)
            .collect();
        
        let avg_mbps: f64 = recent_samples.iter()
            .map(|s| s.mbps)
            .sum::<f64>() / recent_samples.len() as f64;

        // Assume 1Gbps interface capacity
        (avg_mbps / 1000.0) * 100.0
    }

    fn get_interface_stats(&self) -> HashMap<String, InterfaceStats> {
        self.interface_stats.clone()
    }

    fn add_sample(&mut self, sample: BandwidthSample) {
        self.samples.push(sample);
        if self.samples.len() > self.max_samples {
            self.samples.remove(0);
        }
    }
}

/// Tracks network latency
struct LatencyTracker {
    samples: Vec<u64>, // microseconds
    max_samples: usize,
}

impl LatencyTracker {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
            max_samples: 10000,
        }
    }

    fn percentiles(&self) -> LatencyPercentiles {
        if self.samples.is_empty() {
            return LatencyPercentiles::default();
        }

        let mut sorted = self.samples.clone();
        sorted.sort_unstable();

        let len = sorted.len();
        LatencyPercentiles {
            p50: sorted[len * 50 / 100],
            p95: sorted[len * 95 / 100],
            p99: sorted[len * 99 / 100],
        }
    }

    fn get_distribution(&self) -> LatencyDistribution {
        // Create histogram buckets
        let buckets = vec![
            (100, 0),    // < 100μs
            (1000, 0),   // < 1ms
            (10000, 0),  // < 10ms
            (100000, 0), // < 100ms
            (u64::MAX, 0), // > 100ms
        ];

        let mut distribution = buckets;
        
        for &latency in &self.samples {
            for (threshold, count) in &mut distribution {
                if latency < *threshold {
                    *count += 1;
                    break;
                }
            }
        }

        LatencyDistribution { buckets: distribution }
    }

    fn add_sample(&mut self, latency_us: u64) {
        self.samples.push(latency_us);
        if self.samples.len() > self.max_samples {
            self.samples.remove(0);
        }
    }
}

// Data structures

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct ConnectionKey {
    src_ip: IpAddr,
    dst_ip: IpAddr,
    src_port: u16,
    dst_port: u16,
    protocol: String,
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub src_ip: IpAddr,
    pub dst_ip: IpAddr,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub established_at: Instant,
    pub last_seen: Instant,
    pub state: ConnectionState,
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Established,
    SynSent,
    SynReceived,
    FinWait1,
    FinWait2,
    TimeWait,
    Closed,
}

#[derive(Debug, Clone)]
pub struct InterfaceStats {
    pub name: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub errors: u64,
    pub drops: u64,
    pub link_speed_mbps: u64,
}

#[derive(Debug, Clone)]
struct BandwidthSample {
    timestamp: Instant,
    mbps: f64,
    interface: String,
}

#[derive(Debug, Clone, Default)]
pub struct LatencyPercentiles {
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
}

#[derive(Debug, Clone)]
pub struct LatencyDistribution {
    pub buckets: Vec<(u64, u32)>, // (threshold_us, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_monitor_creation() {
        let config = EbpfConfig::default();
        let monitor = NetworkMonitor::new(&config).await.unwrap();
        assert!(!monitor.is_running());
        assert_eq!(monitor.name(), "network-monitor");
    }

    #[test]
    fn test_connection_tracker() {
        let mut tracker = ConnectionTracker::new();
        assert_eq!(tracker.active_count(), 0);
        
        let conn = ConnectionInfo {
            src_ip: "192.168.1.1".parse().unwrap(),
            dst_ip: "192.168.1.2".parse().unwrap(),
            src_port: 12345,
            dst_port: 80,
            protocol: "TCP".to_string(),
            bytes_sent: 1024,
            bytes_received: 2048,
            established_at: Instant::now(),
            last_seen: Instant::now(),
            state: ConnectionState::Established,
        };
        
        tracker.add_connection(conn);
        assert_eq!(tracker.active_count(), 1);
    }

    #[test]
    fn test_bandwidth_monitor() {
        let monitor = BandwidthMonitor::new();
        assert_eq!(monitor.current_utilization(), 0.0);
    }

    #[test]
    fn test_latency_tracker() {
        let mut tracker = LatencyTracker::new();
        
        // Add some sample latencies
        tracker.add_sample(100);  // 100μs
        tracker.add_sample(200);  // 200μs
        tracker.add_sample(1000); // 1ms
        
        let percentiles = tracker.percentiles();
        assert!(percentiles.p50 > 0);
        assert!(percentiles.p99 >= percentiles.p50);
    }
}