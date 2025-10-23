//! Service discovery implementation

use crate::{Result, NetworkError, HealthStatus};
use nexus_shared::{NodeId, ServiceId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, broadcast};
use tracing::{info, warn, debug, error};

/// Service discovery engine
pub struct ServiceDiscovery {
    config: ServiceDiscoveryConfig,
    node_id: NodeId,
    
    /// Local service registry
    registry: Arc<ServiceRegistry>,
    
    /// Event notifications
    event_sender: broadcast::Sender<ServiceDiscoveryEvent>,
    
    /// Background task handles
    cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

/// Service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryConfig {
    /// Service TTL in seconds
    pub service_ttl: u64,
    
    /// Cleanup interval in seconds
    pub cleanup_interval: u64,
    
    /// Maximum services per node
    pub max_services_per_node: usize,
    
    /// Enable service announcements
    pub enable_announcements: bool,
    
    /// Announcement interval in seconds
    pub announcement_interval: u64,
}

impl Default for ServiceDiscoveryConfig {
    fn default() -> Self {
        Self {
            service_ttl: 300,        // 5 minutes
            cleanup_interval: 60,    // 1 minute
            max_services_per_node: 100,
            enable_announcements: true,
            announcement_interval: 30, // 30 seconds
        }
    }
}

/// Service registry for managing service instances
pub struct ServiceRegistry {
    /// Registered services by service ID
    services: RwLock<HashMap<ServiceId, Vec<ServiceInstance>>>,
    
    /// Services by node ID for cleanup
    services_by_node: RwLock<HashMap<NodeId, Vec<ServiceId>>>,
    
    /// Service metadata
    service_metadata: RwLock<HashMap<ServiceId, ServiceMetadata>>,
}

/// Service instance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    /// Unique service identifier
    pub service_id: ServiceId,
    
    /// Node hosting this service
    pub node_id: NodeId,
    
    /// Network address for this service
    pub address: SocketAddr,
    
    /// Current health status
    pub health_status: HealthStatus,
    
    /// Service metadata
    pub metadata: HashMap<String, String>,
    
    /// Last seen timestamp
    pub last_seen: SystemTime,
}

/// Service metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetadata {
    /// Service version
    pub version: String,
    
    /// Service tags
    pub tags: Vec<String>,
    
    /// Service weight for load balancing
    pub weight: u32,
    
    /// Protocol used by service (HTTP, gRPC, etc.)
    pub protocol: String,
    
    /// Additional custom metadata
    pub custom: HashMap<String, String>,
}

impl Default for ServiceMetadata {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            tags: Vec::new(),
            weight: 100,
            protocol: "http".to_string(),
            custom: HashMap::new(),
        }
    }
}

/// Service discovery events
#[derive(Debug, Clone)]
pub enum ServiceDiscoveryEvent {
    ServiceRegistered {
        service: ServiceInstance,
    },
    ServiceDeregistered {
        service_id: ServiceId,
        node_id: NodeId,
    },
    ServiceHealthChanged {
        service_id: ServiceId,
        node_id: NodeId,
        old_status: HealthStatus,
        new_status: HealthStatus,
    },
    ServiceExpired {
        service_id: ServiceId,
        node_id: NodeId,
    },
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: RwLock::new(HashMap::new()),
            services_by_node: RwLock::new(HashMap::new()),
            service_metadata: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register a service instance
    pub async fn register(&self, instance: ServiceInstance) -> Result<()> {
        let mut services = self.services.write().await;
        let mut services_by_node = self.services_by_node.write().await;
        
        // Add to services map
        services
            .entry(instance.service_id.clone())
            .or_insert_with(Vec::new)
            .push(instance.clone());
        
        // Add to services by node map
        services_by_node
            .entry(instance.node_id)
            .or_insert_with(Vec::new)
            .push(instance.service_id.clone());
        
        debug!("Registered service: {} on node {}", 
               instance.service_id, instance.node_id);
        
        Ok(())
    }
    
    /// Deregister a service instance
    pub async fn deregister(&self, service_id: &ServiceId, node_id: NodeId) -> Result<bool> {
        let mut services = self.services.write().await;
        let mut services_by_node = self.services_by_node.write().await;
        
        let mut found = false;
        
        // Remove from services map
        if let Some(instances) = services.get_mut(service_id) {
            instances.retain(|instance| instance.node_id != node_id);
            
            if instances.is_empty() {
                services.remove(service_id);
            }
            
            found = true;
        }
        
        // Remove from services by node map
        if let Some(service_ids) = services_by_node.get_mut(&node_id) {
            service_ids.retain(|id| id != service_id);
            
            if service_ids.is_empty() {
                services_by_node.remove(&node_id);
            }
        }
        
        if found {
            debug!("Deregistered service: {} on node {}", service_id, node_id);
        }
        
        Ok(found)
    }
    
