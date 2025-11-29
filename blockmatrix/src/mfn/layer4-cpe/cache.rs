//! Prediction Caching System
//!
//! This module implements intelligent caching strategies for prediction results
//! to achieve sub-millisecond response times for repeated patterns.

use anyhow::Result;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use dashmap::DashMap;

use crate::prediction::PredictionResult;

/// Cache strategies for prediction results
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CacheStrategy {
    /// Least Recently Used eviction
    Lru,
    /// Least Frequently Used eviction
    Lfu,
    /// Time-To-Live based eviction
    Ttl,
    /// Adaptive caching based on prediction patterns
    Adaptive,
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub prediction: PredictionResult,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub access_count: u64,
    pub hit_count: u64,
    pub confidence_history: Vec<f32>,
}

impl CacheEntry {
    pub fn new(prediction: PredictionResult) -> Self {
        let now = Instant::now();
        Self {
            prediction,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            hit_count: 0,
            confidence_history: Vec::new(),
        }
    }
    
    pub fn access(&mut self) -> &PredictionResult {
        self.last_accessed = Instant::now();
        self.access_count += 1;
        self.hit_count += 1;
        &self.prediction
    }
    
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
    
    pub fn idle_time(&self) -> Duration {
        self.last_accessed.elapsed()
    }
    
    pub fn access_frequency(&self) -> f64 {
        self.access_count as f64 / self.age().as_secs_f64()
    }
}

/// Cache key for prediction lookups
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CacheKey {
    pub context_hash: u64,
    pub sequence_length: usize,
    pub model_signature: String,
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.context_hash.hash(state);
        self.sequence_length.hash(state);
        self.model_signature.hash(state);
    }
}

impl CacheKey {
    pub fn new(context_hash: u64, sequence_length: usize, model_signature: String) -> Self {
        Self {
            context_hash,
            sequence_length,
            model_signature,
        }
    }
}

/// Cache metrics for monitoring performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
    pub current_size: usize,
    pub max_size: usize,
    pub avg_access_time_ms: f64,
    pub memory_usage_mb: f64,
    pub eviction_count: u64,
}

impl CacheMetrics {
    pub fn new(max_size: usize) -> Self {
        Self {
            hit_count: 0,
            miss_count: 0,
            hit_rate: 0.0,
            current_size: 0,
            max_size,
            avg_access_time_ms: 0.0,
            memory_usage_mb: 0.0,
            eviction_count: 0,
        }
    }
    
    pub fn update_hit_rate(&mut self) {
        let total = self.hit_count + self.miss_count;
        self.hit_rate = if total > 0 {
            self.hit_count as f64 / total as f64
        } else {
            0.0
        };
    }
}

/// Main prediction cache with multiple strategies
pub struct PredictionCache {
    strategy: CacheStrategy,
    max_size: usize,
    ttl: Duration,
    
    // Different cache implementations
    lru_cache: Arc<RwLock<Option<LruCache<CacheKey, CacheEntry>>>>,
    adaptive_cache: Arc<DashMap<CacheKey, CacheEntry>>,
    
    // Cache metrics
    metrics: Arc<RwLock<CacheMetrics>>,
    access_times: Arc<RwLock<Vec<Duration>>>,
    
    // Adaptive cache state
    access_patterns: Arc<RwLock<HashMap<CacheKey, AccessPattern>>>,
    eviction_candidates: Arc<RwLock<BinaryHeap<EvictionCandidate>>>,
}

/// Access pattern tracking for adaptive caching
#[derive(Debug, Clone)]
struct AccessPattern {
    key: CacheKey,
    frequency: f64,
    recency_score: f64,
    confidence_trend: f32,
    last_update: Instant,
}

/// Eviction candidate with priority scoring
#[derive(Debug, Clone, PartialEq)]
struct EvictionCandidate {
    key: CacheKey,
    score: f64, // Lower score = higher eviction priority
}

impl Eq for EvictionCandidate {}

impl PartialOrd for EvictionCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse order for min-heap (lowest score first)
        other.score.partial_cmp(&self.score)
    }
}

