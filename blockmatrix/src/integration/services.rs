//! Service registry and discovery for HyperMesh platform

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error, debug};

use crate::{IntegrationResult, IntegrationError};

/// Service registry for managing platform services
pub struct ServiceRegistry {
    /// Registered services
    services: Arc<RwLock<HashMap<String, RegisteredService>>>,
    /// Service discovery cache
    discovery_cache: Arc<RwLock<HashMap<String, Vec<ServiceEndpoint>>>>,
}

/// Service discovery interface
pub struct ServiceDiscovery {
    /// Reference to service registry
    registry: Arc<ServiceRegistry>,
}

/// Registered service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredService {
    /// Service endpoint information
    pub endpoint: ServiceEndpoint,
    /// Service metadata
    pub metadata: ServiceMetadata,
    /// Registration timestamp
    pub registered_at: SystemTime,
    /// Last health check timestamp
    pub last_health_check: Option<SystemTime>,
    /// Service health status
    pub health_status: ServiceHealthStatus,
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service type/name
    pub service_type: String,
    /// Service address
    pub address: String,
    /// Service port
    pub port: u16,
    /// Health check path
    pub health_check_path: String,
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
    /// Service priority
    pub priority: u32,
    /// Custom attributes
    pub attributes: HashMap<String, String>,
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceHealthStatus {
    /// Service is healthy
    Healthy,
    /// Service is degraded but functional
    Degraded,
    /// Service is unhealthy
    Unhealthy,
    /// Health status unknown
    Unknown,
}

/// Service registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistration {
    /// Service ID (unique identifier)
    pub service_id: String,
    /// Service endpoint
    pub endpoint: ServiceEndpoint,
    /// Service metadata
    pub metadata: ServiceMetadata,
    /// Registration TTL in seconds
    pub ttl: u64,
}

