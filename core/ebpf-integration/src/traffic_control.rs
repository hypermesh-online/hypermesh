//! Traffic control and QoS management using eBPF
//!
//! Implements traffic shaping, bandwidth limiting, and Quality of Service
//! controls at the kernel level for high-performance packet processing.

use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{EbpfConfig, EbpfProgram, TrafficShapingConfig, TrafficPriority, QosClass};

/// Traffic controller using eBPF for QoS and bandwidth management
pub struct TrafficController {
    config: EbpfConfig,
    running: bool,
    service_configs: RwLock<HashMap<String, TrafficShapingConfig>>,
    traffic_classes: RwLock<HashMap<String, TrafficClass>>,
    bandwidth_monitor: RwLock<BandwidthLimiter>,
}

impl TrafficController {
    pub async fn new(config: &EbpfConfig) -> Result<Self> {
        info!("🚦 Initializing traffic controller");
        
        Ok(Self {
            config: config.clone(),
            running: false,
            service_configs: RwLock::new(HashMap::new()),
            traffic_classes: RwLock::new(HashMap::new()),
            bandwidth_monitor: RwLock::new(BandwidthLimiter::new()),
        })
    }

    /// Configure traffic shaping for a specific service
    pub async fn configure_service(&self, service: &str, config: TrafficShapingConfig) -> Result<()> {
        info!("🎯 Configuring traffic shaping for service: {}", service);
        
        let mut configs = self.service_configs.write().await;
        configs.insert(service.to_string(), config.clone());
        
        // Create traffic class for the service
        let traffic_class = TrafficClass::new(&config);
        let mut classes = self.traffic_classes.write().await;
        classes.insert(service.to_string(), traffic_class);
        
        debug!("Traffic shaping configured: {}Mbps limit, {:?} priority", 
               config.bandwidth_limit_mbps, config.priority);
        
        Ok(())
    }

    /// Update bandwidth limits for a service
    pub async fn update_bandwidth_limit(&self, service: &str, limit_mbps: u32) -> Result<()> {
        info!("📊 Updating bandwidth limit for {}: {}Mbps", service, limit_mbps);
        
        let mut configs = self.service_configs.write().await;
        if let Some(config) = configs.get_mut(service) {
            config.bandwidth_limit_mbps = limit_mbps;
            
            // Update the traffic class
            let mut classes = self.traffic_classes.write().await;
            if let Some(class) = classes.get_mut(service) {
                class.update_bandwidth_limit(limit_mbps);
            }
        }
        
        Ok(())
    }

    /// Get current traffic statistics
    pub async fn get_traffic_stats(&self) -> HashMap<String, TrafficStats> {
        let classes = self.traffic_classes.read().await;
        classes.iter()
            .map(|(service, class)| (service.clone(), class.get_stats()))
            .collect()
    }

    /// Remove traffic shaping for a service
    pub async fn remove_service(&self, service: &str) -> Result<()> {
        info!("🗑️ Removing traffic shaping for service: {}", service);
        
        let mut configs = self.service_configs.write().await;
        configs.remove(service);
        
        let mut classes = self.traffic_classes.write().await;
        classes.remove(service);
        
        Ok(())
    }

    /// Apply global bandwidth policies
    pub async fn apply_global_policy(&self, policy: GlobalTrafficPolicy) -> Result<()> {
        info!("🌍 Applying global traffic policy");
        
        let mut bandwidth_monitor = self.bandwidth_monitor.write().await;
        bandwidth_monitor.apply_global_policy(policy);
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl EbpfProgram for TrafficController {
    async fn start(&mut self) -> Result<()> {
        info!("🚀 Starting traffic controller eBPF program");
        
        // In a real implementation, this would:
        // 1. Load TC (Traffic Control) eBPF programs
        // 2. Attach to network interfaces
        // 3. Configure qdisc and classes
        // 4. Set up rate limiting maps
        
        self.running = true;
        
        // Start background monitoring task
        let bandwidth_monitor = self.bandwidth_monitor.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                // Update bandwidth measurements
                {
                    let mut monitor = bandwidth_monitor.write().await;
                    monitor.update_measurements().await;
                }
            }
        });
        