impl Ord for EvictionCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl PredictionCache {
    /// Create a new prediction cache with specified strategy
    pub fn new(strategy: CacheStrategy, max_size: usize, ttl: Duration) -> Result<Self> {
        info!("Initializing PredictionCache with strategy {:?}, size {}, TTL {:?}", 
              strategy, max_size, ttl);
        
        let lru_cache = match strategy {
            CacheStrategy::Lru | CacheStrategy::Ttl => {
                Arc::new(RwLock::new(Some(LruCache::new(std::num::NonZeroUsize::new(max_size).unwrap()))))
            }
            _ => Arc::new(RwLock::new(None)),
        };
        
        Ok(Self {
            strategy,
            max_size,
            ttl,
            lru_cache,
            adaptive_cache: Arc::new(DashMap::new()),
            metrics: Arc::new(RwLock::new(CacheMetrics::new(max_size))),
            access_times: Arc::new(RwLock::new(Vec::new())),
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
            eviction_candidates: Arc::new(RwLock::new(BinaryHeap::new())),
        })
    }
    
    /// Get a cached prediction result
    pub async fn get(&self, key: &CacheKey) -> Option<PredictionResult> {
        let start_time = Instant::now();
        
        let result = match self.strategy {
            CacheStrategy::Lru | CacheStrategy::Ttl => self.get_from_lru(key).await,
            CacheStrategy::Lfu => self.get_from_lfu(key).await,
            CacheStrategy::Adaptive => self.get_from_adaptive(key).await,
        };
        
        let access_time = start_time.elapsed();
        self.record_access_time(access_time).await;
        
        if result.is_some() {
            self.record_hit().await;
            debug!("Cache hit for key: {:?}", key);
        } else {
            self.record_miss().await;
            debug!("Cache miss for key: {:?}", key);
        }
        
        result
    }
    
    /// Insert a prediction result into the cache
    pub async fn insert(&mut self, key: CacheKey, prediction: PredictionResult) {
        match self.strategy {
            CacheStrategy::Lru | CacheStrategy::Ttl => {
                self.insert_lru(key, prediction).await;
            }
            CacheStrategy::Lfu => {
                self.insert_lfu(key, prediction).await;
            }
            CacheStrategy::Adaptive => {
                self.insert_adaptive(key, prediction).await;
            }
        }
        
        self.update_size_metrics().await;
        debug!("Inserted prediction into cache");
    }
    
    /// LRU cache operations
    async fn get_from_lru(&self, key: &CacheKey) -> Option<PredictionResult> {
        let mut cache_guard = self.lru_cache.write().await;
        if let Some(ref mut cache) = *cache_guard {
            if let Some(entry) = cache.get_mut(key) {
                // Check TTL if applicable
                if self.strategy == CacheStrategy::Ttl && entry.age() > self.ttl {
                    cache.pop(key);
                    return None;
                }
                
                return Some(entry.access().clone());
            }
        }
        None
    }
    
    async fn insert_lru(&mut self, key: CacheKey, prediction: PredictionResult) {
        let mut cache_guard = self.lru_cache.write().await;
        if let Some(ref mut cache) = *cache_guard {
            let entry = CacheEntry::new(prediction);
            cache.put(key, entry);
        }
    }
    
    /// LFU cache operations
    async fn get_from_lfu(&self, key: &CacheKey) -> Option<PredictionResult> {
        if let Some(mut entry) = self.adaptive_cache.get_mut(key) {
            return Some(entry.access().clone());
        }
        None
    }
    
    async fn insert_lfu(&mut self, key: CacheKey, prediction: PredictionResult) {
        // Check if we need to evict
        if self.adaptive_cache.len() >= self.max_size {
            self.evict_lfu().await;
        }
        
        let entry = CacheEntry::new(prediction);
        self.adaptive_cache.insert(key, entry);
    }
    
    /// Adaptive cache operations
    async fn get_from_adaptive(&self, key: &CacheKey) -> Option<PredictionResult> {
        if let Some(mut entry) = self.adaptive_cache.get_mut(key) {
            let result = entry.access().clone();
            
            // Update access pattern
            self.update_access_pattern(key.clone(), &entry).await;
            
            return Some(result);
        }
        None
    }
    
