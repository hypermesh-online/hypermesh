// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//! Locally bound service routing, with no implicit DNS or hardcoded fallback.
//! Context selection is not authenticated authority, proof of state, or a resource grant.

use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::net::Ipv6Addr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

mod cache;
mod context;
mod metrics;
mod types;
use cache::{valid_endpoint, DiscoveryState};
pub use context::{DiscoveryContext, DnsBinding};
use metrics::DiscoveryMetrics;
pub use metrics::DiscoveryStats;
pub use types::{ServiceEndpoint, ServiceMetadata, ServiceType};

/// Synchronous, bounded local provider callback contract.
/// Implementations must return promptly; callers must not bridge async runtimes by
/// nesting Tokio or block_on. No discovery/connection guard is held during callbacks.
/// A provider is responsible for enforcing its locally configured authority.
pub trait TrustChainDnsClient: Send + Sync {
    fn resolve(&self, service_name: &str) -> Result<Vec<ServiceEndpoint>>;
    fn is_available(&self) -> bool;
}

/// One immutable binding and cache per discovery instance; never process-global.
pub struct ServiceDiscovery {
    state: Mutex<DiscoveryState>,
    default_ttl: Duration,
    metrics: DiscoveryMetrics,
}

impl ServiceDiscovery {
    /// Unconfigured instances fail closed, including for previously cached/local names.
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            state: Mutex::new(DiscoveryState::default()),
            default_ttl,
            metrics: DiscoveryMetrics::default(),
        }
    }

    pub fn with_dns(default_ttl: Duration, binding: DnsBinding) -> Self {
        let mut state = DiscoveryState::default();
        state.binding = Some(binding);
        Self {
            state: Mutex::new(state),
            default_ttl,
            metrics: DiscoveryMetrics::default(),
        }
    }

    pub fn set_dns_binding(&self, binding: DnsBinding) -> Result<()> {
        self.state.lock().configure(binding)
    }

    /// Legacy caller remains callable, but cannot silently invent trusted context.
    pub fn set_trustchain_dns(&mut self, _client: Arc<dyn TrustChainDnsClient>) -> Result<()> {
        Err(anyhow!(
            "explicit DiscoveryContext required; use set_dns_binding"
        ))
    }

    pub fn resolve(&self, service_name: &str) -> Result<ServiceEndpoint> {
        self.metrics
            .total_resolutions
            .fetch_add(1, Ordering::Relaxed);
        let result = self.resolve_bound(service_name);
        if result.is_err() {
            self.metrics
                .failed_resolutions
                .fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    fn resolve_bound(&self, service_name: &str) -> Result<ServiceEndpoint> {
        let (binding, generation, cached) = self.state.lock().begin(service_name)?;
        if let Some(endpoint) = cached {
            self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(endpoint);
        }
        if self.default_ttl.is_zero() || !binding.client().is_available() {
            return Err(anyhow!("bound DNS provider unavailable or TTL is zero"));
        }
        let now = SystemTime::now();
        let endpoint = binding
            .client()
            .resolve(service_name)?
            .into_iter()
            .filter(|endpoint| valid_endpoint(endpoint, now))
            .min_by_key(|endpoint| endpoint.metadata.priority)
            .ok_or_else(|| anyhow!("bound DNS returned no valid live endpoints"))?;
        let published =
            self.state
                .lock()
                .publish(service_name, generation, endpoint, self.default_ttl)?;
        self.metrics.dns_resolutions.fetch_add(1, Ordering::Relaxed);
        Ok(published)
    }

    pub fn resolve_multiple(&self, service_names: &[&str]) -> Vec<Result<ServiceEndpoint>> {
        service_names
            .iter()
            .map(|name| self.resolve(name))
            .collect()
    }

    pub fn clear_cache(&self) {
        self.state.lock().invalidate();
    }

    /// Legacy explicit local registration bookkeeping only, never a DNS fallback.
    /// Routing still requires the configured provider to return the endpoint.
    pub fn add_hardcoded_endpoint(&self, endpoint: ServiceEndpoint) {
        let mut state = self.state.lock();
        state.invalidate();
        state.registered.insert(endpoint.name.clone(), endpoint);
    }

    pub fn get_metrics(&self) -> DiscoveryStats {
        self.metrics
            .snapshot(self.state.lock().cached_names().len())
    }

    pub fn list_cached_services(&self) -> Vec<String> {
        self.state.lock().cached_names()
    }

    pub fn list_hardcoded_services(&self) -> Vec<String> {
        self.state.lock().registered.keys().cloned().collect()
    }

    /// Register local metadata, not resolution authority or remote announcement.
    pub fn register_service(
        &self,
        service_type: ServiceType,
        address: Ipv6Addr,
        port: u16,
        metadata: ServiceMetadata,
    ) -> Result<()> {
        let endpoint = ServiceEndpoint {
            name: service_type.to_string(),
            address,
            port,
            server_name: None,
            metadata,
            expires_at: SystemTime::now()
                .checked_add(self.default_ttl)
                .ok_or_else(|| anyhow!("registration TTL overflow"))?,
        };
        if !valid_endpoint(&endpoint, SystemTime::now()) {
            return Err(anyhow!("invalid local registration endpoint"));
        }
        self.add_hardcoded_endpoint(endpoint);
        Ok(())
    }

    pub fn unregister_service(&self, service_name: &str) {
        let mut state = self.state.lock();
        state.invalidate();
        state.registered.remove(service_name);
    }

    pub fn registered_service_types(&self) -> Vec<ServiceType> {
        self.state
            .lock()
            .registered
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

    #[deprecated(since = "0.1.0", note = "use resolve instead")]
    pub fn resolve_service(&self, name: &str) -> Result<ServiceEndpoint> {
        self.resolve(name)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/network_scoped_discovery.rs"]
mod network_scoped_discovery;
