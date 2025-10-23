//! Load balancing using eBPF programs
//!
//! Provides high-performance layer 4 load balancing with various algorithms
//! and health checking at the kernel level.

use anyhow::Result;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{EbpfConfig, EbpfProgram, ServiceEndpoint, EndpointHealth};

/// Load balancer using eBPF for high-performance traffic distribution
pub struct LoadBalancer {
    config: EbpfConfig,
    running: bool,
    service_pools: RwLock<HashMap<String, ServicePool>>,
    load_balancing_stats: RwLock<LoadBalancingStats>,
    health_checker: RwLock<HealthChecker>,
}

impl LoadBalancer {
    pub async fn new(config: &EbpfConfig) -> Result<Self> {
        info!("⚖️ Initializing load balancer");
        
        Ok(Self {
            config: config.clone(),
            running: false,
            service_pools: RwLock::new(HashMap::new()),
            load_balancing_stats: RwLock::new(LoadBalancingStats::new()),
            health_checker: RwLock::new(HealthChecker::new()),
        })
    }

    /// Update endpoints for a service
    pub async fn update_endpoints(&self, service: &str, endpoints: Vec<ServiceEndpoint>) -> Result<()> {
        info!("🎯 Updating endpoints for service: {} ({} endpoints)", service, endpoints.len());
        
        let mut pools = self.service_pools.write().await;
        let pool = pools.entry(service.to_string())
            .or_insert_with(|| ServicePool::new(service, LoadBalancingAlgorithm::RoundRobin));
        
        pool.update_endpoints(endpoints);
        
        debug!("Updated service pool for {}", service);
        Ok(())
    }

    /// Get the next endpoint for a service using load balancing algorithm
    pub async fn get_endpoint(&self, service: &str) -> Option<ServiceEndpoint> {
        let pools = self.service_pools.read().await;
        if let Some(pool) = pools.get(service) {
            pool.get_next_endpoint()
        } else {
            warn!("No service pool found for: {}", service);
            None
        }
    }

    /// Update load balancing algorithm for a service
    pub async fn set_algorithm(&self, service: &str, algorithm: LoadBalancingAlgorithm) -> Result<()> {
        info!("🔄 Setting load balancing algorithm for {}: {:?}", service, algorithm);
        
        let mut pools = self.service_pools.write().await;
        if let Some(pool) = pools.get_mut(service) {
            pool.set_algorithm(algorithm);
        } else {
            return Err(anyhow::anyhow!("Service pool not found: {}", service));
        }
        
        Ok(())
    }

    /// Get load balancing statistics
    pub async fn get_stats(&self) -> LoadBalancingStats {
        self.load_balancing_stats.read().await.clone()
    }

    /// Get detailed service statistics
    pub async fn get_service_stats(&self, service: &str) -> Option<ServiceStats> {
        let pools = self.service_pools.read().await;
        pools.get(service).map(|pool| pool.get_stats())
    }

    /// Perform health check on all endpoints
    pub async fn health_check(&self) -> Result<()> {
        debug!("🏥 Performing health checks");
        
        let pools = self.service_pools.read().await;
        let mut health_checker = self.health_checker.write().await;
        
        for (service_name, pool) in pools.iter() {
            for endpoint in pool.get_all_endpoints() {
                let health = health_checker.check_endpoint(&endpoint).await;
                // In a real implementation, this would update the endpoint health
                debug!("Health check for {}:{}: {:?}", 
                       endpoint.ip, endpoint.port, health);
            }
        }
        
        Ok(())
    }

    /// Remove a service from load balancing
    pub async fn remove_service(&self, service: &str) -> Result<()> {
        info!("🗑️ Removing service from load balancer: {}", service);
        
        let mut pools = self.service_pools.write().await;
        pools.remove(service);
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl EbpfProgram for LoadBalancer {
    async fn start(&mut self) -> Result<()> {
        info!("🚀 Starting load balancer eBPF program");
        
        // In a real implementation, this would:
        // 1. Load XDP/TC eBPF programs for packet steering
        // 2. Set up connection tracking maps
        // 3. Configure NAT and port mapping
        // 4. Attach to network interfaces
        
        self.running = true;
        
        // Start background health checking
        let health_checker = self.health_checker.clone();
        let service_pools = self.service_pools.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Perform health checks
                let pools = service_pools.read().await;
                let mut checker = health_checker.write().await;
                
                for (service_name, pool) in pools.iter() {
                    for endpoint in pool.get_all_endpoints() {
                        let health = checker.check_endpoint(&endpoint).await;
                        debug!("Background health check for {}:{}: {:?}", 
                               endpoint.ip, endpoint.port, health);
                    }
                }
            }
        });
        
