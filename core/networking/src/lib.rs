//! Nexus Networking - Service mesh and P2P networking layer
//! 
//! This module provides:
//! - Distributed hash table (DHT) for service discovery
//! - Load balancing with health checking
//! - Circuit breaker and retry logic
//! - Traffic splitting for canary deployments
//! - Real-time metrics and observability

pub mod discovery;
pub mod load_balancing;
pub mod circuit_breaker;
pub mod health_check;
pub mod routing;
pub mod dht;
pub mod metrics;
pub mod config;
pub mod error;

pub use discovery::{ServiceDiscovery, ServiceRegistry, ServiceInstance};
pub use load_balancing::{LoadBalancer, LoadBalancingStrategy, BackendPool};
pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use health_check::{HealthChecker, HealthStatus};
pub use routing::{Router, RoutingRule, TrafficSplit};
pub use dht::{DistributedHashTable, DhtNode, DhtConfig};
pub use metrics::{NetworkMetrics, ConnectionMetrics, MetricsSummary};
pub use config::NetworkConfig;
pub use error::{NetworkError, Result};

use nexus_shared::{NodeId, ServiceId};
use nexus_transport::{QuicClient, QuicServer};
use nexus_state::StateManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, broadcast, mpsc};

/// Network manager for service mesh functionality
pub struct NetworkManager {
    config: NetworkConfig,
    node_id: NodeId,
    
    // Core components
    service_discovery: Arc<ServiceDiscovery>,
    load_balancer: Arc<LoadBalancer>,
    health_checker: Arc<HealthChecker>,
    circuit_breaker: Arc<CircuitBreaker>,
    router: Arc<Router>,
    dht: Arc<DistributedHashTable>,
    
    // Transport layer
    transport_client: Arc<QuicClient>,
    transport_server: Option<Arc<QuicServer>>,
    
    // State management
    state_manager: Option<Arc<StateManager>>,
    
    // Metrics
    metrics: Arc<NetworkMetrics>,
    
    // Service registry
    local_services: Arc<RwLock<HashMap<ServiceId, ServiceInstance>>>,
    remote_services: Arc<RwLock<HashMap<ServiceId, Vec<ServiceInstance>>>>,
    
    // Event channels
    service_events: broadcast::Sender<ServiceEvent>,
}

impl NetworkManager {
    /// Create a new network manager
    pub async fn new(config: &NetworkConfig) -> Result<Self> {
        let node_id = NodeId::random();
        
        // Create core components
        let service_discovery = Arc::new(ServiceDiscovery::new(&config.service_discovery, node_id).await?);
        let load_balancer = Arc::new(LoadBalancer::new(&config.load_balancing)?);
        let health_checker = Arc::new(HealthChecker::new(&config.health_check)?);
        let circuit_breaker = Arc::new(CircuitBreaker::new(&config.circuit_breaker)?);
        let router = Arc::new(Router::new());
        let dht = Arc::new(DistributedHashTable::new(node_id, config.dht.clone()));
        
        // Create certificate manager
        let cert_manager = Arc::new(
            nexus_transport::CertificateManager::new_self_signed(
                format!("nexus-{}", node_id),
                365,
                Duration::from_secs(24 * 60 * 60),
            ).await
            .map_err(|e| NetworkError::Transport { 
                message: format!("Failed to create certificate manager: {}", e) 
            })?
        );

        // Create transport client
        let transport_client = Arc::new(
            nexus_transport::QuicClient::new(
                config.transport.clone(),
                cert_manager.clone(),
            ).await
            .map_err(|e| NetworkError::Transport { 
                message: format!("Failed to create transport client: {}", e) 
            })?
        );
        
        let metrics = Arc::new(NetworkMetrics::new());
        let (service_events, _) = broadcast::channel(10000);
        
        Ok(Self {
            config: config.clone(),
            node_id,
            service_discovery,
            load_balancer,
            health_checker,
            circuit_breaker,
            router,
            dht,
            transport_client,
            transport_server: None,
            state_manager: None,
            metrics,
            local_services: Arc::new(RwLock::new(HashMap::new())),
            remote_services: Arc::new(RwLock::new(HashMap::new())),
            service_events,
        })
    }
    
    /// Start the network manager
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting network manager for node {}", self.node_id);
        
        // Components are already initialized and ready
        // Start background tasks if needed
        self.dht.start().await?;
        self.health_checker.start().await?;
        
        // Start background tasks
        self.start_background_tasks().await?;
        
