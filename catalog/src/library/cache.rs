//! Multi-tier Package Cache Implementation
//!
//! Provides L1/L2/L3 caching for optimal performance with automatic tier management.

use super::types::LibraryAssetPackage;
use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, VecDeque};
use std::time::{Instant, Duration};
use serde::{Serialize, Deserialize};

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub l1_hits: u64,
    pub l2_hits: u64,
    pub l3_hits: u64,
    pub total_hits: u64,
    pub total_misses: u64,
    pub evictions: u64,
}

/// Cache entry with metadata
#[derive(Clone)]
struct CacheEntry {
    /// The cached package
    package: Arc<LibraryAssetPackage>,
    /// Last access time
    last_accessed: Instant,
    /// Access count
    access_count: u32,
    /// Entry size in bytes (estimated)
    size_bytes: usize,
}

/// Multi-tier package cache for performance optimization
pub struct PackageCache {
    /// L1 cache (hot data, in-memory)
    l1_cache: Arc<RwLock<LRUCache>>,
    /// L2 cache (warm data, in-memory)
    l2_cache: Arc<RwLock<LRUCache>>,
    /// L3 cache path (cold data, disk-based)
    l3_cache_path: Option<String>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
    /// L3 cache (disk-based, optional)
    l3_cache: Option<Arc<RwLock<DiskCache>>>,
}

/// LRU Cache implementation for L1/L2 tiers
struct LRUCache {
    /// Maximum capacity (number of items)
    capacity: usize,
    /// Cache entries
    entries: HashMap<Arc<str>, CacheEntry>,
    /// Access order (most recent last)
    access_order: VecDeque<Arc<str>>,
    /// Current size in bytes
    current_size: usize,
    /// Maximum size in bytes (optional)
    max_size_bytes: Option<usize>,
}

