//! Pattern Cache for Fast Similarity Lookups
//!
//! Implements an LRU cache with adaptive sizing for storing similarity results
//! and patterns to achieve <1ms response times.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant, SystemTime};
use tracing::{debug, trace, warn};

/// Similarity result with caching metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResult {
    pub similarity_score: f64,
    pub pattern_id: usize,
    pub confidence: f64,
    pub processing_time: Duration,
    pub cache_hit: bool,
}

/// Cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    pub key: String,
    pub result: SimilarityResult,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub access_count: u64,
    pub ttl: Duration,
    pub size_bytes: usize,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hit_rate: f64,
    pub current_size: usize,
    pub max_size: usize,
    pub eviction_count: u64,
    pub average_lookup_time_ns: f64,
    pub memory_usage_bytes: usize,
}

/// Cache eviction policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used  
    LFU,
    /// Time-based expiration
    TTL,
    /// Hybrid LRU + frequency + TTL
    Adaptive,
}

/// Pattern cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub max_memory_mb: usize,
    pub default_ttl_seconds: u64,
    pub eviction_policy: EvictionPolicy,
    pub cleanup_interval_seconds: u64,
    pub adaptive_sizing: bool,
    pub compression_threshold_kb: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            max_memory_mb: 100,
            default_ttl_seconds: 300, // 5 minutes
            eviction_policy: EvictionPolicy::Adaptive,
            cleanup_interval_seconds: 60, // 1 minute
            adaptive_sizing: true,
            compression_threshold_kb: 1, // Compress entries > 1KB
        }
    }
}

/// High-performance pattern cache
pub struct PatternCache {
    config: CacheConfig,
    entries: HashMap<String, CacheEntry>,
    /// LRU ordering for eviction
    lru_order: VecDeque<String>,
    /// Frequency tracking for LFU
    frequency_map: HashMap<String, u64>,
    /// Statistics
    total_requests: u64,
    cache_hits: u64,
    cache_misses: u64,
    eviction_count: u64,
    total_lookup_time_ns: u64,
    /// Memory tracking
    current_memory_bytes: usize,
    last_cleanup: Instant,
    /// Performance monitoring
    hit_rate_history: VecDeque<f64>,
    response_time_history: VecDeque<Duration>,
}

impl PatternCache {
    pub fn new(max_size: usize) -> Result<Self> {
        let config = CacheConfig {
            max_entries: max_size,
            ..Default::default()
        };
        
        Self::with_config(config)
    }
    
    pub fn with_config(config: CacheConfig) -> Result<Self> {
        Ok(Self {
            config,
            entries: HashMap::with_capacity(1000),
            lru_order: VecDeque::with_capacity(1000),
            frequency_map: HashMap::with_capacity(1000),
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            eviction_count: 0,
            total_lookup_time_ns: 0,
            current_memory_bytes: 0,
            last_cleanup: Instant::now(),
            hit_rate_history: VecDeque::with_capacity(1000),
            response_time_history: VecDeque::with_capacity(1000),
        })
    }
    
    /// Get similarity result from cache
    pub fn get(&mut self, key: &str) -> Option<SimilarityResult> {
        let start = Instant::now();
        self.total_requests += 1;
        
        // Check if cleanup is needed
        if self.last_cleanup.elapsed().as_secs() >= self.config.cleanup_interval_seconds {
            self.cleanup_expired_entries();
        }
        
        let result = if let Some(entry) = self.entries.get_mut(key) {
            // Check TTL
            if entry.created_at.elapsed() <= entry.ttl {
                // Update access metadata
                entry.last_accessed = Instant::now();
                entry.access_count += 1;
                
                // Update LRU order
                self.update_lru_order(key);
                
                // Update frequency
                *self.frequency_map.entry(key.to_string()).or_insert(0) += 1;
                
                self.cache_hits += 1;
                
                let mut result = entry.result.clone();
                result.cache_hit = true;
                
                trace!("Cache hit for key: {} (accessed {} times)", 
                      key, entry.access_count);
                
                Some(result)
            } else {
                // Entry expired
                self.remove_entry(key);
                self.cache_misses += 1;
                None
            }
        } else {
            self.cache_misses += 1;
            None
        };
        
        // Record lookup time
        let lookup_time = start.elapsed();
        self.total_lookup_time_ns += lookup_time.as_nanos() as u64;
        
        // Update performance history
        self.update_performance_history(lookup_time);
        
        result
    }
    