/// Service query for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceQuery {
    /// Service type to discover
    pub service_type: String,
    /// Required tags
    pub required_tags: Option<Vec<String>>,
    /// Preferred tags
    pub preferred_tags: Option<Vec<String>>,
    /// Minimum health status required
    pub min_health: Option<ServiceHealthStatus>,
    /// Maximum number of results
    pub limit: Option<usize>,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            discovery_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a service
    pub async fn register_service(&self, service_id: String, endpoint: ServiceEndpoint) -> IntegrationResult<()> {
        info!("Registering service: {} at {}:{}", service_id, endpoint.address, endpoint.port);
        
        let registered_service = RegisteredService {
            endpoint,
            metadata: ServiceMetadata::default(),
            registered_at: SystemTime::now(),
            last_health_check: None,
            health_status: ServiceHealthStatus::Unknown,
        };
        
        let mut services = self.services.write().await;
        services.insert(service_id.clone(), registered_service);
        
        // Invalidate discovery cache for this service type
        let mut cache = self.discovery_cache.write().await;
        cache.remove(&registered_service.endpoint.service_type);
        
        info!("Service {} registered successfully", service_id);
        Ok(())
    }
    
    /// Register service with full metadata
    pub async fn register_service_with_metadata(
        &self,
        registration: ServiceRegistration,
    ) -> IntegrationResult<()> {
        info!("Registering service with metadata: {}", registration.service_id);
        
        let registered_service = RegisteredService {
            endpoint: registration.endpoint.clone(),
            metadata: registration.metadata,
            registered_at: SystemTime::now(),
            last_health_check: None,
            health_status: ServiceHealthStatus::Unknown,
        };
        
        let mut services = self.services.write().await;
        services.insert(registration.service_id.clone(), registered_service);
        
        // Invalidate discovery cache
        let mut cache = self.discovery_cache.write().await;
        cache.remove(&registration.endpoint.service_type);
        
        info!("Service {} registered with metadata successfully", registration.service_id);
        Ok(())
    }
    
    /// Unregister a service
    pub async fn unregister_service(&self, service_id: &str) -> IntegrationResult<()> {
        info!("Unregistering service: {}", service_id);
        
        let mut services = self.services.write().await;
        if let Some(service) = services.remove(service_id) {
            // Invalidate discovery cache
            let mut cache = self.discovery_cache.write().await;
            cache.remove(&service.endpoint.service_type);
            
            info!("Service {} unregistered successfully", service_id);
        } else {
            warn!("Attempted to unregister non-existent service: {}", service_id);
        }
        
        Ok(())
    }
    
    /// Update service health status
    pub async fn update_service_health(
        &self,
        service_id: &str,
        health_status: ServiceHealthStatus,
    ) -> IntegrationResult<()> {
        debug!("Updating health status for service: {} -> {:?}", service_id, health_status);
        
        let mut services = self.services.write().await;
        if let Some(service) = services.get_mut(service_id) {
            service.health_status = health_status;
            service.last_health_check = Some(SystemTime::now());
            
            // Invalidate discovery cache for this service type
            let mut cache = self.discovery_cache.write().await;
            cache.remove(&service.endpoint.service_type);
        } else {
            return Err(IntegrationError::ServiceRegistry {
                message: format!("Service '{}' not found for health update", service_id),
            });
        }
        
        Ok(())
    }
    
    /// Get service by ID
    pub async fn get_service(&self, service_id: &str) -> Option<RegisteredService> {
        let services = self.services.read().await;
        services.get(service_id).cloned()
    }
    
    /// Discover services by query
    pub async fn discover_services(&self, query: ServiceQuery) -> Vec<ServiceEndpoint> {
        debug!("Discovering services for query: {:?}", query);
        
        // Check cache first
        {
            let cache = self.discovery_cache.read().await;
            if let Some(cached_endpoints) = cache.get(&query.service_type) {
                return self.filter_services(cached_endpoints.clone(), &query);
            }
        }
        
        // Build results from registry
        let services = self.services.read().await;
        let mut matching_services = Vec::new();
        
        for service in services.values() {
            if service.endpoint.service_type == query.service_type {
                // Check health requirement
                if let Some(min_health) = &query.min_health {
                    if !self.meets_health_requirement(&service.health_status, min_health) {
                        continue;
                    }
                }
                
                // Check required tags
                if let Some(required_tags) = &query.required_tags {
                    if !required_tags.iter().all(|tag| service.metadata.tags.contains(tag)) {
                        continue;
                    }
                }
                
                matching_services.push(service.endpoint.clone());
            }
        }
        
        // Sort by priority and weight
        matching_services.sort_by(|a, b| {
            // This would require access to metadata, simplified for now
            a.service_type.cmp(&b.service_type)
        });
        
        // Apply limit
        if let Some(limit) = query.limit {
            matching_services.truncate(limit);
        }
        
        // Update cache
        {
            let mut cache = self.discovery_cache.write().await;
            cache.insert(query.service_type.clone(), matching_services.clone());
        }
        
        debug!("Discovered {} services for type: {}", matching_services.len(), query.service_type);
        matching_services
    }
    
    /// List all registered services
    pub async fn list_services(&self) -> Vec<(String, RegisteredService)> {
        let services = self.services.read().await;
        services.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
    
    /// Get service registry statistics
    pub async fn get_statistics(&self) -> ServiceRegistryStatistics {
        let services = self.services.read().await;
        let total_services = services.len();
        
        let mut healthy_count = 0;
        let mut unhealthy_count = 0;
        let mut degraded_count = 0;
        let mut unknown_count = 0;
        let mut service_types = HashMap::new();
        
        for service in services.values() {
            match service.health_status {
                ServiceHealthStatus::Healthy => healthy_count += 1,
                ServiceHealthStatus::Unhealthy => unhealthy_count += 1,
                ServiceHealthStatus::Degraded => degraded_count += 1,
                ServiceHealthStatus::Unknown => unknown_count += 1,
            }
            
            *service_types.entry(service.endpoint.service_type.clone()).or_insert(0) += 1;
        }
        
        ServiceRegistryStatistics {
            total_services,
            healthy_services: healthy_count,
            unhealthy_services: unhealthy_count,
            degraded_services: degraded_count,
            unknown_services: unknown_count,
            service_types,
        }
    }
    
    /// Cleanup expired services (if TTL-based registration was used)
    pub async fn cleanup_expired_services(&self) -> IntegrationResult<()> {
        info!("Cleaning up expired services");
        
        let mut services = self.services.write().await;
        let now = SystemTime::now();
        let mut to_remove = Vec::new();
        
        for (service_id, service) in services.iter() {
            // Check if service is considered stale (no health check in 5 minutes)
            if let Some(last_check) = service.last_health_check {
                if now.duration_since(last_check).unwrap_or(Duration::from_secs(0)) > Duration::from_secs(300) {
                    warn!("Service {} is stale, marking for removal", service_id);
                    to_remove.push(service_id.clone());
                }
            } else if now.duration_since(service.registered_at).unwrap_or(Duration::from_secs(0)) > Duration::from_secs(600) {
                warn!("Service {} never had health check and is old, marking for removal", service_id);
                to_remove.push(service_id.clone());
            }
        }
        
        // Remove stale services
        for service_id in &to_remove {
            services.remove(service_id);
            info!("Removed expired service: {}", service_id);
        }
        
        // Clear discovery cache
        if !to_remove.is_empty() {
            let mut cache = self.discovery_cache.write().await;
            cache.clear();
        }
        
        info!("Cleaned up {} expired services", to_remove.len());
        Ok(())
    }
    
    /// Helper to filter services by query parameters
    fn filter_services(&self, services: Vec<ServiceEndpoint>, query: &ServiceQuery) -> Vec<ServiceEndpoint> {
        let mut filtered = services;
        
        // Apply limit
        if let Some(limit) = query.limit {
            filtered.truncate(limit);
        }
        
        filtered
    }
    
    /// Helper to check if health status meets requirement
    fn meets_health_requirement(&self, actual: &ServiceHealthStatus, required: &ServiceHealthStatus) -> bool {
        match required {
            ServiceHealthStatus::Unknown => true, // Any status acceptable
            ServiceHealthStatus::Degraded => matches!(actual, ServiceHealthStatus::Healthy | ServiceHealthStatus::Degraded),
            ServiceHealthStatus::Healthy => matches!(actual, ServiceHealthStatus::Healthy),
            ServiceHealthStatus::Unhealthy => false, // Never want unhealthy services
        }
    }
}