    /// Get all instances of a service
    pub async fn get_instances(&self, service_id: &ServiceId) -> Vec<ServiceInstance> {
        self.services
            .read()
            .await
            .get(service_id)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get healthy instances of a service
    pub async fn get_healthy_instances(&self, service_id: &ServiceId) -> Vec<ServiceInstance> {
        self.get_instances(service_id)
            .await
            .into_iter()
            .filter(|instance| instance.health_status == HealthStatus::Healthy)
            .collect()
    }
    
    /// Get all services
    pub async fn get_all_services(&self) -> HashMap<ServiceId, Vec<ServiceInstance>> {
        self.services.read().await.clone()
    }
    
    /// Update service health status
    pub async fn update_health_status(
        &self,
        service_id: &ServiceId,
        node_id: NodeId,
        status: HealthStatus,
    ) -> Result<Option<HealthStatus>> {
        let mut services = self.services.write().await;
        
        if let Some(instances) = services.get_mut(service_id) {
            for instance in instances {
                if instance.node_id == node_id {
                    let old_status = instance.health_status.clone();
                    instance.health_status = status.clone();
                    instance.last_seen = SystemTime::now();
                    
                    debug!("Updated health status for {} on {}: {:?} -> {:?}",
                           service_id, node_id, old_status, status);
                    
                    return Ok(Some(old_status));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Clean up expired services
    pub async fn cleanup_expired(&self, ttl: Duration) -> Vec<(ServiceId, NodeId)> {
        let mut services = self.services.write().await;
        let mut services_by_node = self.services_by_node.write().await;
        let mut expired = Vec::new();
        let now = SystemTime::now();
        
        // Find expired services
        services.retain(|service_id, instances| {
            instances.retain(|instance| {
                if let Ok(elapsed) = now.duration_since(instance.last_seen) {
                    if elapsed > ttl {
                        expired.push((service_id.clone(), instance.node_id));
                        false
                    } else {
                        true
                    }
                } else {
                    false
                }
            });
            
            !instances.is_empty()
        });
        
        // Clean up services by node map
        for (service_id, node_id) in &expired {
            if let Some(service_ids) = services_by_node.get_mut(node_id) {
                service_ids.retain(|id| id != service_id);
                
                if service_ids.is_empty() {
                    services_by_node.remove(node_id);
                }
            }
        }
        
        if !expired.is_empty() {
            info!("Cleaned up {} expired services", expired.len());
        }
        
        expired
    }
    
    /// Get service count
    pub async fn service_count(&self) -> usize {
        self.services.read().await.len()
    }
    
    /// Get instance count
    pub async fn instance_count(&self) -> usize {
        self.services
            .read()
            .await
            .values()
            .map(|instances| instances.len())
            .sum()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceDiscovery {
    /// Create a new service discovery instance
    pub async fn new(config: &ServiceDiscoveryConfig, node_id: NodeId) -> Result<Self> {
        let registry = Arc::new(ServiceRegistry::new());
        let (event_sender, _) = broadcast::channel(10000);
        
        Ok(Self {
            config: config.clone(),
            node_id,
            registry,
            event_sender,
            cleanup_task: None,
        })
    }
    
    /// Start service discovery
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting service discovery for node {}", self.node_id);
        
        // Start cleanup task
        let registry = Arc::clone(&self.registry);
        let config = self.config.clone();
        let event_sender = self.event_sender.clone();
        
        let cleanup_task = tokio::spawn(async move {
            Self::cleanup_task(registry, config, event_sender).await;
        });
        
        self.cleanup_task = Some(cleanup_task);
        
        info!("Service discovery started");
        Ok(())
    }
    
    /// Stop service discovery
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping service discovery");
        
        if let Some(task) = self.cleanup_task.take() {
            task.abort();
        }
        
        info!("Service discovery stopped");
        Ok(())
    }
    
    /// Register a service
    pub async fn register_service(&self, instance: ServiceInstance) -> Result<()> {
        self.registry.register(instance.clone()).await?;
        
        // Emit event
        let _ = self.event_sender.send(ServiceDiscoveryEvent::ServiceRegistered {
            service: instance,
        });
        
        Ok(())
    }
    
    /// Deregister a service
    pub async fn deregister_service(&self, service_id: &ServiceId) -> Result<()> {
        let found = self.registry.deregister(service_id, self.node_id).await?;
        
        if found {
            // Emit event
            let _ = self.event_sender.send(ServiceDiscoveryEvent::ServiceDeregistered {
                service_id: service_id.clone(),
                node_id: self.node_id,
            });
        }
        
        Ok(())
    }
    
    /// Discover services by name
    pub async fn discover_services(&self, service_name: &str) -> Result<Vec<ServiceInstance>> {
        let service_id = ServiceId::new(service_name, "default");
        let instances = self.registry.get_healthy_instances(&service_id).await;
        
        Ok(instances)
    }
    
    /// Get all services
    pub async fn get_all_services(&self) -> HashMap<ServiceId, Vec<ServiceInstance>> {
        self.registry.get_all_services().await
    }
    
    /// Update service health
    pub async fn update_service_health(
        &self,
        service_id: &ServiceId,
        node_id: NodeId,
        status: HealthStatus,
    ) -> Result<()> {
        if let Some(old_status) = self.registry
            .update_health_status(service_id, node_id, status.clone())
            .await?
        {
            // Emit event
            let _ = self.event_sender.send(ServiceDiscoveryEvent::ServiceHealthChanged {
                service_id: service_id.clone(),
                node_id,
                old_status,
                new_status: status,
            });
        }
        
        Ok(())
    }
    
    /// Subscribe to service discovery events
    pub fn subscribe(&self) -> broadcast::Receiver<ServiceDiscoveryEvent> {
        self.event_sender.subscribe()
    }
    
    /// Cleanup task for expired services
    async fn cleanup_task(
        registry: Arc<ServiceRegistry>,
        config: ServiceDiscoveryConfig,
        event_sender: broadcast::Sender<ServiceDiscoveryEvent>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(config.cleanup_interval));
        let ttl = Duration::from_secs(config.service_ttl);
        
        loop {
            interval.tick().await;
            
            let expired = registry.cleanup_expired(ttl).await;
            
            // Emit expiration events
            for (service_id, node_id) in expired {
                let _ = event_sender.send(ServiceDiscoveryEvent::ServiceExpired {
                    service_id,
                    node_id,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_service_registry() {
        let registry = ServiceRegistry::new();
        
        let service = ServiceInstance {
            service_id: ServiceId::new("test-service", "default"),
            node_id: NodeId::random(),
            address: "127.0.0.1:8080".parse().unwrap(),
            health_status: HealthStatus::Healthy,
            metadata: HashMap::new(),
            last_seen: SystemTime::now(),
        };
        
        // Register service
        registry.register(service.clone()).await.unwrap();
        
        // Get instances
        let instances = registry.get_instances(&service.service_id).await;
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].node_id, service.node_id);
        
        // Get healthy instances
        let healthy = registry.get_healthy_instances(&service.service_id).await;
        assert_eq!(healthy.len(), 1);
        
        // Deregister service
        let found = registry.deregister(&service.service_id, service.node_id).await.unwrap();
        assert!(found);
        
        // Should be empty now
        let instances = registry.get_instances(&service.service_id).await;
        assert!(instances.is_empty());
    }
    
    #[tokio::test]
    async fn test_service_discovery() {
        let config = ServiceDiscoveryConfig::default();
        let node_id = NodeId::random();
        
        let mut discovery = ServiceDiscovery::new(&config, node_id).await.unwrap();
        discovery.start().await.unwrap();
        
        let service = ServiceInstance {
            service_id: ServiceId::new("test-service", "default"),
            node_id,
            address: "127.0.0.1:8080".parse().unwrap(),
            health_status: HealthStatus::Healthy,
            metadata: HashMap::new(),
            last_seen: SystemTime::now(),
        };
        
        // Register service
        discovery.register_service(service.clone()).await.unwrap();
        
        // Discover services
        let instances = discovery.discover_services("test-service").await.unwrap();
        assert_eq!(instances.len(), 1);
        
        discovery.stop().await.unwrap();
    }
    
    #[test]
    fn test_service_instance_serialization() {
        let instance = ServiceInstance {
            service_id: ServiceId::new("test", "default"),
            node_id: NodeId::random(),
            address: "127.0.0.1:8080".parse().unwrap(),
            health_status: HealthStatus::Healthy,
            metadata: HashMap::new(),
            last_seen: SystemTime::now(),
        };
        
        let json = serde_json::to_string(&instance).unwrap();
        let parsed: ServiceInstance = serde_json::from_str(&json).unwrap();
        
        assert_eq!(instance.service_id, parsed.service_id);
        assert_eq!(instance.node_id, parsed.node_id);
        assert_eq!(instance.address, parsed.address);
    }
}