    /// Insert similarity result into cache
    pub fn insert(&mut self, key: String, result: SimilarityResult) -> Result<()> {
        // Calculate entry size
        let entry_size = self.estimate_entry_size(&key, &result);
        
        // Check if we need to make space
        self.ensure_capacity(entry_size)?;
        
        // Create cache entry
        let cache_entry = CacheEntry {
            key: key.clone(),
            result: result.clone(),
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
            ttl: Duration::from_secs(self.config.default_ttl_seconds),
            size_bytes: entry_size,
        };
        
        // Insert entry
        if let Some(old_entry) = self.entries.insert(key.clone(), cache_entry) {
            // Update memory usage
            self.current_memory_bytes -= old_entry.size_bytes;
        } else {
            // New entry - add to LRU order
            self.lru_order.push_back(key.clone());
        }
        
        self.current_memory_bytes += entry_size;
        
        trace!("Cached similarity result for key: {} (size: {} bytes)", 
              key, entry_size);
        
        Ok(())
    }
    
    /// Remove entry from cache
    pub fn remove(&mut self, key: &str) -> Option<SimilarityResult> {
        self.remove_entry(key).map(|entry| entry.result)
    }
    
    /// Clear entire cache
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru_order.clear();
        self.frequency_map.clear();
        self.current_memory_bytes = 0;
        self.eviction_count = 0;
        
        debug!("Cache cleared");
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let hit_rate = if self.total_requests > 0 {
            self.cache_hits as f64 / self.total_requests as f64
        } else {
            0.0
        };
        
        let avg_lookup_time = if self.total_requests > 0 {
            self.total_lookup_time_ns as f64 / self.total_requests as f64
        } else {
            0.0
        };
        