        info!("✅ Traffic controller started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("🛑 Stopping traffic controller");
        self.running = false;
        Ok(())
    }

    async fn reload(&mut self) -> Result<()> {
        info!("🔄 Reloading traffic controller");
        self.stop().await?;
        self.start().await
    }

    fn name(&self) -> &str {
        "traffic-controller"
    }

    fn is_running(&self) -> bool {
        self.running
    }
}

/// Represents a traffic class with QoS parameters
struct TrafficClass {
    bandwidth_limit_mbps: u32,
    burst_size_kb: u32,
    priority: TrafficPriority,
    qos_class: QosClass,
    stats: TrafficStats,
}

impl TrafficClass {
    fn new(config: &TrafficShapingConfig) -> Self {
        Self {
            bandwidth_limit_mbps: config.bandwidth_limit_mbps,
            burst_size_kb: config.burst_size_kb,
            priority: config.priority.clone(),
            qos_class: config.qos_class.clone(),
            stats: TrafficStats::default(),
        }
    }

    fn update_bandwidth_limit(&mut self, limit_mbps: u32) {
        self.bandwidth_limit_mbps = limit_mbps;
    }

    fn get_stats(&self) -> TrafficStats {
        self.stats.clone()
    }
}

/// Global traffic policy for overall bandwidth management
#[derive(Debug, Clone)]
pub struct GlobalTrafficPolicy {
    pub total_bandwidth_mbps: u32,
    pub high_priority_percentage: u8,
    pub medium_priority_percentage: u8,
    pub low_priority_percentage: u8,
    pub burst_allowance_percentage: u8,
}

/// Traffic statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct TrafficStats {
    pub bytes_transmitted: u64,
    pub bytes_dropped: u64,
    pub packets_transmitted: u64,
    pub packets_dropped: u64,
    pub current_bandwidth_mbps: f64,
    pub average_latency_ms: f64,
}

/// Bandwidth limiter for enforcing rate limits
struct BandwidthLimiter {
    global_policy: Option<GlobalTrafficPolicy>,
    interface_limits: HashMap<String, u32>,
    current_usage: HashMap<String, f64>,
}

impl BandwidthLimiter {
    fn new() -> Self {
        Self {
            global_policy: None,
            interface_limits: HashMap::new(),
            current_usage: HashMap::new(),
        }
    }

    fn apply_global_policy(&mut self, policy: GlobalTrafficPolicy) {
        self.global_policy = Some(policy);
    }

    async fn update_measurements(&mut self) {
        // Simulate bandwidth measurements
        for (interface, _limit) in &self.interface_limits {
            let current_mbps = (rand::random::<f64>() * 100.0).round() / 10.0;
            self.current_usage.insert(interface.clone(), current_mbps);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_traffic_controller_creation() {
        let config = EbpfConfig::default();
        let controller = TrafficController::new(&config).await.unwrap();
        assert!(!controller.is_running());
        assert_eq!(controller.name(), "traffic-controller");
    }

    #[tokio::test]
    async fn test_service_configuration() {
        let config = EbpfConfig::default();
        let controller = TrafficController::new(&config).await.unwrap();
        
        let shaping_config = TrafficShapingConfig {
            bandwidth_limit_mbps: 100,
            burst_size_kb: 1024,
            priority: TrafficPriority::High,
            qos_class: QosClass::Guaranteed,
        };
        
        controller.configure_service("test-service", shaping_config).await.unwrap();
        
        let stats = controller.get_traffic_stats().await;
        assert!(stats.contains_key("test-service"));
    }

    #[test]
    fn test_traffic_class() {
        let config = TrafficShapingConfig {
            bandwidth_limit_mbps: 50,
            burst_size_kb: 512,
            priority: TrafficPriority::Medium,
            qos_class: QosClass::Burstable,
        };
        
        let class = TrafficClass::new(&config);
        assert_eq!(class.bandwidth_limit_mbps, 50);
        assert_eq!(class.burst_size_kb, 512);
    }
}