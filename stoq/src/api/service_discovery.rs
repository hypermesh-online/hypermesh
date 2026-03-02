// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Service Discovery Module for STOQ
//!
//! Integrates with TrustChain DNS for service resolution.
//! Provides fallback chain: DNS → Cache → Hardcoded

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

/// Service type enumeration for discovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    /// Caesar wallet/exchange service
    Caesar,
    /// BlockMatrix coordination service
    BlockMatrix,
    /// TrustChain CA service
    TrustChain,
    /// Catalog VM service
    Catalog,
    /// STOQ transport service
    Stoq,
    /// HyperMesh dashboard
    HyperMesh,
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ServiceType::Caesar => "caesar",
            ServiceType::BlockMatrix => "blockmatrix",
            ServiceType::TrustChain => "trustchain",
            ServiceType::Catalog => "catalog",
            ServiceType::Stoq => "stoq",
            ServiceType::HyperMesh => "hypermesh",
        };
        write!(f, "{s}")
    }
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service name
    pub name: String,

    /// IPv6 address
    pub address: Ipv6Addr,

    /// Port number
    pub port: u16,

    /// Optional server name for TLS
    pub server_name: Option<String>,

    /// Service metadata
    pub metadata: ServiceMetadata,

    /// Time when this record expires
    pub expires_at: SystemTime,
}

/// Service metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceMetadata {
    /// Service version
    pub version: Option<String>,

    /// Service capabilities
    pub capabilities: Vec<String>,

    /// Service priority (lower is higher priority)
    pub priority: u32,

    /// Service weight for load balancing
    pub weight: u32,

    /// Matrix position in Block-MATRIX topology
    pub matrix_position: Option<(u32, u32, u32)>,
}

/// Cached service record
#[derive(Debug, Clone)]
struct CachedService {
    endpoint: ServiceEndpoint,
    cached_at: SystemTime,
    ttl: Duration,
}

/// TrustChain DNS client trait
pub trait TrustChainDnsClient: Send + Sync {
    /// Resolve service name to endpoints
    fn resolve(&self, service_name: &str) -> Result<Vec<ServiceEndpoint>>;

    /// Check if TrustChain DNS is available
    fn is_available(&self) -> bool;
}

/// Service discovery manager
pub struct ServiceDiscovery {
    /// Cache of resolved services
    cache: Arc<DashMap<String, CachedService>>,

    /// TrustChain DNS client
    trustchain_dns: Option<Arc<dyn TrustChainDnsClient>>,

    /// Default cache TTL
    default_ttl: Duration,

    /// Hardcoded fallback endpoints
    hardcoded_endpoints: Arc<RwLock<std::collections::HashMap<String, ServiceEndpoint>>>,

    /// Discovery metrics
    metrics: Arc<DiscoveryMetrics>,
}

/// Discovery metrics
struct DiscoveryMetrics {
    /// Total resolutions
    total_resolutions: std::sync::atomic::AtomicU64,

    /// Cache hits
    cache_hits: std::sync::atomic::AtomicU64,

    /// DNS resolutions
    dns_resolutions: std::sync::atomic::AtomicU64,

    /// Fallback resolutions
    fallback_resolutions: std::sync::atomic::AtomicU64,

    /// Failed resolutions
    failed_resolutions: std::sync::atomic::AtomicU64,
}