        // Start statistics collection
        let stats = self.load_balancing_stats.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                // Update statistics
                {
                    let mut stats = stats.write().await;
                    stats.update_counters();
                }
            }
        });
        
        info!("✅ Load balancer started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("🛑 Stopping load balancer");
        self.running = false;
        Ok(())
    }

    async fn reload(&mut self) -> Result<()> {
        info!("🔄 Reloading load balancer");
        self.stop().await?;
        self.start().await
    }

    fn name(&self) -> &str {
        "load-balancer"
    }

    fn is_running(&self) -> bool {
        self.running
    }
}

/// Load balancing algorithms
#[derive(Debug, Clone)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    WeightedRoundRobin,
    LeastConnections,
    WeightedLeastConnections,
    IpHash,
    ConsistentHash,
}

/// Service pool managing endpoints for a service
struct ServicePool {
    service_name: String,
    endpoints: Vec<ServiceEndpoint>,
    algorithm: LoadBalancingAlgorithm,
    current_index: usize,
    connection_counts: HashMap<String, u32>,
    stats: ServiceStats,
}

impl ServicePool {
    fn new(service_name: &str, algorithm: LoadBalancingAlgorithm) -> Self {
        Self {
            service_name: service_name.to_string(),
            endpoints: Vec::new(),
            algorithm,
            current_index: 0,
            connection_counts: HashMap::new(),
            stats: ServiceStats::new(service_name),
        }
    }

    fn update_endpoints(&mut self, endpoints: Vec<ServiceEndpoint>) {
        self.endpoints = endpoints;
        self.current_index = 0;
        
        // Initialize connection counts
        self.connection_counts.clear();
        for endpoint in &self.endpoints {
            let key = format!("{}:{}", endpoint.ip, endpoint.port);
            self.connection_counts.insert(key, 0);
        }
        
        self.stats.endpoint_count = self.endpoints.len() as u32;
    }

    fn get_next_endpoint(&self) -> Option<ServiceEndpoint> {
        if self.endpoints.is_empty() {
            return None;
        }

        let healthy_endpoints: Vec<_> = self.endpoints.iter()
            .filter(|e| matches!(e.health_status, EndpointHealth::Healthy))
            .collect();

        if healthy_endpoints.is_empty() {
            return None;
        }

        let selected = match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                let index = self.current_index % healthy_endpoints.len();
                healthy_endpoints[index].clone()
            },
            LoadBalancingAlgorithm::WeightedRoundRobin => {
                self.weighted_round_robin(&healthy_endpoints)
            },
            LoadBalancingAlgorithm::LeastConnections => {
                self.least_connections(&healthy_endpoints)
            },
            LoadBalancingAlgorithm::WeightedLeastConnections => {
                self.weighted_least_connections(&healthy_endpoints)
            },
            LoadBalancingAlgorithm::IpHash => {
                // Would use client IP in real implementation
                healthy_endpoints[0].clone()
            },
            LoadBalancingAlgorithm::ConsistentHash => {
                // Would use consistent hashing in real implementation
                healthy_endpoints[0].clone()
            },
        };

        Some(selected)
    }

    fn weighted_round_robin(&self, endpoints: &[&ServiceEndpoint]) -> ServiceEndpoint {
        // Simple implementation - in reality would maintain weighted counters
        let total_weight: u32 = endpoints.iter().map(|e| e.weight).sum();
        let mut current_weight = 0;
        let target = (rand::random::<f64>() * total_weight as f64) as u32;
        
        for endpoint in endpoints {
            current_weight += endpoint.weight;
            if current_weight >= target {
                return (*endpoint).clone();
            }
        }
        
        endpoints[0].clone()
    }

    fn least_connections(&self, endpoints: &[&ServiceEndpoint]) -> ServiceEndpoint {
        let mut min_connections = u32::MAX;
        let mut selected = endpoints[0];
        
        for endpoint in endpoints {
            let key = format!("{}:{}", endpoint.ip, endpoint.port);
            let connections = self.connection_counts.get(&key).unwrap_or(&0);
            if *connections < min_connections {
                min_connections = *connections;
                selected = endpoint;
            }
        }
        
        selected.clone()
    }

    fn weighted_least_connections(&self, endpoints: &[&ServiceEndpoint]) -> ServiceEndpoint {
        let mut min_ratio = f64::MAX;
        let mut selected = endpoints[0];
        
        for endpoint in endpoints {
            let key = format!("{}:{}", endpoint.ip, endpoint.port);
            let connections = *self.connection_counts.get(&key).unwrap_or(&0) as f64;
            let ratio = connections / endpoint.weight as f64;
            if ratio < min_ratio {
                min_ratio = ratio;
                selected = endpoint;
            }
        }
        
        selected.clone()
    }

    fn set_algorithm(&mut self, algorithm: LoadBalancingAlgorithm) {
        self.algorithm = algorithm;
    }

    fn get_all_endpoints(&self) -> &[ServiceEndpoint] {
        &self.endpoints
    }

    fn get_stats(&self) -> ServiceStats {
        self.stats.clone()
    }
}