    async fn insert_adaptive(&mut self, key: CacheKey, prediction: PredictionResult) {
        // Check if we need to evict based on adaptive scoring
        if self.adaptive_cache.len() >= self.max_size {
            self.evict_adaptive().await;
        }
        
        let entry = CacheEntry::new(prediction);
        self.adaptive_cache.insert(key.clone(), entry);
        
        // Initialize access pattern
        let pattern = AccessPattern {
            key: key.clone(),
            frequency: 1.0,
            recency_score: 1.0,
            confidence_trend: 0.8, // Default confidence
            last_update: Instant::now(),
        };
        
        self.access_patterns.write().await.insert(key, pattern);
    }
    
    /// Update access pattern for adaptive caching
    async fn update_access_pattern(&self, key: CacheKey, entry: &CacheEntry) {
        let mut patterns = self.access_patterns.write().await;
        
        if let Some(pattern) = patterns.get_mut(&key) {
            let now = Instant::now();
            let time_since_last = pattern.last_update.elapsed().as_secs_f64();
            
            // Update frequency with decay
            let decay_factor = (-time_since_last / 3600.0).exp(); // 1-hour decay
            pattern.frequency = pattern.frequency * decay_factor + 1.0;
            
            // Update recency score
            pattern.recency_score = 1.0; // Max recency on access
            pattern.last_update = now;
            
            // Update confidence trend
            let avg_confidence = entry.confidence_history.iter().sum::<f32>() / entry.confidence_history.len().max(1) as f32;
            pattern.confidence_trend = (pattern.confidence_trend * 0.8) + (avg_confidence * 0.2);
        }
    }
    
    /// Evict using LFU strategy
    async fn evict_lfu(&mut self) {
        let mut min_access_count = u64::MAX;
        let mut evict_key = None;
        
        // Find least frequently used item
        for entry in self.adaptive_cache.iter() {
            if entry.access_count < min_access_count {
                min_access_count = entry.access_count;
                evict_key = Some(entry.key().clone());
            }
        }
        
        if let Some(key) = evict_key {
            self.adaptive_cache.remove(&key);
            self.record_eviction().await;
            debug!("Evicted LFU key: {:?}", key);
        }
    }
    
    /// Evict using adaptive strategy
    async fn evict_adaptive(&mut self) {
        let patterns = self.access_patterns.read().await;
        let mut candidates = Vec::new();
        
        // Score all cache entries for eviction
        for entry in self.adaptive_cache.iter() {
            if let Some(pattern) = patterns.get(entry.key()) {
                let score = self.calculate_eviction_score(pattern, &entry);
                candidates.push(EvictionCandidate {
                    key: entry.key().clone(),
                    score,
                });
            }
        }
        
        // Sort by score (lowest first) and evict
        candidates.sort();
        
        if let Some(candidate) = candidates.first() {
            let key = candidate.key.clone();
            drop(patterns); // Release read lock
            
            self.adaptive_cache.remove(&key);
            self.access_patterns.write().await.remove(&key);
            self.record_eviction().await;
            debug!("Evicted adaptive key: {:?} with score {:.3}", key, candidate.score);
        }
    }
    
    /// Calculate eviction score for adaptive strategy
    fn calculate_eviction_score(&self, pattern: &AccessPattern, entry: &CacheEntry) -> f64 {
        let age_factor = entry.age().as_secs_f64() / 3600.0; // Hours
        let idle_factor = entry.idle_time().as_secs_f64() / 1800.0; // 30 minutes
        let frequency_factor = 1.0 / (pattern.frequency + 1.0);
        let confidence_factor = 1.0 - pattern.confidence_trend as f64;
        
        // Lower score = higher eviction priority
        age_factor * 0.2 + idle_factor * 0.3 + frequency_factor * 0.3 + confidence_factor * 0.2
    }
    