impl ServiceDiscovery {
    /// Create new service discovery instance
    pub fn new(default_ttl: Duration) -> Self {
        let mut hardcoded = std::collections::HashMap::new();

        // Add default hardcoded endpoints
        hardcoded.insert(
            "trustchain".to_string(),
            ServiceEndpoint {
                name: "trustchain".to_string(),
                address: Ipv6Addr::LOCALHOST,
                port: 9293,
                server_name: Some("trust.hypermesh.online".to_string()),
                metadata: ServiceMetadata {
                    version: Some("1.0.0".to_string()),
                    capabilities: vec!["ca".to_string(), "verification".to_string()],
                    priority: 1,
                    weight: 100,
                    matrix_position: Some((0, 0, 0)),
                },
                expires_at: SystemTime::now() + Duration::from_secs(86400), // 24 hours
            },
        );

        hardcoded.insert(
            "hypermesh".to_string(),
            ServiceEndpoint {
                name: "hypermesh".to_string(),
                address: Ipv6Addr::LOCALHOST,
                port: 9292,
                server_name: Some("hypermesh.hypermesh.online".to_string()),
                metadata: ServiceMetadata {
                    version: Some("1.0.0".to_string()),
                    capabilities: vec!["compute".to_string(), "storage".to_string()],
                    priority: 1,
                    weight: 100,
                    matrix_position: Some((0, 0, 1)),
                },
                expires_at: SystemTime::now() + Duration::from_secs(86400),
            },
        );

        hardcoded.insert(
            "caesar".to_string(),
            ServiceEndpoint {
                name: "caesar".to_string(),
                address: Ipv6Addr::LOCALHOST,
                port: 9294,
                server_name: Some("caesar.hypermesh.online".to_string()),
                metadata: ServiceMetadata {
                    version: Some("1.0.0".to_string()),
                    capabilities: vec!["wallet".to_string(), "exchange".to_string()],
                    priority: 1,
                    weight: 100,
                    matrix_position: Some((0, 0, 2)),
                },
                expires_at: SystemTime::now() + Duration::from_secs(86400),
            },
        );

        hardcoded.insert(
            "catalog".to_string(),
            ServiceEndpoint {
                name: "catalog".to_string(),
                address: Ipv6Addr::LOCALHOST,
                port: 9295,
                server_name: Some("catalog.hypermesh.online".to_string()),
                metadata: ServiceMetadata {
                    version: Some("1.0.0".to_string()),
                    capabilities: vec!["vm".to_string(), "assets".to_string()],
                    priority: 1,
                    weight: 100,
                    matrix_position: Some((0, 0, 3)),
                },
                expires_at: SystemTime::now() + Duration::from_secs(86400),
            },
        );

        Self {
            cache: Arc::new(DashMap::new()),
            trustchain_dns: None,
            default_ttl,
            hardcoded_endpoints: Arc::new(RwLock::new(hardcoded)),
            metrics: Arc::new(DiscoveryMetrics {
                total_resolutions: std::sync::atomic::AtomicU64::new(0),
                cache_hits: std::sync::atomic::AtomicU64::new(0),
                dns_resolutions: std::sync::atomic::AtomicU64::new(0),
                fallback_resolutions: std::sync::atomic::AtomicU64::new(0),
                failed_resolutions: std::sync::atomic::AtomicU64::new(0),
            }),
        }
    }

    /// Set TrustChain DNS client
    pub fn set_trustchain_dns(&mut self, client: Arc<dyn TrustChainDnsClient>) {
        self.trustchain_dns = Some(client);
        info!("TrustChain DNS client configured for service discovery");
    }

    /// Resolve service name to endpoint
    /// Fallback chain: DNS → Cache → Hardcoded
    pub fn resolve(&self, service_name: &str) -> Result<ServiceEndpoint> {
        self.metrics
            .total_resolutions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 1. Check cache first
        if let Some(cached) = self.cache.get(service_name) {
            if cached.cached_at + cached.ttl > SystemTime::now() {
                self.metrics
                    .cache_hits
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                debug!("Service '{}' resolved from cache", service_name);
                return Ok(cached.endpoint.clone());
            }
        }

        // 2. Try TrustChain DNS if available
        if let Some(ref dns) = self.trustchain_dns {
            if dns.is_available() {
                match dns.resolve(service_name) {
                    Ok(endpoints) if !endpoints.is_empty() => {
                        self.metrics
                            .dns_resolutions
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                        // Use the highest priority (lowest number) endpoint
                        // Safety: endpoints is non-empty (checked above), so min_by_key always returns Some
                        let best_endpoint =
                            match endpoints.into_iter().min_by_key(|e| e.metadata.priority) {
                                Some(endpoint) => endpoint,
                                None => {
                                    // This should never happen since we verified endpoints is non-empty
                                    warn!(
                                    "Unexpected: min_by_key returned None for non-empty endpoints"
                                );
                                    return Err(anyhow!("Failed to select best endpoint"));
                                }
                            };

                        // Cache the result
                        let cached = CachedService {
                            endpoint: best_endpoint.clone(),
                            cached_at: SystemTime::now(),
                            ttl: self.default_ttl,
                        };
                        self.cache.insert(service_name.to_string(), cached);

                        info!(
                            "Service '{}' resolved via TrustChain DNS: [{}]:{}",
                            service_name, best_endpoint.address, best_endpoint.port
                        );

                        return Ok(best_endpoint);
                    }
                    Ok(_) => {
                        warn!(
                            "TrustChain DNS returned no endpoints for '{}'",
                            service_name
                        );
                    }
                    Err(e) => {
                        debug!(
                            "TrustChain DNS resolution failed for '{}': {}",
                            service_name, e
                        );
                    }
                }
            }
        }

        // 3. Fall back to hardcoded endpoints
        if let Some(endpoint) = self.hardcoded_endpoints.read().get(service_name) {
            self.metrics
                .fallback_resolutions
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // Cache the hardcoded result with shorter TTL
            let cached = CachedService {
                endpoint: endpoint.clone(),
                cached_at: SystemTime::now(),
                ttl: Duration::from_secs(60), // Short TTL for fallback
            };
            self.cache.insert(service_name.to_string(), cached);

            debug!(
                "Service '{}' resolved from hardcoded fallback: [{}]:{}",
                service_name, endpoint.address, endpoint.port
            );

            return Ok(endpoint.clone());
        }

        // 4. Resolution failed
        self.metrics
            .failed_resolutions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Err(anyhow!("Service '{service_name}' not found"))
    }

