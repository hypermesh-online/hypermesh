// Package graph implements intelligent path caching for routing optimization
package graph

import (
	"fmt"
	"sync"
	"time"

	lru "github.com/hashicorp/golang-lru"
)

// PathCache provides intelligent caching of routing paths
type PathCache struct {
	cache      *lru.ARCCache
	stats      *CacheStats
	
	// Node invalidation tracking
	nodeInvalidation map[int64]time.Time
	
	mutex sync.RWMutex
}

// CacheKey represents a unique cache key for path queries
type CacheKey struct {
	From        int64
	To          int64
	Preferences PathPreferences
}

// CachedPath represents a cached routing path with metadata
type CachedPath struct {
	Path      *OptimalPath
	CreatedAt time.Time
	AccessAt  time.Time
	HitCount  int64
}

// CacheStats tracks cache performance metrics
type CacheStats struct {
	Hits        int64
	Misses      int64
	Evictions   int64
	Invalidations int64
	
	mutex sync.Mutex
}

// NewPathCache creates a new path cache with the specified capacity
func NewPathCache(capacity int) *PathCache {
	cache, _ := lru.NewARC(capacity)
	
	return &PathCache{
		cache:            cache,
		stats:            &CacheStats{},
		nodeInvalidation: make(map[int64]time.Time),
	}
}

// Get retrieves a cached path if available and valid
func (pc *PathCache) Get(from, to int64, preferences PathPreferences) *OptimalPath {
	pc.mutex.RLock()
	defer pc.mutex.RUnlock()
	
	key := pc.createKey(from, to, preferences)
	
	if value, ok := pc.cache.Get(key); ok {
		cached := value.(*CachedPath)
		
		// Check if path is still valid (no node invalidations after creation)
		if pc.isPathValid(cached) {
			cached.AccessAt = time.Now()
			cached.HitCount++
			
			pc.stats.recordHit()
			return cached.Path
		} else {
			// Remove invalid path
			pc.cache.Remove(key)
			pc.stats.recordInvalidation()
		}
	}
	
	pc.stats.recordMiss()
	return nil
}

// Put stores a path in the cache
func (pc *PathCache) Put(from, to int64, preferences PathPreferences, path *OptimalPath) {
	pc.mutex.Lock()
	defer pc.mutex.Unlock()
	
	key := pc.createKey(from, to, preferences)
	
	cached := &CachedPath{
		Path:      path,
		CreatedAt: time.Now(),
		AccessAt:  time.Now(),
		HitCount:  0,
	}
	
	pc.cache.Add(key, cached)
	pc.stats.recordPut()
}

// InvalidateNode invalidates all cached paths that include the specified node
func (pc *PathCache) InvalidateNode(nodeID int64) {
	pc.mutex.Lock()
	defer pc.mutex.Unlock()
	
	pc.nodeInvalidation[nodeID] = time.Now()
	
	// Remove paths that include this node
	keys := pc.cache.Keys()
	removed := 0
	
	for _, keyInterface := range keys {
		key := keyInterface.(string)
		if value, ok := pc.cache.Peek(key); ok {
			cached := value.(*CachedPath)
			
			// Check if path includes the invalidated node
			for _, pathNodeID := range cached.Path.NodeIDs {
				if pathNodeID == nodeID {
					pc.cache.Remove(key)
					removed++
					break
				}
			}
		}
	}
	
	pc.stats.recordInvalidations(int64(removed))
}

// InvalidateAll clears the entire cache
func (pc *PathCache) InvalidateAll() {
	pc.mutex.Lock()
	defer pc.mutex.Unlock()
	
	pc.cache.Purge()
	pc.nodeInvalidation = make(map[int64]time.Time)
}

// GetHitRate returns the cache hit rate as a percentage
func (pc *PathCache) GetHitRate() float64 {
	pc.stats.mutex.Lock()
	defer pc.stats.mutex.Unlock()
	
	total := pc.stats.Hits + pc.stats.Misses
	if total == 0 {
		return 0.0
	}
	
	return float64(pc.stats.Hits) / float64(total) * 100.0
}

// GetStats returns current cache statistics
func (pc *PathCache) GetStats() CacheStatistics {
	pc.stats.mutex.Lock()
	defer pc.stats.mutex.Unlock()
	
	return CacheStatistics{
		Hits:          pc.stats.Hits,
		Misses:        pc.stats.Misses,
		Evictions:     pc.stats.Evictions,
		Invalidations: pc.stats.Invalidations,
		HitRate:       pc.GetHitRate(),
		Size:          pc.cache.Len(),
	}
}

// createKey generates a unique cache key
func (pc *PathCache) createKey(from, to int64, preferences PathPreferences) string {
	return fmt.Sprintf("%d-%d-%.3f-%.3f-%.3f-%.3f",
		from, to,
		preferences.LatencyWeight,
		preferences.ThroughputWeight,
		preferences.ReliabilityWeight,
		preferences.CostWeight,
	)
}

// isPathValid checks if a cached path is still valid
func (pc *PathCache) isPathValid(cached *CachedPath) bool {
	// Check if any nodes in the path have been invalidated after the path was created
	for _, nodeID := range cached.Path.NodeIDs {
		if invalidTime, exists := pc.nodeInvalidation[nodeID]; exists {
			if invalidTime.After(cached.CreatedAt) {
				return false
			}
		}
	}
	
	// Check if path is too old (configurable TTL)
	maxAge := 5 * time.Minute
	if time.Since(cached.CreatedAt) > maxAge {
		return false
	}
	
	return true
}

// CacheStatistics provides cache performance metrics
type CacheStatistics struct {
	Hits          int64
	Misses        int64
	Evictions     int64
	Invalidations int64
	HitRate       float64
	Size          int
}

// recordHit increments the hit counter
func (cs *CacheStats) recordHit() {
	cs.mutex.Lock()
	defer cs.mutex.Unlock()
	cs.Hits++
}

// recordMiss increments the miss counter
func (cs *CacheStats) recordMiss() {
	cs.mutex.Lock()
	defer cs.mutex.Unlock()
	cs.Misses++
}

// recordEviction increments the eviction counter
func (cs *CacheStats) recordEviction() {
	cs.mutex.Lock()
	defer cs.mutex.Unlock()
	cs.Evictions++
}

// recordInvalidation increments the invalidation counter
func (cs *CacheStats) recordInvalidation() {
	cs.mutex.Lock()
	defer cs.mutex.Unlock()
	cs.Invalidations++
}

// recordInvalidations adds to the invalidation counter
func (cs *CacheStats) recordInvalidations(count int64) {
	cs.mutex.Lock()
	defer cs.mutex.Unlock()
	cs.Invalidations += count
}

// recordPut increments the put counter
func (cs *CacheStats) recordPut() {
	cs.mutex.Lock()
	defer cs.mutex.Unlock()
	// This is handled differently since LRU doesn't return eviction info
}