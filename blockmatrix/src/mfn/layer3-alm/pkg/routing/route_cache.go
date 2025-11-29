// Package routing implements high-performance route caching for the routing table
package routing

import (
	"fmt"
	"sync"
	"time"

	lru "github.com/hashicorp/golang-lru"
)

// RouteCache provides intelligent caching of routing entries with TTL and invalidation
type RouteCache struct {
	cache    *lru.ARCCache
	ttl      time.Duration
	
	// Statistics
	stats    *RouteCacheStats
	
	// Thread safety
	mutex    sync.RWMutex
}

// RouteCacheStats tracks cache performance
type RouteCacheStats struct {
	Hits          int64
	Misses        int64
	Puts          int64
	Invalidations int64
	
	mutex sync.Mutex
}

// NewRouteCache creates a new route cache
func NewRouteCache(size int, ttl time.Duration) *RouteCache {
	cache, _ := lru.NewARC(size)
	
	return &RouteCache{
		cache: cache,
		ttl:   ttl,
		stats: &RouteCacheStats{},
	}
}

// Get retrieves a route from the cache if valid
func (rc *RouteCache) Get(key string) *RouteEntry {
	rc.mutex.RLock()
	defer rc.mutex.RUnlock()
	
	if value, ok := rc.cache.Get(key); ok {
		route := value.(*RouteEntry)
		
		// Check if route has expired
		if time.Since(route.CreatedAt) > rc.ttl {
			rc.cache.Remove(key)
			rc.stats.recordInvalidation()
			rc.stats.recordMiss()
			return nil
		}
		
		// Update access time
		route.LastUsed = time.Now()
		route.UseCount++
		
		rc.stats.recordHit()
		return route
	}
	
	rc.stats.recordMiss()
	return nil
}

// GetByKey retrieves a route by key without updating access stats
func (rc *RouteCache) GetByKey(key string) *RouteEntry {
	rc.mutex.RLock()
	defer rc.mutex.RUnlock()
	
	if value, ok := rc.cache.Peek(key); ok {
		route := value.(*RouteEntry)
		
		// Check if route has expired
		if time.Since(route.CreatedAt) > rc.ttl {
			return nil
		}
		
		return route
	}
	
	return nil
}

// Put stores a route in the cache
func (rc *RouteCache) Put(key string, route *RouteEntry) {
	rc.mutex.Lock()
	defer rc.mutex.Unlock()
	
	rc.cache.Add(key, route)
	rc.stats.recordPut()
}

// Invalidate removes a route from the cache
func (rc *RouteCache) Invalidate(key string) {
	rc.mutex.Lock()
	defer rc.mutex.Unlock()
	
	if rc.cache.Remove(key) {
		rc.stats.recordInvalidation()
	}
}

// InvalidateByDestination removes all routes to a destination
func (rc *RouteCache) InvalidateByDestination(destination int64) int {
	rc.mutex.Lock()
	defer rc.mutex.Unlock()
	
	keys := rc.cache.Keys()
	removed := 0
	
	for _, keyInterface := range keys {
		key := keyInterface.(string)
		if value, ok := rc.cache.Peek(key); ok {
			route := value.(*RouteEntry)
			if route.Destination == destination {
				rc.cache.Remove(key)
				removed++
			}
		}
	}
	
	rc.stats.recordInvalidations(int64(removed))
	return removed
}

// InvalidateByPath removes all routes containing specific nodes
func (rc *RouteCache) InvalidateByPath(nodeIDs []int64) int {
	rc.mutex.Lock()
	defer rc.mutex.Unlock()
	
	keys := rc.cache.Keys()
	removed := 0
	
	for _, keyInterface := range keys {
		key := keyInterface.(string)
		if value, ok := rc.cache.Peek(key); ok {
			route := value.(*RouteEntry)
			
			// Check if route contains any of the specified nodes
			for _, routeNode := range route.Path {
				for _, invalidNodeID := range nodeIDs {
					if routeNode.ID == invalidNodeID {
						rc.cache.Remove(key)
						removed++
						goto nextRoute
					}
				}
			}
			nextRoute:
		}
	}
	
	rc.stats.recordInvalidations(int64(removed))
	return removed
}

// Purge removes all entries from the cache
func (rc *RouteCache) Purge() {
	rc.mutex.Lock()
	defer rc.mutex.Unlock()
	
	size := rc.cache.Len()
	rc.cache.Purge()
	rc.stats.recordInvalidations(int64(size))
}