    /// Resolve multiple services
    pub fn resolve_multiple(&self, service_names: &[&str]) -> Vec<Result<ServiceEndpoint>> {
        service_names
            .iter()
            .map(|name| self.resolve(name))
            .collect()
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.clear();
        info!("Service discovery cache cleared");
    }

    /// Add or update a hardcoded endpoint
    pub fn add_hardcoded_endpoint(&self, endpoint: ServiceEndpoint) {
        let name = endpoint.name.clone();
        self.hardcoded_endpoints
            .write()
            .insert(name.clone(), endpoint);
        info!("Added/updated hardcoded endpoint for service '{}'", name);
    }

    /// Get discovery metrics
    pub fn get_metrics(&self) -> DiscoveryStats {
        DiscoveryStats {
            total_resolutions: self
                .metrics
                .total_resolutions
                .load(std::sync::atomic::Ordering::Relaxed),
            cache_hits: self
                .metrics
                .cache_hits
                .load(std::sync::atomic::Ordering::Relaxed),
            dns_resolutions: self
                .metrics
                .dns_resolutions
                .load(std::sync::atomic::Ordering::Relaxed),
            fallback_resolutions: self
                .metrics
                .fallback_resolutions
                .load(std::sync::atomic::Ordering::Relaxed),
            failed_resolutions: self
                .metrics
                .failed_resolutions
                .load(std::sync::atomic::Ordering::Relaxed),
            cache_size: self.cache.len(),
        }
    }

    /// List all cached services
    pub fn list_cached_services(&self) -> Vec<String> {
        self.cache.iter().map(|entry| entry.key().clone()).collect()
    }

    /// List all hardcoded services
    pub fn list_hardcoded_services(&self) -> Vec<String> {
        self.hardcoded_endpoints.read().keys().cloned().collect()
    }

    /// Register a local service for discovery by other nodes.
    ///
    /// Adds the service endpoint to the hardcoded registry and the cache,
    /// making it immediately resolvable. Remote registration (announcing
    /// to peers) requires the gossip or mDNS layer in blockmatrix.
    pub fn register_service(
        &self,
        service_type: ServiceType,
        address: Ipv6Addr,
        port: u16,
        metadata: ServiceMetadata,
    ) -> Result<()> {
        let name = service_type.to_string();
        let endpoint = ServiceEndpoint {
            name: name.clone(),
            address,
            port,
            server_name: None,
            metadata,
            expires_at: SystemTime::now() + self.default_ttl,
        };

        self.add_hardcoded_endpoint(endpoint);
        info!("Registered local service '{}' at [{}]:{}", name, address, port);
        Ok(())
    }

    /// Unregister a service.
    pub fn unregister_service(&self, service_name: &str) {
        self.hardcoded_endpoints.write().remove(service_name);
        self.cache.remove(service_name);
        info!("Unregistered service '{}'", service_name);
    }

    /// Get all registered service types.
    pub fn registered_service_types(&self) -> Vec<ServiceType> {
        let hardcoded = self.hardcoded_endpoints.read();
        hardcoded
            .keys()
            .filter_map(|name| match name.as_str() {
                "caesar" => Some(ServiceType::Caesar),
                "blockmatrix" => Some(ServiceType::BlockMatrix),
                "trustchain" => Some(ServiceType::TrustChain),
                "catalog" => Some(ServiceType::Catalog),
                "stoq" => Some(ServiceType::Stoq),
                "hypermesh" => Some(ServiceType::HyperMesh),
                _ => None,
            })
            .collect()
    }

    /// Backward compatibility: resolve_service -> resolve
    #[deprecated(since = "0.1.0", note = "use resolve instead")]
    pub fn resolve_service(&self, name: &str) -> Result<ServiceEndpoint> {
        self.resolve(name)
    }
}

