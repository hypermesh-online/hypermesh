//! Embedded DNS Resolution for STOQ Transport
//! 
//! Provides DNS resolution directly embedded in the transport layer,
//! eliminating external DNS dependencies for Internet 2.0.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::net::Ipv6Addr;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use dashmap::DashMap;

use crate::config::HyperMeshServerConfig;
use crate::authority::TrustChainAuthorityLayer;

/// Embedded DNS resolver for STOQ transport
pub struct EmbeddedDnsResolver {
    /// Configuration
    config: Arc<HyperMeshServerConfig>,
    
    /// TrustChain integration for DNS resolution
    trustchain: Arc<TrustChainAuthorityLayer>,
    
    /// DNS resolution cache
    dns_cache: Arc<DashMap<String, CachedDnsResult>>,
    
    /// DNS resolution statistics
    stats: Arc<RwLock<DnsStats>>,
    
    /// Static DNS mappings for Internet 2.0 infrastructure
    static_mappings: Arc<HashMap<String, Vec<Ipv6Addr>>>,
}

/// DNS resolution result
#[derive(Debug, Clone)]
pub struct DnsResolutionResult {
    /// Domain name queried
    pub domain: String,
    
    /// Resolved IPv6 addresses
    pub addresses: Vec<Ipv6Addr>,
    
    /// Resolution time
    pub resolution_time: Duration,
    
    /// Resolution source
    pub source: DnsSource,
    
    /// TTL for the result
    pub ttl: Duration,
    
    /// Resolution timestamp
    pub resolved_at: Instant,
}

/// DNS resolution source
#[derive(Debug, Clone, PartialEq)]
pub enum DnsSource {
    /// Static mapping (built-in Internet 2.0 infrastructure)
    Static,
    /// Cache hit
    Cache,
    /// TrustChain embedded resolver
    TrustChain,
    /// Fallback to system resolver (development only)
    System,
}

/// Cached DNS result
#[derive(Debug, Clone)]
struct CachedDnsResult {
    addresses: Vec<Ipv6Addr>,
    cached_at: Instant,
    ttl: Duration,
}

/// DNS resolution statistics
#[derive(Debug, Clone, Default)]
struct DnsStats {
    total_queries: u64,
    successful_queries: u64,
    failed_queries: u64,
    cache_hits: u64,
    cache_misses: u64,
    static_hits: u64,
    trustchain_queries: u64,
    system_fallback_queries: u64,
    avg_resolution_time_ms: f64,
}

