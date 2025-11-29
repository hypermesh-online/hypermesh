//! Configuration management for HyperMesh platform integration

use serde::{Serialize, Deserialize};
use crate::transport::TransportConfig;
use crate::consensus::ConsensusConfig;
use crate::container::config::ContainerConfig as ContainerRuntimeConfig;
use crate::security::config::SecurityConfig;
use crate::orchestration::HyperMeshIntegrationConfig as OrchestrationConfig;
// Temporarily comment out STOQ
// use stoq::StoqConfig;

/// Temporary placeholder for StoqConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqConfig {
    pub enabled: bool,
}

impl Default for StoqConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

/// Main HyperMesh platform configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperMeshConfig {
    /// STOQ protocol configuration (placeholder)
    pub stoq: StoqConfig,
    /// Transport layer configuration
    pub transport: TransportConfig,
    /// Consensus configuration
    pub consensus: ConsensusConfig,
    /// Container runtime configuration
    pub container: ContainerRuntimeConfig,
    /// Security framework configuration
    pub security: SecurityConfig,
    /// Orchestration engine configuration
    pub orchestration: OrchestrationConfig,
    /// Integration-specific configuration
    pub integration: IntegrationConfig,
}

/// Integration layer specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// Platform initialization timeout
    pub initialization_timeout_secs: u64,
    /// Component health check interval
    pub health_check_interval_secs: u64,
    /// Cross-component communication timeout
    pub communication_timeout_secs: u64,
    /// Service registry configuration
    pub service_registry: ServiceRegistryConfig,
    /// Metrics collection configuration
    pub metrics: MetricsConfig,
}

/// Service registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistryConfig {
    /// Service registration TTL
    pub registration_ttl_secs: u64,
    /// Service discovery refresh interval
    pub discovery_refresh_interval_secs: u64,
    /// Health check timeout
    pub health_check_timeout_secs: u64,
    /// Maximum number of service retries
    pub max_retries: u32,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Metrics collection interval
    pub collection_interval_secs: u64,
    /// Metrics retention period
    pub retention_period_secs: u64,
    /// Export to Prometheus
    pub enable_prometheus: bool,
    /// Prometheus export port
    pub prometheus_port: u16,
}

/// Component-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    /// Component name
    pub name: String,
    /// Startup priority (lower numbers start first)
    pub priority: u32,
    /// Startup timeout
    pub startup_timeout_secs: u64,
    /// Health check endpoint
    pub health_endpoint: Option<String>,
    /// Dependencies on other components
    pub dependencies: Vec<String>,
}

impl Default for HyperMeshConfig {
    fn default() -> Self {
        Self {
            stoq: StoqConfig::default(),
            transport: TransportConfig::default(),
            consensus: ConsensusConfig::default(),
            container: ContainerRuntimeConfig::default(),
            security: SecurityConfig::default(),
            orchestration: OrchestrationConfig::default(),
            integration: IntegrationConfig::default(),
        }
    }
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            initialization_timeout_secs: 300, // 5 minutes
            health_check_interval_secs: 30,   // 30 seconds
            communication_timeout_secs: 10,   // 10 seconds
            service_registry: ServiceRegistryConfig::default(),
            metrics: MetricsConfig::default(),
        }
    }
}

impl Default for ServiceRegistryConfig {
    fn default() -> Self {
        Self {
            registration_ttl_secs: 300,           // 5 minutes
            discovery_refresh_interval_secs: 60,  // 1 minute
            health_check_timeout_secs: 5,         // 5 seconds
            max_retries: 3,
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            collection_interval_secs: 10,    // 10 seconds
            retention_period_secs: 86400,    // 24 hours
            enable_prometheus: true,
            prometheus_port: 9090,
        }
    }
}

impl HyperMeshConfig {
    /// Load configuration from YAML file
    pub fn from_yaml_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to YAML file
    pub fn to_yaml_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Validate configuration consistency
    pub fn validate(&self) -> Result<(), String> {
        // Validate port conflicts
        let mut ports = Vec::new();
        ports.push(self.transport.bind_port);
        ports.push(self.consensus.port);
        if let Some(port) = self.container.runtime_port {
            ports.push(port);
        }
        ports.push(self.integration.metrics.prometheus_port);
        
        // Check for port duplicates
        ports.sort_unstable();
        for window in ports.windows(2) {
            if window[0] == window[1] {
                return Err(format!("Port conflict detected: port {} is used multiple times", window[0]));
            }
        }
        
        // Validate timeout configurations
        if self.integration.initialization_timeout_secs < 60 {
            return Err("Initialization timeout must be at least 60 seconds".to_string());
        }
        
        if self.integration.health_check_interval_secs > self.integration.service_registry.registration_ttl_secs / 2 {
            return Err("Health check interval should be less than half of registration TTL".to_string());
        }
        
        Ok(())
    }
    
    /// Get configuration optimized for high performance
    pub fn high_performance_preset() -> Self {
        let mut config = Self::default();
        
        // Optimize integration settings for performance
        config.integration.health_check_interval_secs = 10; // Faster health checks
        config.integration.communication_timeout_secs = 5;  // Faster timeouts
        config.integration.metrics.collection_interval_secs = 5; // More frequent metrics
        
        // Optimize service registry for performance
        config.integration.service_registry.discovery_refresh_interval_secs = 30;
        config.integration.service_registry.health_check_timeout_secs = 2;
        
        config
    }
    
    /// Get configuration optimized for reliability
    pub fn high_reliability_preset() -> Self {
        let mut config = Self::default();
        
        // Optimize for reliability
        config.integration.initialization_timeout_secs = 600; // Longer initialization
        config.integration.health_check_interval_secs = 15;   // Conservative health checks
        config.integration.communication_timeout_secs = 30;   // Longer timeouts
        
        // Optimize service registry for reliability
        config.integration.service_registry.registration_ttl_secs = 600; // Longer TTL
        config.integration.service_registry.max_retries = 5; // More retries
        
        config
    }
}