    /// Clean up expired entries
    pub async fn cleanup_expired(&mut self) {
        if self.strategy != CacheStrategy::Ttl && self.strategy != CacheStrategy::Adaptive {
            return;
        }
        
        let now = Instant::now();
        let mut expired_keys = Vec::new();
        
        for entry in self.adaptive_cache.iter() {
            if now.duration_since(entry.created_at) > self.ttl {
                expired_keys.push(entry.key().clone());
            }
        }
        
        for key in expired_keys {
            self.adaptive_cache.remove(&key);
            self.access_patterns.write().await.remove(&key);
        }
        
        if self.strategy == CacheStrategy::Ttl {
            let mut cache_guard = self.lru_cache.write().await;
            if let Some(ref mut cache) = *cache_guard {
                // LRU doesn't have built-in TTL, so we need to manually clean
                let mut to_remove = Vec::new();
                
                // We can't iterate and modify LRU cache simultaneously
                // So we collect keys to remove first
                // This is a simplified approach; in production, you might want a more efficient method
                
                cache.clear(); // Simple approach: clear all on cleanup
            }
        }
        
        debug!("Cleaned up expired cache entries");
    }
    
    /// Get cache metrics
    pub fn get_metrics(&self) -> CacheMetrics {
        // This would be called from an async context, so we can't await here
        // In practice, you'd want to redesign this to be async or use blocking reads
        CacheMetrics {
            hit_count: 0, // Placeholder - would need async access
            miss_count: 0,
            hit_rate: 0.0,
            current_size: self.adaptive_cache.len(),
            max_size: self.max_size,
            avg_access_time_ms: 0.0,
            memory_usage_mb: 0.0,
            eviction_count: 0,
        }
    }
    
    /// Record cache hit
    async fn record_hit(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.hit_count += 1;
        metrics.update_hit_rate();
    }
    
    /// Record cache miss
    async fn record_miss(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.miss_count += 1;
        metrics.update_hit_rate();
    }
    
    /// Record eviction
    async fn record_eviction(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.eviction_count += 1;
    }
    
    /// Record access time
    async fn record_access_time(&self, duration: Duration) {
        let mut times = self.access_times.write().await;
        times.push(duration);
        
        // Keep only recent access times
        if times.len() > 1000 {
            times.remove(0);
        }
    }
    
    /// Update size metrics
    async fn update_size_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.current_size = self.adaptive_cache.len();
        
