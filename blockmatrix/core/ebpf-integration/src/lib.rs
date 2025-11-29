//! eBPF Integration for Nexus
//! 
//! Provides kernel-level networking, monitoring, and policy enforcement
//! using eBPF programs for high-performance packet processing.

use anyhow::Result;
use nexus_shared::*;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

pub mod network_monitor;
pub mod traffic_control;
pub mod load_balancer;
pub mod security_policy;
pub mod metrics;
pub mod programs;
pub mod dns_ct;

/// Main eBPF manager that coordinates all eBPF programs
pub struct EbpfManager {
    programs: RwLock<HashMap<String, Box<dyn EbpfProgram>>>,
    network_monitor: Option<network_monitor::NetworkMonitor>,
    traffic_control: Option<traffic_control::TrafficController>,
    load_balancer: Option<load_balancer::LoadBalancer>,
    security_policy: Option<security_policy::SecurityPolicyEngine>,
    dns_ct: Option<dns_ct::DnsCtManager>,
    metrics: metrics::EbpfMetrics,
}

impl EbpfManager {
    /// Create a new eBPF manager
    pub async fn new(config: &EbpfConfig) -> Result<Self> {
        info!("🔧 Initializing eBPF manager");

        // Check for required capabilities
        Self::check_capabilities()?;

        let mut manager = Self {
            programs: RwLock::new(HashMap::new()),
            network_monitor: None,
            traffic_control: None,
            load_balancer: None,
            security_policy: None,
            dns_ct: None,
            metrics: metrics::EbpfMetrics::new(),
        };

        // Initialize components based on configuration
        if config.network_monitoring {
            manager.network_monitor = Some(network_monitor::NetworkMonitor::new(config).await?);
            info!("📊 Network monitoring enabled");
        }

        if config.traffic_control {
            manager.traffic_control = Some(traffic_control::TrafficController::new(config).await?);
            info!("🚦 Traffic control enabled");
        }

        if config.load_balancing {
            manager.load_balancer = Some(load_balancer::LoadBalancer::new(config).await?);
            info!("⚖️  Load balancing enabled");
        }

        if config.security_policies {
            manager.security_policy = Some(security_policy::SecurityPolicyEngine::new(config).await?);
            info!("🔒 Security policies enabled");
        }

        // Initialize DNS/CT eBPF if configured
        if config.dns_ct_enabled {
            let dns_ct_config = dns_ct::DnsCtConfig::default();
            manager.dns_ct = Some(dns_ct::DnsCtManager::new(dns_ct_config).await?);
            info!("🌐 DNS/CT eBPF enabled");
        }

        Ok(manager)
    }

    /// Start all eBPF programs
    pub async fn start(&mut self) -> Result<()> {
        info!("🚀 Starting eBPF programs...");

        if let Some(ref mut monitor) = self.network_monitor {
            monitor.start().await?;
            debug!("Network monitor started");
        }

        if let Some(ref mut controller) = self.traffic_control {
            controller.start().await?;
            debug!("Traffic controller started");
        }

        if let Some(ref mut balancer) = self.load_balancer {
            balancer.start().await?;
            debug!("Load balancer started");
        }

        if let Some(ref mut policy) = self.security_policy {
            policy.start().await?;
            debug!("Security policy engine started");
        }

        if let Some(ref mut dns_ct) = self.dns_ct {
            dns_ct.start().await?;
            debug!("DNS/CT eBPF started");
        }

        info!("✅ eBPF programs started successfully");
        Ok(())
    }

    /// Stop all eBPF programs
    pub async fn stop(&mut self) -> Result<()> {
        info!("🛑 Stopping eBPF programs...");

        if let Some(ref mut policy) = self.security_policy {
            policy.stop().await?;
        }

        if let Some(ref mut balancer) = self.load_balancer {
            balancer.stop().await?;
        }

        if let Some(ref mut controller) = self.traffic_control {
            controller.stop().await?;
        }

        if let Some(ref mut monitor) = self.network_monitor {
            monitor.stop().await?;
        }

        info!("👋 eBPF programs stopped");
        Ok(())
    }

    /// Get network statistics from eBPF programs
    pub async fn network_stats(&self) -> Result<NetworkStats> {
        if let Some(ref monitor) = self.network_monitor {
            monitor.get_stats().await
        } else {
            Ok(NetworkStats::default())
        }
    }

    /// Configure traffic shaping for a service
    pub async fn configure_traffic_shaping(&self, service: &str, config: TrafficShapingConfig) -> Result<()> {
        info!("🚦 Configuring traffic shaping for service: {}", service);
        
        if let Some(ref controller) = self.traffic_control {
            controller.configure_service(service, config).await
        } else {
            Err(anyhow::anyhow!("Traffic control not enabled"))
        }
    }

    /// Update load balancing rules
    pub async fn update_load_balancing(&self, service: &str, endpoints: Vec<ServiceEndpoint>) -> Result<()> {
        info!("⚖️  Updating load balancing for service: {}", service);
        
        if let Some(ref balancer) = self.load_balancer {
            balancer.update_endpoints(service, endpoints).await
        } else {
            Err(anyhow::anyhow!("Load balancing not enabled"))
        }
    }

    /// Apply security policy
    pub async fn apply_security_policy(&self, policy: SecurityPolicy) -> Result<()> {
        info!("🔒 Applying security policy: {}", policy.name);
        
        if let Some(ref engine) = self.security_policy {
            engine.apply_policy(policy).await
        } else {
            Err(anyhow::anyhow!("Security policies not enabled"))
        }
    }