        CacheStats {
            total_requests: self.total_requests,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            hit_rate,
            current_size: self.entries.len(),
            max_size: self.config.max_entries,
            eviction_count: self.eviction_count,
            average_lookup_time_ns: avg_lookup_time,
            memory_usage_bytes: self.current_memory_bytes,
        }
    }
    
    /// Get cache performance metrics
    pub fn get_performance_metrics(&self) -> CachePerformanceMetrics {
        let recent_hit_rate = if self.hit_rate_history.len() >= 10 {
            let recent: Vec<f64> = self.hit_rate_history.iter().rev().take(10).cloned().collect();
            recent.iter().sum::<f64>() / recent.len() as f64
        } else {
            self.get_stats().hit_rate
        };
        
        let recent_response_time = if self.response_time_history.len() >= 10 {
            let recent: Vec<Duration> = self.response_time_history.iter().rev().take(10).cloned().collect();
            let avg_nanos = recent.iter().map(|d| d.as_nanos()).sum::<u128>() / recent.len() as u128;
            Duration::from_nanos(avg_nanos as u64)
        } else {
            Duration::from_nanos(self.get_stats().average_lookup_time_ns as u64)
        };
        
        CachePerformanceMetrics {
            recent_hit_rate,
            recent_response_time,
            memory_efficiency: self.calculate_memory_efficiency(),
            fragmentation_ratio: self.calculate_fragmentation(),
            hotspot_concentration: self.calculate_hotspot_concentration(),
        }
    }
    
    /// Adaptive cache sizing based on performance
    pub fn adapt_cache_size(&mut self) -> Result<()> {
        if !self.config.adaptive_sizing {
            return Ok(());
        }
        
        let stats = self.get_stats();
        let performance = self.get_performance_metrics();
        
        // Increase cache size if hit rate is good and memory allows
        if performance.recent_hit_rate > 0.8 && 
           self.current_memory_bytes < (self.config.max_memory_mb * 1024 * 1024) / 2 {
            let new_size = (self.config.max_entries as f64 * 1.2) as usize;
            self.config.max_entries = new_size.min(50000); // Cap at 50k entries
            
            debug!("Increased cache size to {} entries (hit rate: {:.1}%)", 
                  self.config.max_entries, performance.recent_hit_rate * 100.0);
        }
        
        // Decrease cache size if hit rate is poor or memory pressure
        else if performance.recent_hit_rate < 0.5 || 
                self.current_memory_bytes > (self.config.max_memory_mb * 1024 * 1024) * 9 / 10 {
            let new_size = (self.config.max_entries as f64 * 0.8) as usize;
            self.config.max_entries = new_size.max(100); // Minimum 100 entries
            
            // Trigger cleanup to fit new size
            self.ensure_capacity(0)?;
            
            debug!("Decreased cache size to {} entries (hit rate: {:.1}%, memory: {}MB)", 
                  self.config.max_entries, performance.recent_hit_rate * 100.0,
                  self.current_memory_bytes / (1024 * 1024));
        }
        
        Ok(())
    }
    
    /// Prefetch patterns based on prediction
    pub fn prefetch_patterns(&mut self, predicted_keys: &[String], predictions: &[SimilarityResult]) -> Result<usize> {
        let mut prefetched = 0;
        
        for (key, result) in predicted_keys.iter().zip(predictions.iter()) {
            if !self.entries.contains_key(key) && self.entries.len() < self.config.max_entries {
                self.insert(key.clone(), result.clone())?;
                prefetched += 1;
            }
        }
        
        if prefetched > 0 {
            debug!("Prefetched {} patterns", prefetched);
        }
        
        Ok(prefetched)
    }
    
    /// Internal helper methods
    
    fn remove_entry(&mut self, key: &str) -> Option<CacheEntry> {
        if let Some(entry) = self.entries.remove(key) {
            // Update memory usage
            self.current_memory_bytes -= entry.size_bytes;
            
            // Remove from LRU order
            if let Some(pos) = self.lru_order.iter().position(|k| k == key) {
                self.lru_order.remove(pos);
            }
            
            // Remove from frequency map
            self.frequency_map.remove(key);
            
            Some(entry)
        } else {
            None
        }
    }
    
    fn update_lru_order(&mut self, key: &str) {
        // Remove key from current position
        if let Some(pos) = self.lru_order.iter().position(|k| k == key) {
            self.lru_order.remove(pos);
        }
        
        // Add to back (most recently used)
        self.lru_order.push_back(key.to_string());
    }
    
    fn ensure_capacity(&mut self, needed_bytes: usize) -> Result<()> {
        // Check entry count limit
        while self.entries.len() >= self.config.max_entries {
            self.evict_one_entry()?;
        }
        
        // Check memory limit
        let max_memory_bytes = self.config.max_memory_mb * 1024 * 1024;
        while self.current_memory_bytes + needed_bytes > max_memory_bytes {
            self.evict_one_entry()?;
        }
        
        Ok(())
    }
    
    fn evict_one_entry(&mut self) -> Result<()> {
        let key_to_evict = match self.config.eviction_policy {
            EvictionPolicy::LRU => self.select_lru_victim(),
            EvictionPolicy::LFU => self.select_lfu_victim(),
            EvictionPolicy::TTL => self.select_ttl_victim(),
            EvictionPolicy::Adaptive => self.select_adaptive_victim(),
        };
        
        if let Some(key) = key_to_evict {
            self.remove_entry(&key);
            self.eviction_count += 1;
            
            trace!("Evicted cache entry: {}", key);
        } else {
            warn!("No entries available for eviction");
        }
        
        Ok(())
    }
    
    fn select_lru_victim(&self) -> Option<String> {
        self.lru_order.front().cloned()
    }
    
    fn select_lfu_victim(&self) -> Option<String> {
        self.frequency_map.iter()
            .min_by_key(|(_, &count)| count)
            .map(|(key, _)| key.clone())
    }
    
    fn select_ttl_victim(&self) -> Option<String> {
        let now = Instant::now();
        
        // Find expired entries first
        for (key, entry) in &self.entries {
            if now.duration_since(entry.created_at) > entry.ttl {
                return Some(key.clone());
            }
        }
        
        // If no expired entries, select oldest
        self.entries.iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone())
    }
    
    fn select_adaptive_victim(&self) -> Option<String> {
        let now = Instant::now();
        let mut best_key = None;
        let mut best_score = f64::INFINITY;
        
        for (key, entry) in &self.entries {
            // Combine multiple factors for adaptive eviction
            let age_factor = now.duration_since(entry.created_at).as_secs_f64() / 3600.0; // Age in hours
            let frequency_factor = 1.0 / (entry.access_count + 1) as f64; // Lower frequency = higher score
            let recency_factor = now.duration_since(entry.last_accessed).as_secs_f64() / 3600.0; // Recency in hours
            let size_factor = entry.size_bytes as f64 / 1024.0; // Size in KB
            
            // Weighted combination (lower score = better eviction candidate)
            let score = age_factor * 0.3 + frequency_factor * 0.4 + recency_factor * 0.2 + size_factor * 0.1;
            
            if score < best_score {
                best_score = score;
                best_key = Some(key.clone());
            }
        }
        
        best_key
    }
    
    fn cleanup_expired_entries(&mut self) {
        let now = Instant::now();
        let expired_keys: Vec<String> = self.entries.iter()
            .filter(|(_, entry)| now.duration_since(entry.created_at) > entry.ttl)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in expired_keys {
            self.remove_entry(&key);
            self.eviction_count += 1;
        }
        
        self.last_cleanup = now;
        
        if self.eviction_count > 0 {
            debug!("Cleaned up expired entries");
        }
    }
    
    fn estimate_entry_size(&self, key: &str, result: &SimilarityResult) -> usize {
        // Rough estimate of memory usage
        let key_size = key.len() * std::mem::size_of::<char>();
        let result_size = std::mem::size_of::<SimilarityResult>();
        let metadata_size = std::mem::size_of::<CacheEntry>();
        
        key_size + result_size + metadata_size
    }
    
    fn update_performance_history(&mut self, lookup_time: Duration) {
        // Update hit rate history
        let current_hit_rate = if self.total_requests > 0 {
            self.cache_hits as f64 / self.total_requests as f64
        } else {
            0.0
        };
        
        self.hit_rate_history.push_back(current_hit_rate);
        if self.hit_rate_history.len() > 1000 {
            self.hit_rate_history.pop_front();
        }
        
        // Update response time history
        self.response_time_history.push_back(lookup_time);
        if self.response_time_history.len() > 1000 {
            self.response_time_history.pop_front();
        }
    }
    
    fn calculate_memory_efficiency(&self) -> f64 {
        if self.current_memory_bytes == 0 {
            return 1.0;
        }
        
        // Calculate ratio of useful data to total memory usage
        let data_size: usize = self.entries.values()
            .map(|entry| entry.key.len() + std::mem::size_of::<SimilarityResult>())
            .sum();
        
        data_size as f64 / self.current_memory_bytes as f64
    }
    
    fn calculate_fragmentation(&self) -> f64 {
        // Simplified fragmentation calculation
        // In a real implementation, this would analyze memory layout
        if self.entries.is_empty() {
            return 0.0;
        }
        
        let avg_entry_size = self.current_memory_bytes as f64 / self.entries.len() as f64;
        let size_variance: f64 = self.entries.values()
            .map(|entry| {
                let diff = entry.size_bytes as f64 - avg_entry_size;
                diff * diff
            })
            .sum::<f64>() / self.entries.len() as f64;
        
        size_variance.sqrt() / avg_entry_size
    }
    
    fn calculate_hotspot_concentration(&self) -> f64 {
        if self.entries.len() < 10 {
            return 0.0;
        }
        
        // Calculate how concentrated the access patterns are
        let total_accesses: u64 = self.entries.values().map(|e| e.access_count).sum();
        if total_accesses == 0 {
            return 0.0;
        }
        
        // Calculate entropy of access distribution
        let entropy: f64 = self.entries.values()
            .filter(|e| e.access_count > 0)
            .map(|entry| {
                let p = entry.access_count as f64 / total_accesses as f64;
                -p * p.log2()
            })
            .sum();
        
        let max_entropy = (self.entries.len() as f64).log2();
        if max_entropy > 0.0 {
            1.0 - (entropy / max_entropy) // Higher concentration = lower entropy
        } else {
            0.0
        }
    }
}

