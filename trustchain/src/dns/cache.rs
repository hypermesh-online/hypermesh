//! DNS Cache Implementation
//! 
//! High-performance DNS cache with TTL support and automatic cleanup.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use trust_dns_proto::rr::RecordType;
use crate::errors::{DnsError, Result as TrustChainResult};
use super::{DnsResponse};

/// DNS cache entry
#[derive(Clone, Debug, Serialize, Deserialize)]
struct CacheEntry {
    /// Cached DNS response
    response: DnsResponse,
    /// Entry creation time
    created_at: SystemTime,
    /// TTL in seconds
    ttl: u32,
    /// Access count for LRU
    access_count: u64,
    /// Last access time
    last_accessed: SystemTime,
}

impl CacheEntry {
    fn new(response: DnsResponse, ttl: u32) -> Self {
        let now = SystemTime::now();
        Self {
            response,
            created_at: now,
            ttl,
            access_count: 1,
            last_accessed: now,
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed().unwrap_or(Duration::ZERO).as_secs() > self.ttl as u64
    }

    fn access(&mut self) -> DnsResponse {
        self.access_count += 1;
        self.last_accessed = SystemTime::now();
        self.response.clone()
    }
}

/// Cache key for DNS records
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct CacheKey {
    name: String,
    record_type: RecordType,
}

impl CacheKey {
    fn new(name: &str, record_type: RecordType) -> Self {
        Self {
            name: name.to_lowercase(),
            record_type,
        }
    }
}

/// DNS cache implementation
pub struct DnsCache {
    /// Cache storage
    cache: Arc<DashMap<CacheKey, CacheEntry>>,
    /// Default TTL for cached entries
    default_ttl: Duration,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Maximum cache size
    max_size: usize,
}

/// Cache statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: u64,
    pub memory_usage_bytes: u64,
    pub hit_rate: f64,
    pub last_cleanup: SystemTime,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            entries: 0,
            memory_usage_bytes: 0,
            hit_rate: 0.0,
            last_cleanup: SystemTime::now(),
        }
    }
}

impl DnsCache {
    /// Create new DNS cache
    pub async fn new(default_ttl: Duration) -> TrustChainResult<Self> {
        info!("Initializing DNS cache with default TTL: {:?}", default_ttl);

        Ok(Self {
            cache: Arc::new(DashMap::new()),
            default_ttl,
            stats: Arc::new(RwLock::new(CacheStats::default())),
            max_size: 10000, // Maximum 10K entries
        })
    }

    /// Get cached DNS response
    pub async fn get(&self, name: &str, record_type: RecordType) -> TrustChainResult<Option<DnsResponse>> {
        let key = CacheKey::new(name, record_type);
        
        if let Some(mut entry) = self.cache.get_mut(&key) {
            if !entry.is_expired() {
                let response = entry.access();
                
                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.hits += 1;
                    stats.hit_rate = stats.hits as f64 / (stats.hits + stats.misses) as f64;
                }

                debug!("Cache hit for {} ({:?})", name, record_type);
                return Ok(Some(response));
            } else {
                // Remove expired entry
                drop(entry);
                self.cache.remove(&key);
                debug!("Removed expired cache entry for {} ({:?})", name, record_type);
            }
        }

        // Update miss stats
        {
            let mut stats = self.stats.write().await;
            stats.misses += 1;
            stats.hit_rate = stats.hits as f64 / (stats.hits + stats.misses) as f64;
        }