/// Discovery statistics
#[derive(Debug)]
pub struct DiscoveryStats {
    pub total_resolutions: u64,
    pub cache_hits: u64,
    pub dns_resolutions: u64,
    pub fallback_resolutions: u64,
    pub failed_resolutions: u64,
    pub cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardcoded_resolution() {
        let discovery = ServiceDiscovery::new(Duration::from_secs(300));

        // Test hardcoded services
        let trustchain = discovery.resolve("trustchain").expect("test: expected success");
        assert_eq!(trustchain.port, 9293);

        let hypermesh = discovery.resolve("hypermesh").expect("test: expected success");
        assert_eq!(hypermesh.port, 9292);

        let caesar = discovery.resolve("caesar").expect("test: expected success");
        assert_eq!(caesar.port, 9294);

        let catalog = discovery.resolve("catalog").expect("test: expected success");
        assert_eq!(catalog.port, 9295);
    }

    #[test]
    fn test_cache_functionality() {
        let discovery = ServiceDiscovery::new(Duration::from_secs(300));

        // First resolution should use fallback
        let _ = discovery.resolve("trustchain").expect("test: expected success");
        let stats = discovery.get_metrics();
        assert_eq!(stats.fallback_resolutions, 1);
        assert_eq!(stats.cache_hits, 0);

        // Second resolution should use cache
        let _ = discovery.resolve("trustchain").expect("test: expected success");
        let stats = discovery.get_metrics();
        assert_eq!(stats.fallback_resolutions, 1);
        assert_eq!(stats.cache_hits, 1);
    }

    #[test]
    fn test_unknown_service() {
        let discovery = ServiceDiscovery::new(Duration::from_secs(300));

        let result = discovery.resolve("unknown_service");
        assert!(result.is_err());

        let stats = discovery.get_metrics();
        assert_eq!(stats.failed_resolutions, 1);
    }

    #[test]
    fn test_add_hardcoded_endpoint() {
        let discovery = ServiceDiscovery::new(Duration::from_secs(300));

        let new_endpoint = ServiceEndpoint {
            name: "custom".to_string(),
            address: Ipv6Addr::LOCALHOST,
            port: 9999,
            server_name: None,
            metadata: ServiceMetadata::default(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
        };

        discovery.add_hardcoded_endpoint(new_endpoint);

        let resolved = discovery.resolve("custom").expect("test: expected success");
        assert_eq!(resolved.port, 9999);
    }

    #[test]
    fn test_register_service() {
        let discovery = ServiceDiscovery::new(Duration::from_secs(300));

        discovery
            .register_service(
                ServiceType::Stoq,
                Ipv6Addr::LOCALHOST,
                9292,
                ServiceMetadata {
                    version: Some("1.0.0".to_string()),
                    capabilities: vec!["transport".to_string()],
                    priority: 1,
                    weight: 100,
                    matrix_position: Some((5, 10, 15)),
                },
            )
            .expect("test: register should succeed");

        let resolved = discovery.resolve("stoq").expect("test: resolve registered service");
        assert_eq!(resolved.port, 9292);
        assert_eq!(resolved.address, Ipv6Addr::LOCALHOST);
    }

    #[test]
    fn test_unregister_service() {
        let discovery = ServiceDiscovery::new(Duration::from_secs(300));

        // Resolve a hardcoded service first
        let _ = discovery.resolve("caesar").expect("test: resolve caesar");

        // Unregister it
        discovery.unregister_service("caesar");

        // Should now fail
        assert!(discovery.resolve("caesar").is_err());
    }

    #[test]
    fn test_registered_service_types() {
        let discovery = ServiceDiscovery::new(Duration::from_secs(300));

        let types = discovery.registered_service_types();
        assert!(types.contains(&ServiceType::TrustChain));
        assert!(types.contains(&ServiceType::HyperMesh));
        assert!(types.contains(&ServiceType::Caesar));
        assert!(types.contains(&ServiceType::Catalog));
    }

    #[test]
    fn test_list_services() {
        let discovery = ServiceDiscovery::new(Duration::from_secs(300));

        // Resolve some services to populate cache
        let _ = discovery.resolve("trustchain");
        let _ = discovery.resolve("hypermesh");

        let cached = discovery.list_cached_services();
        assert!(cached.contains(&"trustchain".to_string()));
        assert!(cached.contains(&"hypermesh".to_string()));

        let hardcoded = discovery.list_hardcoded_services();
        assert!(hardcoded.contains(&"trustchain".to_string()));
        assert!(hardcoded.contains(&"hypermesh".to_string()));
        assert!(hardcoded.contains(&"caesar".to_string()));
        assert!(hardcoded.contains(&"catalog".to_string()));
    }
}
