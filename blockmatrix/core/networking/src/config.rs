//! Network configuration

use crate::discovery::ServiceDiscoveryConfig as DiscoveryConfig;
use crate::health_check::HealthCheckConfig as HealthConfig;
use crate::circuit_breaker::CircuitBreakerConfig as CircuitConfig;
use crate::dht::DhtConfig;
use nexus_transport::TransportConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub service_discovery: DiscoveryConfig,
    pub load_balancing: LoadBalancingConfig,
    pub circuit_breaker: CircuitConfig,
    pub health_check: HealthConfig,
    pub dht: DhtConfig,
    pub metrics: MetricsConfig,
    pub transport: TransportConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            service_discovery: DiscoveryConfig::default(),
            load_balancing: LoadBalancingConfig::default(),
            circuit_breaker: CircuitConfig::default(),
            health_check: HealthConfig::default(),
            dht: DhtConfig::default(),
            metrics: MetricsConfig::default(),
            transport: TransportConfig::default(),
        }
    }
}


/// Load balancing configuration  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    pub strategy: crate::load_balancing::LoadBalancingStrategy,
}

impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            strategy: crate::load_balancing::LoadBalancingStrategy::RoundRobin,
        }
    }
}




/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub collection_interval: Duration,
    pub retention_period: Duration,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_secs(10),
            retention_period: Duration::from_secs(86400), // 24 hours
        }
    }
}