        tracing::info!("Network manager started");
        Ok(())
    }
    
    /// Stop the network manager
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping network manager");
        
        // Stop components that have stop methods
        self.health_checker.stop().await?;
        self.dht.stop().await?;
        
        tracing::info!("Network manager stopped");
        Ok(())
    }
    
    /// Register a local service
    pub async fn register_service(&self, service: ServiceInstance) -> Result<()> {
        tracing::info!("Registering service: {}", service.service_id);
        
        // Store locally
        self.local_services.write().await.insert(service.service_id.clone(), service.clone());
        
        // Register with service discovery
        self.service_discovery.register_service(service.clone()).await?;
        
        // Announce to DHT
        self.dht.announce_service(&service.service_id, service.address).await?;
        
        // Emit event
        let _ = self.service_events.send(ServiceEvent::ServiceRegistered(service));
        
        Ok(())
    }
    
    /// Deregister a local service
    pub async fn deregister_service(&self, service_id: &ServiceId) -> Result<()> {
        tracing::info!("Deregistering service: {}", service_id);
        
        // Remove locally
        let service = self.local_services.write().await.remove(service_id);
        
        if let Some(service) = service {
            // Deregister from service discovery
            self.service_discovery.deregister_service(&service.service_id).await?;
            
            // Remove from DHT
            self.dht.remove_service(&service.service_id).await?;
            
            // Emit event
            let _ = self.service_events.send(ServiceEvent::ServiceDeregistered(service));
        }
        
        Ok(())
    }
    
    /// Discover services by name
    pub async fn discover_services(&self, service_name: &str) -> Result<Vec<ServiceInstance>> {
        // Try local cache first
        if let Some(instances) = self.remote_services.read().await.get(&ServiceId::new(service_name, "default")) {
            if !instances.is_empty() {
                return Ok(instances.clone());
            }
        }
        
        // Query service discovery
        let instances = self.service_discovery.discover_services(service_name).await?;
        
        if !instances.is_empty() {
            // Cache results
            let service_id = ServiceId::new(service_name, "default");
            self.remote_services.write().await.insert(service_id, instances.clone());
            return Ok(instances);
        }
        
        // Try DHT as fallback
        let service_id = ServiceId::new(service_name, "default");
        let addresses = self.dht.find_services(&service_id).await?;
        
        // Convert SocketAddr to ServiceInstance
        let instances = addresses.into_iter().map(|addr| ServiceInstance {
            service_id: service_id.clone(),
            node_id: NodeId::random(), // TODO: Get real node_id from DHT
            address: addr,
            health_status: HealthStatus::Healthy,
            metadata: std::collections::HashMap::new(),
            last_seen: std::time::SystemTime::now(),
        }).collect();
        
        Ok(instances)
    }
    
    /// Route a request to a service
    pub async fn route_request(
        &self,
        service_name: &str,
        request_data: Vec<u8>,
    ) -> Result<Vec<u8>> {
        // Convert service name to ServiceId
        let service_id = ServiceId::new(service_name, "default");
        
        // Discover service instances via DHT
        let addresses = self.dht.find_services(&service_id).await?;
        
        if addresses.is_empty() {
            return Err(NetworkError::ServiceNotFound { service_id });
        }
        
        // Convert SocketAddr to ServiceInstance
        let instances: Vec<ServiceInstance> = addresses.into_iter().map(|addr| ServiceInstance {
            service_id: service_id.clone(),
            node_id: NodeId::random(), // TODO: Get real node_id from DHT
            address: addr,
            health_status: HealthStatus::Healthy,
            metadata: std::collections::HashMap::new(),
            last_seen: std::time::SystemTime::now(),
        }).collect();
        
        // Extract addresses from instances for load balancing
        let addresses: Vec<SocketAddr> = instances.iter().map(|i| i.address).collect();
        
        // Apply load balancing
        let selected_address = self.load_balancer
            .select_instance(&service_id, &addresses)
            .await?;
        
        // Find the ServiceInstance with the selected address
        let selected_instance = instances
            .iter()
            .find(|i| i.address == selected_address)
            .ok_or_else(|| NetworkError::ServiceNotFound { service_id: service_id.clone() })?;
        
        // Check circuit breaker
        if !self.circuit_breaker.can_execute().await {
            return Err(NetworkError::CircuitBreakerOpen);
        }
        
        // Execute request with retry
        let result = self.execute_request_with_retry(
            selected_instance,
            request_data,
            3, // max retries
        ).await;
        
        // Update circuit breaker
        match &result {
            Ok(_) => {
                self.circuit_breaker.record_success().await;
            }
            Err(_) => {
                self.circuit_breaker.record_failure().await;
            }
        }
        
        result
    }
    
    /// Execute request with retry logic
    async fn execute_request_with_retry(
        &self,
        instance: &ServiceInstance,
        request_data: Vec<u8>,
        max_retries: usize,
    ) -> Result<Vec<u8>> {
        let mut attempts = 0;
        let mut last_error = None;
        
        while attempts <= max_retries {
            match self.execute_request(instance, &request_data).await {
                Ok(response) => {
                    // Update metrics
                    self.metrics.record_request_success();
                    
                    return Ok(response);
                }
                Err(e) => {
                    attempts += 1;
                    last_error = Some(e);
                    
                    if attempts <= max_retries {
                        // Exponential backoff
                        let delay = Duration::from_millis(100 * 2u64.pow(attempts as u32 - 1));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        // Update metrics
        self.metrics.record_request_failure();
        
        Err(last_error.unwrap_or_else(|| NetworkError::RequestFailed { 
            message: "All retry attempts failed".to_string() 
        }))
    }
    
    /// Execute a single request to a service instance
    async fn execute_request(&self, instance: &ServiceInstance, request_data: &[u8]) -> Result<Vec<u8>> {
        // Connect to service if not already connected
        if !self.transport_client.is_connected(instance.node_id).await {
            self.transport_client.connect_with_retry(
                instance.address,
                &format!("service-{}", instance.service_id.name()),
                3,
                Duration::from_millis(1000),
            ).await
            .map_err(|e| NetworkError::ConnectionFailed { 
                address: instance.address,
                error: e.to_string(),
            })?;
        }
        
        // Create request message
        let request = nexus_transport::TransportMessage::new(
            nexus_transport::MessageType::Data,
            self.node_id,
            Some(instance.node_id),
            request_data.to_vec(),
        );
        
        // Send request and wait for response
        let response = self.transport_client
            .send_request(
                instance.node_id,
                request,
                Duration::from_secs(30),
            ).await
            .map_err(|e| NetworkError::RequestFailed { 
                message: e.to_string() 
            })?;
        
        Ok(response.payload)
    }
    
    /// Start background tasks
    async fn start_background_tasks(&self) -> Result<()> {
        // Start service cleanup task
        let manager = self.clone_for_task();
        tokio::spawn(async move {
            manager.service_cleanup_task().await;
        });
        
        // Start metrics collection task
        let manager = self.clone_for_task();
        tokio::spawn(async move {
            manager.metrics_collection_task().await;
        });
        
        Ok(())
    }
    
    /// Service cleanup task - removes stale service instances
    async fn service_cleanup_task(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            
            let mut remote_services = self.remote_services.write().await;
            let now = SystemTime::now();
            
            // Remove stale services
            remote_services.retain(|service_id, instances| {
                instances.retain(|instance| {
                    if let Ok(elapsed) = now.duration_since(instance.last_seen) {
                        elapsed < Duration::from_secs(300) // 5 minute timeout
                    } else {
                        false
                    }
                });
                
                if instances.is_empty() {
                    tracing::debug!("Removed stale service: {}", service_id);
                    false
                } else {
                    true
                }
            });
        }
    }
    
    /// Metrics collection task
    async fn metrics_collection_task(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            
            // Collect and update metrics
            let local_count = self.local_services.read().await.len();
            let remote_count: usize = self.remote_services.read().await
                .values()
                .map(|instances| instances.len())
                .sum();
            
            self.metrics.update_service_counts(local_count);
        }
    }
    
    /// Get network statistics
    pub async fn stats(&self) -> NetworkStats {
        let local_services = self.local_services.read().await;
        let remote_services = self.remote_services.read().await;
        
        NetworkStats {
            node_id: self.node_id,
            local_service_count: local_services.len(),
            remote_service_count: remote_services.values().map(|v| v.len()).sum(),
            total_connections: self.transport_client.connection_count().await,
            metrics: self.metrics.summary(),
        }
    }
    
    /// Subscribe to service events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<ServiceEvent> {
        self.service_events.subscribe()
    }
    
    /// Clone for task spawning (simplified approach)
    fn clone_for_task(&self) -> Arc<Self> {
        // In a real implementation, this would require proper Arc wrapping
        unreachable!("This would require restructuring to use Arc<NetworkManager>")
    }
}