        debug!("Cache miss for {} ({:?})", name, record_type);
        Ok(None)
    }

    /// Set cached DNS response
    pub async fn set(
        &self,
        name: &str,
        record_type: RecordType,
        response: &DnsResponse,
        ttl: u32,
    ) -> TrustChainResult<()> {
        let key = CacheKey::new(name, record_type);
        let effective_ttl = if ttl > 0 { ttl } else { self.default_ttl.as_secs() as u32 };
        
        // Check cache size limit
        if self.cache.len() >= self.max_size {
            self.evict_lru_entry().await;
        }

        let entry = CacheEntry::new(response.clone(), effective_ttl);
        self.cache.insert(key, entry);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.entries = self.cache.len() as u64;
            stats.memory_usage_bytes = self.estimate_memory_usage();
        }

        debug!("Cached DNS response for {} ({:?}) with TTL {}", name, record_type, effective_ttl);
        Ok(())
    }

    /// Remove entry from cache
    pub async fn remove(&self, name: &str, record_type: RecordType) -> TrustChainResult<bool> {
        let key = CacheKey::new(name, record_type);
        let removed = self.cache.remove(&key).is_some();

        if removed {
            let mut stats = self.stats.write().await;
            stats.entries = self.cache.len() as u64;
            stats.memory_usage_bytes = self.estimate_memory_usage();
            debug!("Removed cache entry for {} ({:?})", name, record_type);
        }

        Ok(removed)
    }

    /// Clear all cached entries
    pub async fn clear(&self) -> TrustChainResult<()> {
        let count = self.cache.len();
        self.cache.clear();

        {
            let mut stats = self.stats.write().await;
            stats.entries = 0;
            stats.memory_usage_bytes = 0;
        }

        info!("Cleared DNS cache ({} entries)", count);
        Ok(())
    }

    /// Clean up expired entries
    pub async fn cleanup(&self) -> TrustChainResult<()> {
        debug!("Starting DNS cache cleanup");

        let mut expired_keys = Vec::new();
        
        // Collect expired keys
        for item in self.cache.iter() {
            if item.value().is_expired() {
                expired_keys.push(item.key().clone());
            }
        }

        // Remove expired entries
        for key in &expired_keys {
            self.cache.remove(key);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.entries = self.cache.len() as u64;
            stats.memory_usage_bytes = self.estimate_memory_usage();
            stats.last_cleanup = SystemTime::now();
        }

        debug!("DNS cache cleanup completed: removed {} expired entries", expired_keys.len());
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let mut stats = self.stats.read().await.clone();
        stats.entries = self.cache.len() as u64;
        stats.memory_usage_bytes = self.estimate_memory_usage();
        stats
    }

    /// Flush cache to persistent storage (if implemented)
    pub async fn flush(&self) -> TrustChainResult<()> {
        // For now, this is a no-op as we use in-memory cache
        // In production, this could write to disk for persistence
        debug!("DNS cache flush completed (in-memory cache)");
        Ok(())
    }

    // Internal helper methods

    async fn evict_lru_entry(&self) {
        // Find entry with lowest access count and oldest last access time
        let mut lru_key: Option<CacheKey> = None;
        let mut min_priority = u64::MAX;

        for item in self.cache.iter() {
            // Priority = access_count * 1000 + seconds_since_last_access
            let seconds_since_access = item.value().last_accessed
                .elapsed()
                .unwrap_or(Duration::ZERO)
                .as_secs();
            
            let priority = item.value().access_count * 1000 + seconds_since_access;
            
            if priority < min_priority {
                min_priority = priority;
                lru_key = Some(item.key().clone());
            }
        }

        if let Some(key) = lru_key {
            self.cache.remove(&key);
            debug!("Evicted LRU cache entry: {:?}", key);
        }
    }

    fn estimate_memory_usage(&self) -> u64 {
        // Rough estimate: 500 bytes per cache entry
        self.cache.len() as u64 * 500
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::{DnsRecord, DnsRecordData};
    use trust_dns_proto::op::ResponseCode;
    use std::net::Ipv6Addr;

    fn create_test_response(id: u16, name: &str) -> DnsResponse {
        DnsResponse {
            id,
            response_code: ResponseCode::NoError,
            answers: vec![DnsRecord {
                name: name.to_string(),
                record_type: RecordType::AAAA,
                class: trust_dns_proto::rr::DNSClass::IN,
                ttl: 300,
                data: DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
            }],
            authorities: vec![],
            additionals: vec![],
            timestamp: SystemTime::now(),
            ttl: 300,
        }
    }

    #[tokio::test]
    async fn test_cache_creation() {
        let cache = DnsCache::new(Duration::from_secs(300)).await.unwrap();
        let stats = cache.get_stats().await;
        assert_eq!(stats.entries, 0);
    }

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let cache = DnsCache::new(Duration::from_secs(300)).await.unwrap();
        
        let response = create_test_response(1234, "test.example.com");
        cache.set("test.example.com", RecordType::AAAA, &response, 300).await.unwrap();

        let cached = cache.get("test.example.com", RecordType::AAAA).await.unwrap();
        assert!(cached.is_some());
        
        let cached_response = cached.unwrap();
        assert_eq!(cached_response.id, response.id);
        assert_eq!(cached_response.answers.len(), response.answers.len());
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = DnsCache::new(Duration::from_secs(300)).await.unwrap();
        
        let result = cache.get("nonexistent.example.com", RecordType::AAAA).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = DnsCache::new(Duration::from_secs(300)).await.unwrap();
        
        let response = create_test_response(1234, "test.example.com");
        cache.set("test.example.com", RecordType::AAAA, &response, 1).await.unwrap(); // 1 second TTL

        // Should be available immediately
        let cached = cache.get("test.example.com", RecordType::AAAA).await.unwrap();
        assert!(cached.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be expired and removed
        let cached = cache.get("test.example.com", RecordType::AAAA).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_removal() {
        let cache = DnsCache::new(Duration::from_secs(300)).await.unwrap();
        
        let response = create_test_response(1234, "test.example.com");
        cache.set("test.example.com", RecordType::AAAA, &response, 300).await.unwrap();

        // Verify it's cached
        let cached = cache.get("test.example.com", RecordType::AAAA).await.unwrap();
        assert!(cached.is_some());

        // Remove it
        let removed = cache.remove("test.example.com", RecordType::AAAA).await.unwrap();
        assert!(removed);

        // Verify it's gone
        let cached = cache.get("test.example.com", RecordType::AAAA).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = DnsCache::new(Duration::from_secs(300)).await.unwrap();
        
        // Add multiple entries
        for i in 0..5 {
            let response = create_test_response(i, &format!("test{}.example.com", i));
            cache.set(&format!("test{}.example.com", i), RecordType::AAAA, &response, 300).await.unwrap();
        }

        let stats = cache.get_stats().await;
        assert_eq!(stats.entries, 5);

        // Clear cache
        cache.clear().await.unwrap();

        let stats = cache.get_stats().await;
        assert_eq!(stats.entries, 0);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = DnsCache::new(Duration::from_secs(300)).await.unwrap();
        
        let response = create_test_response(1234, "test.example.com");
        cache.set("test.example.com", RecordType::AAAA, &response, 300).await.unwrap();

        // Test hit
        cache.get("test.example.com", RecordType::AAAA).await.unwrap();
        
        // Test miss
        cache.get("nonexistent.example.com", RecordType::AAAA).await.unwrap();

        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
        assert_eq!(stats.entries, 1);
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let cache = DnsCache::new(Duration::from_secs(300)).await.unwrap();
        
        // Add entries with different TTLs
        let response1 = create_test_response(1, "test1.example.com");
        let response2 = create_test_response(2, "test2.example.com");
        
        cache.set("test1.example.com", RecordType::AAAA, &response1, 1).await.unwrap(); // Short TTL
        cache.set("test2.example.com", RecordType::AAAA, &response2, 300).await.unwrap(); // Long TTL

        // Wait for first entry to expire
        tokio::time::sleep(Duration::from_secs(2)).await;

        let stats_before = cache.get_stats().await;
        assert_eq!(stats_before.entries, 2);

        // Run cleanup
        cache.cleanup().await.unwrap();

        let stats_after = cache.get_stats().await;
        assert_eq!(stats_after.entries, 1);
    }

    #[tokio::test]
    async fn test_case_insensitive_keys() {
        let cache = DnsCache::new(Duration::from_secs(300)).await.unwrap();
        
        let response = create_test_response(1234, "Test.Example.COM");
        cache.set("Test.Example.COM", RecordType::AAAA, &response, 300).await.unwrap();

        // Should find with lowercase
        let cached = cache.get("test.example.com", RecordType::AAAA).await.unwrap();
        assert!(cached.is_some());

        // Should find with mixed case
        let cached = cache.get("TeSt.ExAmPlE.cOm", RecordType::AAAA).await.unwrap();
        assert!(cached.is_some());
    }
}