impl LRUCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::with_capacity(capacity),
            access_order: VecDeque::with_capacity(capacity),
            current_size: 0,
            max_size_bytes: None,
        }
    }

    fn get(&mut self, key: &Arc<str>) -> Option<Arc<LibraryAssetPackage>> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_accessed = Instant::now();
            entry.access_count += 1;

            // Move to end of access order
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                self.access_order.remove(pos);
            }
            self.access_order.push_back(Arc::clone(key));

            Some(Arc::clone(&entry.package))
        } else {
            None
        }
    }

    fn insert(&mut self, key: Arc<str>, package: Arc<LibraryAssetPackage>) -> Option<CacheEntry> {
        let size_bytes = estimate_package_size(&package);

        // Check if we need to evict
        while self.entries.len() >= self.capacity {
            if let Some(evicted_key) = self.access_order.pop_front() {
                if let Some(evicted) = self.entries.remove(&evicted_key) {
                    self.current_size = self.current_size.saturating_sub(evicted.size_bytes);
                    return Some(evicted);
                }
            }
        }

        // Check size constraints
        if let Some(max_size) = self.max_size_bytes {
            while self.current_size + size_bytes > max_size && !self.access_order.is_empty() {
                if let Some(evicted_key) = self.access_order.pop_front() {
                    if let Some(evicted) = self.entries.remove(&evicted_key) {
                        self.current_size = self.current_size.saturating_sub(evicted.size_bytes);
                    }
                }
            }
        }

        let entry = CacheEntry {
            package: Arc::clone(&package),
            last_accessed: Instant::now(),
            access_count: 1,
            size_bytes,
        };

        self.entries.insert(Arc::clone(&key), entry);
        self.access_order.push_back(key);
        self.current_size += size_bytes;

        None
    }

    fn remove(&mut self, key: &Arc<str>) -> Option<CacheEntry> {
        if let Some(entry) = self.entries.remove(key) {
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                self.access_order.remove(pos);
            }
            self.current_size = self.current_size.saturating_sub(entry.size_bytes);
            Some(entry)
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
        self.current_size = 0;
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Disk-based cache for L3 tier
struct DiskCache {
    /// Cache directory path
    cache_dir: String,
    /// Index of cached items
    index: HashMap<Arc<str>, DiskCacheEntry>,
    /// Maximum cache size in bytes
    max_size_bytes: usize,
    /// Current cache size
    current_size: usize,
}

#[derive(Clone, Serialize, Deserialize)]
struct DiskCacheEntry {
    /// File path relative to cache directory
    file_path: String,
    /// Package hash for verification
    hash: String,
    /// Entry size in bytes
    size_bytes: usize,
    /// Last accessed timestamp
    last_accessed: i64,
}

impl DiskCache {
    async fn new(cache_dir: String, max_size_mb: usize) -> Result<Self> {
        // Create cache directory if it doesn't exist
        tokio::fs::create_dir_all(&cache_dir).await
            .context("Failed to create cache directory")?;

        // Load existing index if present
        let index_path = format!("{}/index.json", cache_dir);
        let index = if tokio::fs::metadata(&index_path).await.is_ok() {
            let index_data = tokio::fs::read_to_string(&index_path).await?;
            serde_json::from_str(&index_data).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(Self {
            cache_dir,
            index,
            max_size_bytes: max_size_mb * 1024 * 1024,
            current_size: 0,
        })
    }

    async fn get(&mut self, key: &Arc<str>) -> Result<Option<Arc<LibraryAssetPackage>>> {
        if let Some(entry) = self.index.get_mut(key) {
            let file_path = format!("{}/{}", self.cache_dir, entry.file_path);

            match tokio::fs::read(&file_path).await {
                Ok(data) => {
                    // Update access time
                    entry.last_accessed = chrono::Utc::now().timestamp();

                    // Deserialize package
                    let package: LibraryAssetPackage = bincode::deserialize(&data)
                        .context("Failed to deserialize cached package")?;

                    Ok(Some(Arc::new(package)))
                }
                Err(_) => {
                    // Remove invalid entry
                    self.index.remove(key);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn insert(&mut self, key: Arc<str>, package: Arc<LibraryAssetPackage>) -> Result<()> {
        // Serialize package
        let data = bincode::serialize(&*package)
            .context("Failed to serialize package")?;

        let size_bytes = data.len();

        // Check if we need to evict
        while self.current_size + size_bytes > self.max_size_bytes && !self.index.is_empty() {
            // Find oldest entry
            let oldest_key = self.index
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(k, _)| Arc::clone(k));

            if let Some(evict_key) = oldest_key {
                self.evict(&evict_key).await?;
            } else {
                break;
            }
        }

        // Write to disk
        let file_name = format!("{}.cache", hex::encode(key.as_bytes()));
        let file_path = format!("{}/{}", self.cache_dir, file_name);

        tokio::fs::write(&file_path, &data).await
            .context("Failed to write cache file")?;

        // Update index
        self.index.insert(key, DiskCacheEntry {
            file_path: file_name,
            hash: package.hash.to_string(),
            size_bytes,
            last_accessed: chrono::Utc::now().timestamp(),
        });

        self.current_size += size_bytes;

        // Save index
        self.save_index().await?;

        Ok(())
    }

    async fn evict(&mut self, key: &Arc<str>) -> Result<()> {
        if let Some(entry) = self.index.remove(key) {
            let file_path = format!("{}/{}", self.cache_dir, entry.file_path);
            let _ = tokio::fs::remove_file(&file_path).await;
            self.current_size = self.current_size.saturating_sub(entry.size_bytes);
        }
        Ok(())
    }

    async fn save_index(&self) -> Result<()> {
        let index_path = format!("{}/index.json", self.cache_dir);
        let index_data = serde_json::to_string(&self.index)?;
        tokio::fs::write(&index_path, index_data).await?;
        Ok(())
    }
}

/// Cache layer identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLayer {
    L1,
    L2,
    L3,
}

impl PackageCache {
    /// Create a new package cache
    pub fn new(l1_capacity: usize, l2_capacity: usize, l3_path: Option<String>) -> Self {
        Self {
            l1_cache: Arc::new(RwLock::new(LRUCache::new(l1_capacity))),
            l2_cache: Arc::new(RwLock::new(LRUCache::new(l2_capacity))),
            l3_cache_path: l3_path,
            l3_cache: None,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Initialize the cache (required for L3)
    pub async fn initialize(&self) -> Result<()> {
        // Initialize L3 cache if path is provided
        if let Some(path) = &self.l3_cache_path {
            let disk_cache = DiskCache::new(path.clone(), 1000).await?; // 1GB default
            // Note: In actual implementation, would store this properly
        }
        Ok(())
    }

    /// Get a package from cache
    pub async fn get(&self, key: &Arc<str>) -> Result<Option<Arc<LibraryAssetPackage>>> {
        let mut stats = self.stats.write().await;

        // Try L1 cache
        {
            let mut l1 = self.l1_cache.write().await;
            if let Some(package) = l1.get(key) {
                stats.l1_hits += 1;
                stats.total_hits += 1;
                return Ok(Some(package));
            }
        }

        // Try L2 cache
        {
            let mut l2 = self.l2_cache.write().await;
            if let Some(package) = l2.get(key) {
                stats.l2_hits += 1;
                stats.total_hits += 1;

                // Promote to L1
                let mut l1 = self.l1_cache.write().await;
                if let Some(evicted) = l1.insert(Arc::clone(key), Arc::clone(&package)) {
                    // Move evicted to L2 (it's already there, just update)
                    drop(l1);
                }

                return Ok(Some(package));
            }
        }

        // Try L3 cache if available
        if let Some(l3) = &self.l3_cache {
            let mut l3 = l3.write().await;
            if let Some(package) = l3.get(key).await? {
                stats.l3_hits += 1;
                stats.total_hits += 1;

                // Promote to L1
                let mut l1 = self.l1_cache.write().await;
                if let Some(evicted) = l1.insert(Arc::clone(key), Arc::clone(&package)) {
                    // Move evicted to L2
                    drop(l1);
                    let mut l2 = self.l2_cache.write().await;
                    l2.insert(Arc::from(evicted.package.id.as_ref()), evicted.package);
                }

                return Ok(Some(package));
            }
        }

        stats.total_misses += 1;
        Ok(None)
    }

    /// Insert a package into cache
    pub async fn insert(&self, key: Arc<str>, package: Arc<LibraryAssetPackage>) -> Result<()> {
        // Insert into L1
        let mut l1 = self.l1_cache.write().await;
        if let Some(evicted) = l1.insert(Arc::clone(&key), package) {
            // Move evicted to L2
            drop(l1);
            let mut l2 = self.l2_cache.write().await;

            let evicted_key = Arc::from(evicted.package.id.as_ref());
            if let Some(l2_evicted) = l2.insert(evicted_key.clone(), evicted.package) {
                // Move L2 evicted to L3 if available
                drop(l2);
                if let Some(l3) = &self.l3_cache {
                    let mut l3 = l3.write().await;
                    l3.insert(Arc::from(l2_evicted.package.id.as_ref()), l2_evicted.package).await?;
                }
            }
        }

        Ok(())
    }

    /// Remove a package from all cache tiers
    pub async fn remove(&self, key: &Arc<str>) -> Result<()> {
        let mut l1 = self.l1_cache.write().await;
        l1.remove(key);
        drop(l1);

        let mut l2 = self.l2_cache.write().await;
        l2.remove(key);
        drop(l2);

        if let Some(l3) = &self.l3_cache {
            let mut l3 = l3.write().await;
            l3.evict(key).await?;
        }

        Ok(())
    }

    /// Clear all cache tiers
    pub async fn clear(&self) -> Result<()> {
        let mut l1 = self.l1_cache.write().await;
        l1.clear();
        drop(l1);

        let mut l2 = self.l2_cache.write().await;
        l2.clear();
        drop(l2);

        // L3 cache clearing would be implemented here

        let mut stats = self.stats.write().await;
        *stats = CacheStats::default();

        Ok(())
    }

    /// Optimize cache by reorganizing tiers
    pub async fn optimize(&self) -> Result<()> {
        // This would implement tier optimization strategies
        // For now, just a placeholder
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> Result<CacheStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

/// Estimate package size in bytes for cache management
fn estimate_package_size(package: &LibraryAssetPackage) -> usize {
    // Rough estimation based on content
    let metadata_size = package.metadata.name.len() +
                       package.metadata.version.len() +
                       package.metadata.description.as_ref().map_or(0, |d| d.len());

    let content_size = package.content_refs.total_size as usize;

    // Add some overhead for structure
    metadata_size + content_size + 1024
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::types::*;

    #[tokio::test]
    async fn test_lru_cache() {
        let mut cache = LRUCache::new(2);

        let package1 = create_test_package("pkg1");
        let package2 = create_test_package("pkg2");
        let package3 = create_test_package("pkg3");

        let key1: Arc<str> = Arc::from("pkg1");
        let key2: Arc<str> = Arc::from("pkg2");
        let key3: Arc<str> = Arc::from("pkg3");

        // Insert packages
        cache.insert(key1.clone(), package1.clone());
        cache.insert(key2.clone(), package2.clone());

        // Cache should have 2 items
        assert_eq!(cache.len(), 2);

        // Get pkg1 (moves it to most recently used)
        assert!(cache.get(&key1).is_some());

        // Insert pkg3 (should evict pkg2 since pkg1 was just accessed)
        cache.insert(key3.clone(), package3.clone());

        // pkg2 should be evicted
        assert!(cache.get(&key2).is_none());
        assert!(cache.get(&key1).is_some());
        assert!(cache.get(&key3).is_some());
    }

    #[tokio::test]
    async fn test_multi_tier_cache() {
        let cache = PackageCache::new(1, 2, None);

        let package1 = Arc::new(create_test_package("pkg1"));
        let package2 = Arc::new(create_test_package("pkg2"));
        let package3 = Arc::new(create_test_package("pkg3"));

        let key1: Arc<str> = Arc::from("pkg1");
        let key2: Arc<str> = Arc::from("pkg2");
        let key3: Arc<str> = Arc::from("pkg3");

        // Insert packages
        cache.insert(key1.clone(), package1.clone()).await.unwrap();
        cache.insert(key2.clone(), package2.clone()).await.unwrap();
        cache.insert(key3.clone(), package3.clone()).await.unwrap();

        // pkg3 should be in L1 (most recent)
        // pkg2 should be in L2 (evicted from L1)
        // pkg1 should be in L2 (evicted from L1)

        // Get pkg1 (should promote from L2 to L1)
        assert!(cache.get(&key1).await.unwrap().is_some());

        let stats = cache.get_stats().await.unwrap();
        assert_eq!(stats.l2_hits, 1);
    }

    fn create_test_package(id: &str) -> LibraryAssetPackage {
        LibraryAssetPackage {
            id: Arc::from(id),
            metadata: PackageMetadata {
                name: Arc::from(id),
                version: Arc::from("1.0.0"),
                description: None,
                author: None,
                license: None,
                tags: Arc::new([]),
                keywords: Arc::new([]),
                created: 0,
                modified: 0,
            },
            spec: PackageSpec {
                asset_type: AssetType::JuliaProgram,
                resources: ResourceRequirements::default(),
                security: SecurityConfig {
                    consensus_required: false,
                    sandbox_level: SandboxLevel::Standard,
                    network_access: false,
                    filesystem_access: FilesystemAccess::ReadOnly,
                    permissions: Arc::new([]),
                },
                execution: ExecutionConfig {
                    strategy: ExecutionStrategy::NearestNode,
                    min_consensus: 1,
                    max_concurrent: None,
                    priority: ExecutionPriority::Normal,
                    retry_policy: RetryPolicy::default(),
                },
                dependencies: Arc::new([]),
                environment: Arc::new(HashMap::new()),
            },
            content_refs: ContentReferences {
                main_ref: ContentRef {
                    path: Arc::from("main.jl"),
                    hash: Arc::from("hash"),
                    size: 100,
                    content_type: ContentType::Source,
                },
                file_refs: Arc::new([]),
                binary_refs: Arc::new([]),
                total_size: 100,
            },
            validation: None,
            hash: Arc::from("hash"),
        }
    }
}