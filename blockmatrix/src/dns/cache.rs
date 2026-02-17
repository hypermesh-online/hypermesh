// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DNS Cache with TTL Management

use super::{DnsRecord, DnsRecordType, DnsError, DnsResult};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::debug;

/// Cache entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    /// DNS records
    pub records: Vec<DnsRecord>,
    /// Cache timestamp
    pub cached_at: SystemTime,
    /// Expiration timestamp
    pub expires_at: SystemTime,
}

impl CacheEntry {
    /// Check if entry is expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }
}

/// DNS cache
pub struct DnsCache {
    /// Cache entries (domain:record_type -> entry)
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Maximum cache size
    max_size: usize,
    /// Cache hits
    hits: Arc<RwLock<u64>>,
    /// Cache misses
    misses: Arc<RwLock<u64>>,
}

impl DnsCache {
    /// Create new DNS cache
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            hits: Arc::new(RwLock::new(0)),
            misses: Arc::new(RwLock::new(0)),
        }
    }

    /// Get cache key
    fn cache_key(domain: &str, record_type: &DnsRecordType) -> String {
        format!("{}:{:?}", domain, record_type)
    }

    /// Get cached records
    pub async fn get(
        &self,
        domain: &str,
        record_type: &DnsRecordType,
    ) -> DnsResult<Option<Vec<DnsRecord>>> {
        let key = Self::cache_key(domain, record_type);
        let entries = self.entries.read().await;

        match entries.get(&key) {
            Some(entry) if !entry.is_expired() => {
                let mut hits = self.hits.write().await;
                *hits += 1;
                debug!("DNS cache hit: {} ({:?})", domain, record_type);
                Ok(Some(entry.records.clone()))
            }
            _ => {
                let mut misses = self.misses.write().await;
                *misses += 1;
                debug!("DNS cache miss: {} ({:?})", domain, record_type);
                Ok(None)
            }
        }
    }

    /// Set cache entry
    pub async fn set(
        &self,
        domain: &str,
        record_type: &DnsRecordType,
        records: Vec<DnsRecord>,
        ttl: u32,
    ) -> DnsResult<()> {
        let key = Self::cache_key(domain, record_type);
        let mut entries = self.entries.write().await;

        // Check size limit
        if entries.len() >= self.max_size && !entries.contains_key(&key) {
            // Evict oldest entry
            if let Some(oldest_key) = entries
                .iter()
                .min_by_key(|(_, v)| v.cached_at)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&oldest_key);
            }
        }

        let now = SystemTime::now();
        let expires_at = now + std::time::Duration::from_secs(ttl as u64);

        entries.insert(
            key,
            CacheEntry {
                records,
                cached_at: now,
                expires_at,
            },
        );

        Ok(())
    }

    /// Remove entry from cache
    pub async fn remove(&self, domain: &str, record_type: &DnsRecordType) -> DnsResult<()> {
        let key = Self::cache_key(domain, record_type);
        let mut entries = self.entries.write().await;
        entries.remove(&key);
        Ok(())
    }

    /// Clear all cache entries
    pub async fn clear(&self) -> DnsResult<()> {
        let mut entries = self.entries.write().await;
        entries.clear();
        Ok(())
    }

    /// Cleanup expired entries
    pub async fn cleanup_expired(&self) -> DnsResult<usize> {
        let mut entries = self.entries.write().await;
        let original_size = entries.len();
        entries.retain(|_, v| !v.is_expired());
        let removed = original_size - entries.len();
        debug!("DNS cache cleanup: removed {} expired entries", removed);
        Ok(removed)
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;
        let hits = *self.hits.read().await;
        let misses = *self.misses.read().await;

        let hit_rate = if hits + misses > 0 {
            hits as f64 / (hits + misses) as f64
        } else {
            0.0
        };

        CacheStats {
            size: entries.len(),
            max_size: self.max_size,
            hits,
            misses,
            hit_rate,
        }
    }
}

/// Cache statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::DnsRecordData;
    use std::net::Ipv6Addr;

    fn create_test_record(domain: &str) -> DnsRecord {
        DnsRecord::new(
            domain.to_string(),
            DnsRecordType::AAAA,
            DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
            300,
            "node-1".to_string(),
        )
    }

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let cache = DnsCache::new(100);
        let record = create_test_record("nike");

        cache
            .set("nike", &DnsRecordType::AAAA, vec![record.clone()], 300)
            .await
            .unwrap();

        let cached = cache
            .get("nike", &DnsRecordType::AAAA)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].domain, "nike");
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = DnsCache::new(100);
        let result = cache.get("nonexistent", &DnsRecordType::AAAA).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = DnsCache::new(100);
        let record = create_test_record("test");

        // Set with 0 TTL (expires immediately)
        cache
            .set("test", &DnsRecordType::AAAA, vec![record], 0)
            .await
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let result = cache.get("test", &DnsRecordType::AAAA).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = DnsCache::new(2);

        for i in 0..3 {
            let record = create_test_record(&format!("domain{}", i));
            cache
                .set(
                    &format!("domain{}", i),
                    &DnsRecordType::AAAA,
                    vec![record],
                    300,
                )
                .await
                .unwrap();
        }

        let stats = cache.stats().await;
        assert_eq!(stats.size, 2); // Should be at max_size
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let cache = DnsCache::new(100);
        let mut record = create_test_record("test");
        record.ttl = 0;
        record.expires_at = SystemTime::now();

        cache
            .set("test", &DnsRecordType::AAAA, vec![record], 0)
            .await
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        let removed = cache.cleanup_expired().await.unwrap();
        assert_eq!(removed, 1);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = DnsCache::new(100);
        let record = create_test_record("nike");

        cache
            .set("nike", &DnsRecordType::AAAA, vec![record], 300)
            .await
            .unwrap();

        // Hit
        let _ = cache.get("nike", &DnsRecordType::AAAA).await;
        // Miss
        let _ = cache.get("nonexistent", &DnsRecordType::AAAA).await;

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
    }
}
