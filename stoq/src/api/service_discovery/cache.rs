use super::{DnsBinding, ServiceEndpoint};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

pub(super) struct CachedService {
    endpoint: ServiceEndpoint,
    deadline: Instant,
}

#[derive(Default)]
pub(super) struct DiscoveryState {
    pub binding: Option<DnsBinding>,
    pub started: bool,
    generation: u64,
    cache: HashMap<String, CachedService>,
    pub registered: HashMap<String, ServiceEndpoint>,
}

pub(super) fn valid_endpoint(endpoint: &ServiceEndpoint, now: SystemTime) -> bool {
    endpoint.expires_at > now && endpoint.port != 0 && endpoint.address.to_ipv4_mapped().is_none()
}

impl DiscoveryState {
    pub fn configure(&mut self, binding: DnsBinding) -> Result<()> {
        if self.started || self.binding.is_some() {
            return Err(anyhow!(
                "DNS binding is immutable and must precede first resolution"
            ));
        }
        self.binding = Some(binding);
        Ok(())
    }

    pub fn begin(&mut self, name: &str) -> Result<(DnsBinding, u64, Option<ServiceEndpoint>)> {
        self.started = true;
        let binding = self
            .binding
            .clone()
            .ok_or_else(|| anyhow!("DNS binding required"))?;
        let cached = self.cached_at(name, Instant::now(), SystemTime::now());
        Ok((binding, self.generation, cached))
    }

    pub fn cached_at(
        &mut self,
        name: &str,
        instant: Instant,
        wall: SystemTime,
    ) -> Option<ServiceEndpoint> {
        let cached = self
            .cache
            .get(name)
            .filter(|entry| entry.deadline > instant && valid_endpoint(&entry.endpoint, wall))
            .map(|entry| entry.endpoint.clone());
        if cached.is_none() {
            self.cache.remove(name);
        }
        cached
    }

    /// Caller holds the same mutation lock used by invalidate: check+insert is atomic.
    pub fn publish(
        &mut self,
        name: &str,
        generation: u64,
        endpoint: ServiceEndpoint,
        ttl: Duration,
    ) -> Result<ServiceEndpoint> {
        if self.generation != generation {
            return Err(anyhow!("discovery invalidated during resolution"));
        }
        let remaining = endpoint
            .expires_at
            .duration_since(SystemTime::now())
            .map_err(|_| anyhow!("DNS endpoint expired before publication"))?;
        let lifetime = ttl.min(remaining);
        if lifetime.is_zero() || !valid_endpoint(&endpoint, SystemTime::now()) {
            return Err(anyhow!("DNS endpoint has zero or invalid lifetime"));
        }
        let deadline = Instant::now()
            .checked_add(lifetime)
            .ok_or_else(|| anyhow!("DNS cache TTL overflow"))?;
        self.cache.insert(
            name.to_owned(),
            CachedService {
                endpoint: endpoint.clone(),
                deadline,
            },
        );
        Ok(endpoint)
    }

    pub fn invalidate(&mut self) {
        // Saturation would permit stale publication; checked increment instead fails closed.
        self.generation = self
            .generation
            .checked_add(1)
            .expect("discovery generation exhausted");
        self.cache.clear();
    }

    pub fn cached_names(&self) -> Vec<String> {
        self.cache
            .iter()
            .filter(|(_, entry)| {
                entry.deadline > Instant::now()
                    && valid_endpoint(&entry.endpoint, SystemTime::now())
            })
            .map(|(name, _)| name.clone())
            .collect()
    }
}