impl ServiceDiscovery {
    /// Create a new service discovery client
    pub fn new(registry: Arc<ServiceRegistry>) -> Self {
        Self { registry }
    }
    
    /// Discover services of a specific type
    pub async fn discover(&self, service_type: &str) -> Vec<ServiceEndpoint> {
        let query = ServiceQuery {
            service_type: service_type.to_string(),
            required_tags: None,
            preferred_tags: None,
            min_health: Some(ServiceHealthStatus::Degraded), // At least degraded
            limit: None,
        };
        
        self.registry.discover_services(query).await
    }
    
    /// Discover healthy services only
    pub async fn discover_healthy(&self, service_type: &str) -> Vec<ServiceEndpoint> {
        let query = ServiceQuery {
            service_type: service_type.to_string(),
            required_tags: None,
            preferred_tags: None,
            min_health: Some(ServiceHealthStatus::Healthy),
            limit: None,
        };
        
        self.registry.discover_services(query).await
    }
    
    /// Discover services with specific tags
    pub async fn discover_with_tags(&self, service_type: &str, tags: Vec<String>) -> Vec<ServiceEndpoint> {
        let query = ServiceQuery {
            service_type: service_type.to_string(),
            required_tags: Some(tags),
            preferred_tags: None,
            min_health: Some(ServiceHealthStatus::Degraded),
            limit: None,
        };
        
        self.registry.discover_services(query).await
    }
}

/// Service registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistryStatistics {
    /// Total number of registered services
    pub total_services: usize,
    /// Number of healthy services
    pub healthy_services: usize,
    /// Number of unhealthy services
    pub unhealthy_services: usize,
    /// Number of degraded services
    pub degraded_services: usize,
    /// Number of services with unknown health
    pub unknown_services: usize,
    /// Count by service type
    pub service_types: HashMap<String, usize>,
}

impl Default for ServiceMetadata {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            tags: Vec::new(),
            weight: 100,
            priority: 1,
            attributes: HashMap::new(),
        }
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}