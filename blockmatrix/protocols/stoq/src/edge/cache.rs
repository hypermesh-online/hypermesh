//! Edge cache management

use anyhow::Result;
use bytes::Bytes;
use std::time::Duration;
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CachePolicy {
    Lru,
    Lfu,
    Ttl,
    Adaptive,
}

pub struct CacheManager {
    policy: CachePolicy,
    max_object_size: usize,
    cache: RwLock<HashMap<String, CacheEntry>>,
}

struct CacheEntry {
    data: Bytes,
    expires_at: std::time::Instant,
    access_count: u64,
    last_access: std::time::Instant,
}

impl CacheManager {
    pub fn new(policy: CachePolicy, max_object_size: usize) -> Result<Self> {
        Ok(Self {
            policy,
            max_object_size,
            cache: RwLock::new(HashMap::new()),
        })
    }
    
    pub async fn put(&self, key: String, data: Bytes, ttl: Duration) -> Result<()> {
        if data.len() > self.max_object_size {
            return Err(anyhow::anyhow!("Object too large for cache"));
        }
        
        let entry = CacheEntry {
            data,
            expires_at: std::time::Instant::now() + ttl,
            access_count: 0,
            last_access: std::time::Instant::now(),
        };
        
        self.cache.write().insert(key, entry);
        Ok(())
    }
    
    pub async fn get(&self, key: &str) -> Option<Bytes> {
        let mut cache = self.cache.write();
        if let Some(entry) = cache.get_mut(key) {
            if entry.expires_at > std::time::Instant::now() {
                entry.access_count += 1;
                entry.last_access = std::time::Instant::now();
                Some(entry.data.clone())
            } else {
                cache.remove(key);
                None
            }
        } else {
            None
        }
    }
    
    pub async fn remove(&self, key: &str) {
        self.cache.write().remove(key);
    }
    
    pub fn size(&self) -> u64 {
        self.cache.read().values()
            .map(|entry| entry.data.len() as u64)
            .sum()
    }
}