    /// Get comprehensive eBPF metrics
    pub async fn metrics(&self) -> Result<metrics::EbpfMetricsSnapshot> {
        self.metrics.snapshot().await
    }

    /// Check if the system has required capabilities for eBPF
    fn check_capabilities() -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use caps::{Capability, CapSet};
            
            let caps = caps::read(None, CapSet::Effective)?;
            
            if !caps.contains(&Capability::CAP_SYS_ADMIN) {
                return Err(anyhow::anyhow!(
                    "CAP_SYS_ADMIN capability required for eBPF programs"
                ));
            }

            if !caps.contains(&Capability::CAP_NET_ADMIN) {
                return Err(anyhow::anyhow!(
                    "CAP_NET_ADMIN capability required for network eBPF programs"
                ));
            }
        }

        // Check for eBPF support in kernel
        if !Self::kernel_supports_ebpf()? {
            return Err(anyhow::anyhow!(
                "Kernel does not support required eBPF features"
            ));
        }

        Ok(())
    }

    /// Check if kernel supports eBPF features
    fn kernel_supports_ebpf() -> Result<bool> {
        // Check /proc/version for minimum kernel version
        let version_info = std::fs::read_to_string("/proc/version")
            .unwrap_or_else(|_| "Unknown".to_string());
        
        debug!("Kernel version: {}", version_info);
        
        // For this demo, assume eBPF is supported
        // In production, this would parse kernel version and check specific features
        Ok(true)
    }
}

/// Configuration for eBPF programs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EbpfConfig {
    pub network_monitoring: bool,
    pub traffic_control: bool,
    pub load_balancing: bool,
    pub security_policies: bool,
    pub dns_ct_enabled: bool,
    pub interfaces: Vec<String>,
    pub log_level: String,
    pub metrics_interval_ms: u64,
}

impl Default for EbpfConfig {
    fn default() -> Self {
        Self {
            network_monitoring: true,
            traffic_control: true,
            load_balancing: true,
            security_policies: true,
            dns_ct_enabled: true,
            interfaces: vec!["eth0".to_string(), "lo".to_string()],
            log_level: "info".to_string(),
            metrics_interval_ms: 1000,
        }
    }
}

/// Network statistics collected by eBPF programs
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NetworkStats {
    pub packets_processed: u64,
    pub bytes_processed: u64,
    pub packets_dropped: u64,
    pub connections_tracked: u32,
    pub bandwidth_utilization: f64,
    pub latency_p50_microseconds: u64,
    pub latency_p95_microseconds: u64,
    pub latency_p99_microseconds: u64,
}

/// Traffic shaping configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrafficShapingConfig {
    pub bandwidth_limit_mbps: u32,
    pub burst_size_kb: u32,
    pub priority: TrafficPriority,
    pub qos_class: QosClass,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TrafficPriority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum QosClass {
    Guaranteed,
    Burstable,
    BestEffort,
}

/// Service endpoint for load balancing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceEndpoint {
    pub ip: std::net::IpAddr,
    pub port: u16,
    pub weight: u32,
    pub health_status: EndpointHealth,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EndpointHealth {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Security policy definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityPolicy {
    pub name: String,
    pub rules: Vec<SecurityRule>,
    pub action: PolicyAction,
    pub priority: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityRule {
    pub source_cidr: String,
    pub destination_port: Option<u16>,
    pub protocol: Option<String>,
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PolicyAction {
    Allow,
    Deny,
    RateLimit(u32),
    Log,
}

/// Base trait for all eBPF programs
#[async_trait::async_trait]
pub trait EbpfProgram: Send + Sync {
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn reload(&mut self) -> Result<()>;
    fn name(&self) -> &str;
    fn is_running(&self) -> bool;
}

/// Initialize eBPF integration
pub async fn init(config: EbpfConfig) -> Result<EbpfManager> {
    info!("🚀 Initializing Nexus eBPF integration");
    
    // Check if running on Linux
    #[cfg(not(target_os = "linux"))]
    {
        warn!("eBPF integration is only supported on Linux");
        return Err(anyhow::anyhow!("eBPF requires Linux kernel"));
    }

    EbpfManager::new(&config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ebpf_config_default() {
        let config = EbpfConfig::default();
        assert!(config.network_monitoring);
        assert!(config.traffic_control);
        assert!(config.load_balancing);
        assert!(config.security_policies);
        assert!(!config.interfaces.is_empty());
    }

    #[test]
    fn test_network_stats_default() {
        let stats = NetworkStats::default();
        assert_eq!(stats.packets_processed, 0);
        assert_eq!(stats.bytes_processed, 0);
        assert_eq!(stats.connections_tracked, 0);
    }

    #[test]
    fn test_security_policy_serialization() {
        let policy = SecurityPolicy {
            name: "test-policy".to_string(),
            rules: vec![SecurityRule {
                source_cidr: "192.168.1.0/24".to_string(),
                destination_port: Some(80),
                protocol: Some("TCP".to_string()),
                rate_limit: Some(100),
            }],
            action: PolicyAction::Allow,
            priority: 1,
        };

        let json = serde_json::to_string(&policy).unwrap();
        let parsed: SecurityPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy.name, parsed.name);
    }
}