impl EmbeddedDnsResolver {
    /// Create new embedded DNS resolver
    pub async fn new(
        config: Arc<HyperMeshServerConfig>,
        trustchain: Arc<TrustChainAuthorityLayer>
    ) -> Result<Self> {
        info!("🌐 Initializing Embedded DNS Resolver for STOQ transport");
        info!("   Features: IPv6-only resolution, TrustChain integration, Internet 2.0 infrastructure");
        
        // Initialize static mappings for Internet 2.0 infrastructure
        let static_mappings = Self::create_static_mappings(&config);
        
        Ok(Self {
            config,
            trustchain,
            dns_cache: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(DnsStats::default())),
            static_mappings: Arc::new(static_mappings),
        })
    }
    
    /// Create static DNS mappings for Internet 2.0 infrastructure
    fn create_static_mappings(config: &HyperMeshServerConfig) -> HashMap<String, Vec<Ipv6Addr>> {
        let server_addr = config.global.bind_address;
        let mut mappings = HashMap::new();
        
        // Core HyperMesh infrastructure domains
        mappings.insert("hypermesh.online".to_string(), vec![server_addr]);
        mappings.insert("stoq.hypermesh.online".to_string(), vec![server_addr]);
        mappings.insert("catalog.hypermesh.online".to_string(), vec![server_addr]);
        mappings.insert("trust.hypermesh.online".to_string(), vec![server_addr]);
        mappings.insert("caesar.hypermesh.online".to_string(), vec![server_addr]);
        
        // Local development domains
        mappings.insert("localhost".to_string(), vec![Ipv6Addr::LOCALHOST]);
        mappings.insert("ip6-localhost".to_string(), vec![Ipv6Addr::LOCALHOST]);
        mappings.insert("ip6-loopback".to_string(), vec![Ipv6Addr::LOCALHOST]);
        
        // STOQ protocol domains
        mappings.insert("stoq.local".to_string(), vec![server_addr]);
        mappings.insert("hypermesh.local".to_string(), vec![server_addr]);
        mappings.insert("trustchain.local".to_string(), vec![server_addr]);
        
        info!("📋 Created {} static DNS mappings for Internet 2.0 infrastructure", mappings.len());
        mappings
    }
    
    /// Resolve domain to IPv6 addresses
    pub async fn resolve_ipv6(&self, domain: &str) -> Result<Vec<Ipv6Addr>> {
        let start_time = Instant::now();
        
        debug!("🔍 Resolving domain: {}", domain);
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_queries += 1;
        }
        
        // Step 1: Check static mappings first
        if let Some(addresses) = self.static_mappings.get(domain) {
            let resolution_time = start_time.elapsed();
            
            {
                let mut stats = self.stats.write().await;
                stats.successful_queries += 1;
                stats.static_hits += 1;
                self.update_avg_resolution_time(&mut stats, resolution_time);
            }
            
            debug!("✅ Static DNS mapping: {} -> {} addresses", domain, addresses.len());
            return Ok(addresses.clone());
        }
        
        // Step 2: Check cache if enabled
        if self.config.stoq.dns.enable_caching {
            if let Some(cached) = self.dns_cache.get(domain) {
                if cached.cached_at.elapsed() < cached.ttl {
                    let resolution_time = start_time.elapsed();
                    
                    {
                        let mut stats = self.stats.write().await;
                        stats.successful_queries += 1;
                        stats.cache_hits += 1;
                        self.update_avg_resolution_time(&mut stats, resolution_time);
                    }
                    
                    debug!("✅ DNS cache hit: {} -> {} addresses", domain, cached.addresses.len());
                    return Ok(cached.addresses.clone());
                } else {
                    // Remove expired cache entry
                    self.dns_cache.remove(domain);
                }
            }
            
            let mut stats = self.stats.write().await;
            stats.cache_misses += 1;
        }
        
        // Step 3: Resolve through TrustChain if configured
        if self.config.stoq.dns.use_embedded_resolver {
            match self.trustchain.resolve_domain(domain).await {
                Ok(addresses) => {
                    let resolution_time = start_time.elapsed();
                    
                    // Cache result if enabled
                    if self.config.stoq.dns.enable_caching {
                        let cached_result = CachedDnsResult {
                            addresses: addresses.clone(),
                            cached_at: start_time,
                            ttl: self.config.stoq.dns.cache_ttl,
                        };
                        self.dns_cache.insert(domain.to_string(), cached_result);
                    }
                    
                    {
                        let mut stats = self.stats.write().await;
                        stats.successful_queries += 1;
                        stats.trustchain_queries += 1;
                        self.update_avg_resolution_time(&mut stats, resolution_time);
                    }
                    
                    debug!("✅ TrustChain DNS resolution: {} -> {} addresses in {:?}", 
                           domain, addresses.len(), resolution_time);
                    return Ok(addresses);
                }
                Err(e) => {
                    debug!("⚠️  TrustChain DNS resolution failed for {}: {}", domain, e);
                }
            }
        }
        
        // Step 4: System fallback (development mode only)
        if self.config.deployment.mode == crate::config::DeploymentMode::Development {
            match self.system_dns_fallback(domain).await {
                Ok(addresses) => {
                    let resolution_time = start_time.elapsed();
                    
                    {
                        let mut stats = self.stats.write().await;
                        stats.successful_queries += 1;
                        stats.system_fallback_queries += 1;
                        self.update_avg_resolution_time(&mut stats, resolution_time);
                    }
                    
                    warn!("⚠️  System DNS fallback used for {}: {} addresses (development mode)", 
                          domain, addresses.len());
                    return Ok(addresses);
                }
                Err(e) => {
                    debug!("System DNS fallback failed for {}: {}", domain, e);
                }
            }
        }
        
        // Resolution failed
        {
            let mut stats = self.stats.write().await;
            stats.failed_queries += 1;
        }
        
        error!("❌ DNS resolution failed for domain: {}", domain);
        Err(anyhow!("DNS resolution failed for domain: {}", domain))
    }
    
    /// System DNS fallback (development mode only)
    async fn system_dns_fallback(&self, domain: &str) -> Result<Vec<Ipv6Addr>> {
        use tokio::net::lookup_host;
        
        // Resolve using system DNS
        let socket_addrs = lookup_host(format!("{}:0", domain)).await?;
        
        // Filter to IPv6 addresses only
        let ipv6_addresses: Vec<Ipv6Addr> = socket_addrs
            .filter_map(|addr| match addr {
                std::net::SocketAddr::V6(v6_addr) => Some(*v6_addr.ip()),
                std::net::SocketAddr::V4(_) => None, // Skip IPv4 addresses
            })
            .collect();
        
        if ipv6_addresses.is_empty() {
            return Err(anyhow!("No IPv6 addresses found for domain: {}", domain));
        }
        
        Ok(ipv6_addresses)
    }
    
    /// Add custom DNS mapping
    pub async fn add_custom_mapping(&self, domain: String, addresses: Vec<Ipv6Addr>) {
        // Add to cache with long TTL
        let cached_result = CachedDnsResult {
            addresses,
            cached_at: Instant::now(),
            ttl: Duration::from_secs(24 * 3600), // 24 hours for custom mappings
        };
        
        self.dns_cache.insert(domain.clone(), cached_result);
        info!("📝 Added custom DNS mapping: {}", domain);
    }
    
    /// Remove custom DNS mapping
    pub async fn remove_custom_mapping(&self, domain: &str) {
        if self.dns_cache.remove(domain).is_some() {
            info!("🗑️  Removed custom DNS mapping: {}", domain);
        }
    }
    
    /// Clear DNS cache
    pub async fn clear_cache(&self) {
        self.dns_cache.clear();
        info!("🧹 DNS cache cleared");
    }
    
    /// Get DNS resolution statistics
    pub async fn get_stats(&self) -> DnsStats {
        self.stats.read().await.clone()
    }
    
    /// Update average resolution time
    fn update_avg_resolution_time(&self, stats: &mut DnsStats, resolution_time: Duration) {
        let total_time = stats.avg_resolution_time_ms * (stats.successful_queries - 1) as f64;
        stats.avg_resolution_time_ms = (total_time + resolution_time.as_millis() as f64) / stats.successful_queries as f64;
    }
    
    /// Cleanup expired cache entries
    pub async fn cleanup_cache(&self) {
        if self.dns_cache.len() <= self.config.stoq.dns.cache_size {
            return;
        }
        
        let now = Instant::now();
        let mut expired_keys = Vec::new();
        
        for entry in self.dns_cache.iter() {
            if now.duration_since(entry.cached_at) > entry.ttl {
                expired_keys.push(entry.key().clone());
            }
        }
        
        let expired_count = expired_keys.len();
        for key in expired_keys {
            self.dns_cache.remove(&key);
        }
        
        debug!("🧹 Cleaned up {} expired DNS cache entries", expired_count);
    }
    
    /// Get cached domains
    pub async fn get_cached_domains(&self) -> Vec<String> {
        self.dns_cache.iter().map(|entry| entry.key().clone()).collect()
    }
    
    /// Get static mappings
    pub fn get_static_mappings(&self) -> &HashMap<String, Vec<Ipv6Addr>> {
        &self.static_mappings
    }
    
    /// Check if domain has IPv6 addresses
    pub async fn has_ipv6_addresses(&self, domain: &str) -> bool {
        match self.resolve_ipv6(domain).await {
            Ok(addresses) => !addresses.is_empty(),
            Err(_) => false,
        }
    }
    
    /// Resolve multiple domains concurrently
    pub async fn resolve_multiple(&self, domains: &[String]) -> HashMap<String, Result<Vec<Ipv6Addr>>> {
        use futures::future::join_all;
        
        let futures = domains.iter().map(|domain| {
            let domain = domain.clone();
            let resolver = self.clone();
            async move {
                let result = resolver.resolve_ipv6(&domain).await;
                (domain, result)
            }
        });
        
        let results = join_all(futures).await;
        results.into_iter().collect()
    }
    
    /// Preload DNS mappings for common domains
    pub async fn preload_common_domains(&self) -> Result<()> {
        let common_domains = vec![
            "hypermesh.online",
            "stoq.hypermesh.online",
            "catalog.hypermesh.online",
            "trust.hypermesh.online",
            "caesar.hypermesh.online",
        ];
        
        for domain in common_domains {
            if let Err(e) = self.resolve_ipv6(domain).await {
                warn!("Failed to preload domain {}: {}", domain, e);
            }
        }
        
        info!("📚 Preloaded common domain mappings");
        Ok(())
    }
}

// Allow cloning for use in async contexts
impl Clone for EmbeddedDnsResolver {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            trustchain: self.trustchain.clone(),
            dns_cache: self.dns_cache.clone(),
            stats: self.stats.clone(),
            static_mappings: self.static_mappings.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HyperMeshServerConfig;
    
    #[tokio::test]
    async fn test_static_dns_mappings() {
        // Test that static DNS mappings work correctly
    }
    
    #[tokio::test]
    async fn test_dns_caching() {
        // Test DNS caching functionality
    }
    
    #[tokio::test]
    async fn test_ipv6_only_resolution() {
        // Test that only IPv6 addresses are returned
    }
    
    #[tokio::test]
    async fn test_trustchain_integration() {
        // Test integration with TrustChain DNS resolution
    }
}