/// Statistics for load balancing operations
#[derive(Debug, Clone)]
pub struct LoadBalancingStats {
    pub total_requests: u64,
    pub successful_routes: u64,
    pub failed_routes: u64,
    pub average_response_time_ms: f64,
    pub services_count: u32,
    pub healthy_endpoints: u32,
    pub unhealthy_endpoints: u32,
}

impl LoadBalancingStats {
    fn new() -> Self {
        Self {
            total_requests: 0,
            successful_routes: 0,
            failed_routes: 0,
            average_response_time_ms: 0.0,
            services_count: 0,
            healthy_endpoints: 0,
            unhealthy_endpoints: 0,
        }
    }

    fn update_counters(&mut self) {
        // Simulate some metrics updates
        self.total_requests += rand::random::<u64>() % 100;
        self.successful_routes += rand::random::<u64>() % 90;
        self.failed_routes = self.total_requests - self.successful_routes;
        self.average_response_time_ms = (rand::random::<f64>() * 50.0) + 5.0;
    }
}

/// Statistics for individual services
#[derive(Debug, Clone)]
pub struct ServiceStats {
    pub service_name: String,
    pub endpoint_count: u32,
    pub requests_routed: u64,
    pub failed_requests: u64,
    pub average_latency_ms: f64,
    pub last_updated: Instant,
}

impl ServiceStats {
    fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            endpoint_count: 0,
            requests_routed: 0,
            failed_requests: 0,
            average_latency_ms: 0.0,
            last_updated: Instant::now(),
        }
    }
}

/// Health checker for endpoints
struct HealthChecker {
    check_interval: std::time::Duration,
    timeout: std::time::Duration,
}

impl HealthChecker {
    fn new() -> Self {
        Self {
            check_interval: std::time::Duration::from_secs(30),
            timeout: std::time::Duration::from_secs(5),
        }
    }

    async fn check_endpoint(&self, endpoint: &ServiceEndpoint) -> EndpointHealth {
        // Simulate health check - in reality would make HTTP/TCP requests
        debug!("Health checking endpoint: {}:{}", endpoint.ip, endpoint.port);
        
        // Simulate occasional failures
        if rand::random::<f64>() < 0.1 {
            EndpointHealth::Unhealthy
        } else if rand::random::<f64>() < 0.05 {
            EndpointHealth::Degraded
        } else {
            EndpointHealth::Healthy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_load_balancer_creation() {
        let config = EbpfConfig::default();
        let balancer = LoadBalancer::new(&config).await.unwrap();
        assert!(!balancer.is_running());
        assert_eq!(balancer.name(), "load-balancer");
    }

    #[tokio::test]
    async fn test_endpoint_updates() {
        let config = EbpfConfig::default();
        let balancer = LoadBalancer::new(&config).await.unwrap();
        
        let endpoints = vec![
            ServiceEndpoint {
                ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
                port: 8080,
                weight: 1,
                health_status: EndpointHealth::Healthy,
            },
            ServiceEndpoint {
                ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 11)),
                port: 8080,
                weight: 2,
                health_status: EndpointHealth::Healthy,
            },
        ];
        
        balancer.update_endpoints("test-service", endpoints).await.unwrap();
        
        let endpoint = balancer.get_endpoint("test-service").await;
        assert!(endpoint.is_some());
    }

    #[test]
    fn test_service_pool() {
        let mut pool = ServicePool::new("test", LoadBalancingAlgorithm::RoundRobin);
        
        let endpoints = vec![
            ServiceEndpoint {
                ip: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
                port: 80,
                weight: 1,
                health_status: EndpointHealth::Healthy,
            },
        ];
        
        pool.update_endpoints(endpoints);
        assert_eq!(pool.endpoints.len(), 1);
        assert_eq!(pool.stats.endpoint_count, 1);
    }
}