/// Cache performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePerformanceMetrics {
    pub recent_hit_rate: f64,
    pub recent_response_time: Duration,
    pub memory_efficiency: f64,
    pub fragmentation_ratio: f64,
    pub hotspot_concentration: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_creation() {
        let cache = PatternCache::new(100);
        assert!(cache.is_ok());
        
        let cache = cache.unwrap();
        assert_eq!(cache.config.max_entries, 100);
    }
    
    #[test]
    fn test_cache_insertion_and_retrieval() {
        let mut cache = PatternCache::new(100).unwrap();
        
        let result = SimilarityResult {
            similarity_score: 0.85,
            pattern_id: 42,
            confidence: 0.9,
            processing_time: Duration::from_millis(5),
            cache_hit: false,
        };
        
        let key = "test_pattern_123".to_string();
        
        // Insert
        let insert_result = cache.insert(key.clone(), result.clone());
        assert!(insert_result.is_ok());
        
        // Retrieve
        let retrieved = cache.get(&key);
        assert!(retrieved.is_some());
        
        let retrieved_result = retrieved.unwrap();
        assert_eq!(retrieved_result.similarity_score, result.similarity_score);
        assert_eq!(retrieved_result.pattern_id, result.pattern_id);
        assert!(retrieved_result.cache_hit); // Should be marked as cache hit
    }
    
    #[test]
    fn test_cache_eviction() {
        let mut cache = PatternCache::new(3).unwrap(); // Small cache for testing
        
        let result1 = SimilarityResult {
            similarity_score: 0.8,
            pattern_id: 1,
            confidence: 0.9,
            processing_time: Duration::from_millis(1),
            cache_hit: false,
        };
        
        let result2 = result1.clone();
        let result3 = result1.clone();
        let result4 = result1.clone();
        
        // Fill cache to capacity
        cache.insert("key1".to_string(), result1).unwrap();
        cache.insert("key2".to_string(), result2).unwrap();
        cache.insert("key3".to_string(), result3).unwrap();
        
        assert_eq!(cache.entries.len(), 3);
        
        // Insert one more to trigger eviction
        cache.insert("key4".to_string(), result4).unwrap();
        
        assert_eq!(cache.entries.len(), 3); // Should still be 3
        assert!(cache.eviction_count > 0); // Should have evicted something
    }
    
    #[test]
    fn test_cache_ttl() {
        let mut cache = PatternCache::with_config(CacheConfig {
            max_entries: 100,
            default_ttl_seconds: 1, // 1 second TTL for testing
            ..Default::default()
        }).unwrap();
        
        let result = SimilarityResult {
            similarity_score: 0.75,
            pattern_id: 10,
            confidence: 0.8,
            processing_time: Duration::from_millis(2),
            cache_hit: false,
        };
        
        cache.insert("ttl_test".to_string(), result).unwrap();
        
        // Should be available immediately
        assert!(cache.get("ttl_test").is_some());
        
        // Wait for TTL to expire (in real test, would need to sleep or mock time)
        // For this test, we'll manually update the entry's creation time
        if let Some(entry) = cache.entries.get_mut("ttl_test") {
            entry.created_at = Instant::now() - Duration::from_secs(2);
        }
        
        // Should be expired now
        assert!(cache.get("ttl_test").is_none());
    }
    
    #[test]
    fn test_cache_statistics() {
        let mut cache = PatternCache::new(100).unwrap();
        
        let result = SimilarityResult {
            similarity_score: 0.9,
            pattern_id: 5,
            confidence: 0.95,
            processing_time: Duration::from_millis(3),
            cache_hit: false,
        };
        
        // Insert and access to generate stats
        cache.insert("stats_test".to_string(), result).unwrap();
        cache.get("stats_test"); // Hit
        cache.get("nonexistent"); // Miss
        
        let stats = cache.get_stats();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
        assert_eq!(stats.current_size, 1);
    }
    
    #[test]
    fn test_lru_eviction() {
        let mut cache = PatternCache::with_config(CacheConfig {
            max_entries: 2,
            eviction_policy: EvictionPolicy::LRU,
            ..Default::default()
        }).unwrap();
        
        let result = SimilarityResult {
            similarity_score: 0.8,
            pattern_id: 1,
            confidence: 0.9,
            processing_time: Duration::from_millis(1),
            cache_hit: false,
        };
        
        // Insert two entries
        cache.insert("lru1".to_string(), result.clone()).unwrap();
        cache.insert("lru2".to_string(), result.clone()).unwrap();
        
        // Access first entry to make it more recently used
        cache.get("lru1");
        
        // Insert third entry - should evict lru2
        cache.insert("lru3".to_string(), result).unwrap();
        
        assert!(cache.get("lru1").is_some()); // Should still be there
        assert!(cache.get("lru2").is_none()); // Should be evicted
        assert!(cache.get("lru3").is_some()); // Should be there
    }
    
    #[test]
    fn test_adaptive_sizing() {
        let mut cache = PatternCache::with_config(CacheConfig {
            max_entries: 10,
            adaptive_sizing: true,
            ..Default::default()
        }).unwrap();
        
        let result = SimilarityResult {
            similarity_score: 0.8,
            pattern_id: 1,
            confidence: 0.9,
            processing_time: Duration::from_millis(1),
            cache_hit: false,
        };
        
        // Create high hit rate scenario
        for i in 0..5 {
            cache.insert(format!("key{}", i), result.clone()).unwrap();
        }
        
        // Generate hits
        for _ in 0..20 {
            for i in 0..5 {
                cache.get(&format!("key{}", i));
            }
        }
        
        let original_size = cache.config.max_entries;
        
        // Trigger adaptive sizing
        cache.adapt_cache_size().unwrap();
        
        // With high hit rate, size might increase
        // (depends on memory constraints in test environment)
        assert!(cache.config.max_entries >= original_size);
    }
}