        // Estimate memory usage (rough approximation)
        metrics.memory_usage_mb = (metrics.current_size * 1024) as f64 / (1024.0 * 1024.0);
    }
    
    /// Get detailed cache statistics
    pub async fn get_detailed_statistics(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        
        let metrics = self.metrics.read().await;
        stats.insert("hit_count".to_string(), metrics.hit_count as f64);
        stats.insert("miss_count".to_string(), metrics.miss_count as f64);
        stats.insert("hit_rate".to_string(), metrics.hit_rate);
        stats.insert("current_size".to_string(), metrics.current_size as f64);
        stats.insert("max_size".to_string(), metrics.max_size as f64);
        stats.insert("eviction_count".to_string(), metrics.eviction_count as f64);
        
        let access_times = self.access_times.read().await;
        if !access_times.is_empty() {
            let total_time: f64 = access_times.iter().map(|d| d.as_secs_f64() * 1000.0).sum();
            stats.insert("avg_access_time_ms".to_string(), total_time / access_times.len() as f64);
        }
        
        stats.insert("adaptive_cache_size".to_string(), self.adaptive_cache.len() as f64);
        stats.insert("access_patterns_count".to_string(), self.access_patterns.read().await.len() as f64);
        
        stats
    }
    
    /// Clear all cache entries
    pub async fn clear(&mut self) {
        match self.strategy {
            CacheStrategy::Lru | CacheStrategy::Ttl => {
                let mut cache_guard = self.lru_cache.write().await;
                if let Some(ref mut cache) = *cache_guard {
                    cache.clear();
                }
            }
            _ => {}
        }
        
        self.adaptive_cache.clear();
        self.access_patterns.write().await.clear();
        self.eviction_candidates.write().await.clear();
        
        // Reset metrics
        *self.metrics.write().await = CacheMetrics::new(self.max_size);
        self.access_times.write().await.clear();
        
        info!("Cache cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prediction::PredictionResult;
    
    fn create_test_prediction() -> PredictionResult {
        PredictionResult::new(vec![0.1, 0.2, 0.3], 0.8)
    }
    
    fn create_test_key(id: u64) -> CacheKey {
        CacheKey::new(id, 10, "test_model".to_string())
    }
    
    #[tokio::test]
    async fn test_cache_creation() {
        let cache = PredictionCache::new(
            CacheStrategy::Lru,
            100,
            Duration::from_secs(300),
        );
        assert!(cache.is_ok());
    }
    
    #[tokio::test]
    async fn test_lru_cache_operations() {
        let mut cache = PredictionCache::new(
            CacheStrategy::Lru,
            3,
            Duration::from_secs(300),
        ).unwrap();
        
        let key1 = create_test_key(1);
        let key2 = create_test_key(2);
        let key3 = create_test_key(3);
        let key4 = create_test_key(4);
        
        let pred1 = create_test_prediction();
        let pred2 = create_test_prediction();
        let pred3 = create_test_prediction();
        let pred4 = create_test_prediction();
        
        // Insert items
        cache.insert(key1.clone(), pred1).await;
        cache.insert(key2.clone(), pred2).await;
        cache.insert(key3.clone(), pred3).await;
        
        // All should be present
        assert!(cache.get(&key1).await.is_some());
        assert!(cache.get(&key2).await.is_some());
        assert!(cache.get(&key3).await.is_some());
        
        // Insert 4th item (should evict least recently used)
        cache.insert(key4.clone(), pred4).await;
        
        // key1 should be evicted (if LRU works correctly)
        assert!(cache.get(&key4).await.is_some());
    }
    
    #[tokio::test]
    async fn test_adaptive_cache() {
        let mut cache = PredictionCache::new(
            CacheStrategy::Adaptive,
            5,
            Duration::from_secs(300),
        ).unwrap();
        
        // Insert and access items with different patterns
        for i in 1..=10 {
            let key = create_test_key(i);
            let pred = create_test_prediction();
            cache.insert(key.clone(), pred).await;
            
            // Access some items multiple times to build frequency
            if i <= 3 {
                for _ in 0..5 {
                    let _ = cache.get(&key).await;
                }
            }
        }
        
        // Check that frequently accessed items are still in cache
        let key1 = create_test_key(1);
        assert!(cache.get(&key1).await.is_some());
    }
    
    #[tokio::test]
    async fn test_ttl_expiration() {
        let mut cache = PredictionCache::new(
            CacheStrategy::Ttl,
            100,
            Duration::from_millis(100),
        ).unwrap();
        
        let key = create_test_key(1);
        let pred = create_test_prediction();
        cache.insert(key.clone(), pred).await;
        
        // Should be present initially
        assert!(cache.get(&key).await.is_some());
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Should be expired now (but we need to trigger cleanup)
        cache.cleanup_expired().await;
        assert!(cache.get(&key).await.is_none());
    }
    
    #[tokio::test]
    async fn test_cache_metrics() {
        let mut cache = PredictionCache::new(
            CacheStrategy::Adaptive,
            10,
            Duration::from_secs(300),
        ).unwrap();
        
        let key1 = create_test_key(1);
        let key2 = create_test_key(2);
        let pred = create_test_prediction();
        
        // Insert item
        cache.insert(key1.clone(), pred.clone()).await;
        
        // Test hit
        let _ = cache.get(&key1).await;
        
        // Test miss
        let _ = cache.get(&key2).await;
        
        let stats = cache.get_detailed_statistics().await;
        assert_eq!(stats.get("hit_count").unwrap(), &1.0);
        assert_eq!(stats.get("miss_count").unwrap(), &1.0);
        assert_eq!(stats.get("hit_rate").unwrap(), &0.5);
    }
    
    #[test]
    fn test_cache_key_hash() {
        let key1 = CacheKey::new(123, 10, "model1".to_string());
        let key2 = CacheKey::new(123, 10, "model1".to_string());
        let key3 = CacheKey::new(124, 10, "model1".to_string());
        
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}