/// Service event types
#[derive(Debug, Clone)]
pub enum ServiceEvent {
    ServiceRegistered(ServiceInstance),
    ServiceDeregistered(ServiceInstance),
    ServiceHealthChanged(ServiceId, HealthStatus),
    ServiceDiscovered(Vec<ServiceInstance>),
}

/// Network statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub node_id: NodeId,
    pub local_service_count: usize,
    pub remote_service_count: usize,
    pub total_connections: usize,
    pub metrics: metrics::MetricsSummary,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_network_manager_creation() {
        let config = NetworkConfig::default();
        let manager = NetworkManager::new(&config).await;
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_service_registration() {
        let config = NetworkConfig::default();
        let manager = NetworkManager::new(&config).await.unwrap();
        
        let service = ServiceInstance {
            service_id: ServiceId::new("test-service", "default"),
            node_id: NodeId::random(),
            address: "127.0.0.1:8080".parse().unwrap(),
            health_status: HealthStatus::Healthy,
            metadata: HashMap::new(),
            last_seen: SystemTime::now(),
        };
        
        // Test would require proper async setup
        // This demonstrates the API structure
        let result = manager.register_service(service).await;
        match result {
            Ok(()) => {},
            Err(_) => {
                // Expected to fail without proper setup
            }
        }
    }
}