// Size returns the current cache size
func (rc *RouteCache) Size() int {
	rc.mutex.RLock()
	defer rc.mutex.RUnlock()
	
	return rc.cache.Len()
}

// GetStats returns cache statistics
func (rc *RouteCache) GetStats() RouteCacheStatistics {
	rc.stats.mutex.Lock()
	defer rc.stats.mutex.Unlock()
	
	total := rc.stats.Hits + rc.stats.Misses
	hitRate := 0.0
	if total > 0 {
		hitRate = float64(rc.stats.Hits) / float64(total) * 100.0
	}
	
	return RouteCacheStatistics{
		Hits:          rc.stats.Hits,
		Misses:        rc.stats.Misses,
		Puts:          rc.stats.Puts,
		Invalidations: rc.stats.Invalidations,
		HitRate:       hitRate,
		Size:          rc.Size(),
	}
}

// CleanupExpired removes expired entries from the cache
func (rc *RouteCache) CleanupExpired() int {
	rc.mutex.Lock()
	defer rc.mutex.Unlock()
	
	keys := rc.cache.Keys()
	removed := 0
	
	for _, keyInterface := range keys {
		key := keyInterface.(string)
		if value, ok := rc.cache.Peek(key); ok {
			route := value.(*RouteEntry)
			if time.Since(route.CreatedAt) > rc.ttl {
				rc.cache.Remove(key)
				removed++
			}
		}
	}
	
	rc.stats.recordInvalidations(int64(removed))
	return removed
}

// GetMostUsedRoutes returns the most frequently used routes
func (rc *RouteCache) GetMostUsedRoutes(limit int) []*RouteEntry {
	rc.mutex.RLock()
	defer rc.mutex.RUnlock()
	
	type routeUsage struct {
		route *RouteEntry
		usage int64
	}
	
	keys := rc.cache.Keys()
	routeUsages := make([]routeUsage, 0, len(keys))
	
	for _, keyInterface := range keys {
		key := keyInterface.(string)
		if value, ok := rc.cache.Peek(key); ok {
			route := value.(*RouteEntry)
			routeUsages = append(routeUsages, routeUsage{
				route: route,
				usage: route.UseCount,
			})
		}
	}
	
	// Sort by usage count (simple bubble sort for small datasets)
	for i := 0; i < len(routeUsages)-1; i++ {
		for j := 0; j < len(routeUsages)-i-1; j++ {
			if routeUsages[j].usage < routeUsages[j+1].usage {
				routeUsages[j], routeUsages[j+1] = routeUsages[j+1], routeUsages[j]
			}
		}
	}
	
	// Extract routes up to limit
	result := make([]*RouteEntry, 0, limit)
	for i, ru := range routeUsages {
		if i >= limit {
			break
		}
		result = append(result, ru.route)
	}
	
	return result
}

// RouteCacheStatistics provides cache performance metrics
type RouteCacheStatistics struct {
	Hits          int64
	Misses        int64
	Puts          int64
	Invalidations int64
	HitRate       float64
	Size          int
}

// Statistics recording methods

func (rcs *RouteCacheStats) recordHit() {
	rcs.mutex.Lock()
	defer rcs.mutex.Unlock()
	rcs.Hits++
}

func (rcs *RouteCacheStats) recordMiss() {
	rcs.mutex.Lock()
	defer rcs.mutex.Unlock()
	rcs.Misses++
}

func (rcs *RouteCacheStats) recordPut() {
	rcs.mutex.Lock()
	defer rcs.mutex.Unlock()
	rcs.Puts++
}

func (rcs *RouteCacheStats) recordInvalidation() {
	rcs.mutex.Lock()
	defer rcs.mutex.Unlock()
	rcs.Invalidations++
}

func (rcs *RouteCacheStats) recordInvalidations(count int64) {
	rcs.mutex.Lock()
	defer rcs.mutex.Unlock()
	rcs.Invalidations += count
}

// String provides a string representation of cache statistics
func (rcs RouteCacheStatistics) String() string {
	return fmt.Sprintf("Cache Stats - Hits: %d, Misses: %d, Hit Rate: %.2f%%, Size: %d, Invalidations: %d",
		rcs.Hits, rcs.Misses, rcs.HitRate, rcs.Size, rcs.Invalidations)
}