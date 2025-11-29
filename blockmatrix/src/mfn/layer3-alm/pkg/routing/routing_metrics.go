// Package routing implements comprehensive metrics collection for routing performance
package routing

import (
	"fmt"
	"math"
	"sync"
	"time"
)

// RoutingMetrics tracks comprehensive performance metrics for the routing system
type RoutingMetrics struct {
	// Lookup statistics
	TotalLookups       int64
	SuccessfulLookups  int64
	FailedLookups      int64
	CacheHits          int64
	CacheMisses        int64
	
	// Timing statistics
	TotalLookupTime    time.Duration
	MinLookupTime      time.Duration
	MaxLookupTime      time.Duration
	
	// Route quality metrics
	totalRouteUpdates  int64
	successfulUpdates  int64
	failedUpdates      int64
	
	// Invalidation tracking
	totalInvalidations int64
	invalidationReasons map[string]int64
	
	// Moving averages
	lookupTimeEMA      *ExponentialMovingAverage
	
	// Historical data (last N lookups for percentile calculations)
	recentLookupTimes  []time.Duration
	maxHistorySize     int
	
	// Thread safety
	mutex              sync.RWMutex
}

// RoutingPerformanceReport provides detailed performance analysis
type RoutingPerformanceReport struct {
	// Overall statistics
	TotalLookups      int64
	SuccessRate       float64
	CacheHitRate      float64
	AverageLatency    time.Duration
	
	// Latency percentiles
	P50Latency        time.Duration
	P90Latency        time.Duration
	P95Latency        time.Duration
	P99Latency        time.Duration
	
	// Quality metrics
	RouteUpdateSuccessRate float64
	InvalidationRate      float64
	
	// Performance trends
	LookupTimeEMA         float64
	
	// Report metadata
	GeneratedAt           time.Time
	MeasurementPeriod     time.Duration
}

// NewRoutingMetrics creates a new routing metrics collector
func NewRoutingMetrics() *RoutingMetrics {
	return &RoutingMetrics{
		MinLookupTime:       time.Duration(math.MaxInt64),
		MaxLookupTime:       time.Duration(0),
		invalidationReasons: make(map[string]int64),
		lookupTimeEMA:       NewExponentialMovingAverage(0.1),
		recentLookupTimes:   make([]time.Duration, 0, 1000),
		maxHistorySize:      1000,
	}
}

// RecordSuccessfulLookup records a successful route lookup
func (rm *RoutingMetrics) RecordSuccessfulLookup(lookupTime time.Duration) {
	rm.mutex.Lock()
	defer rm.mutex.Unlock()
	
	rm.TotalLookups++
	rm.SuccessfulLookups++
	rm.TotalLookupTime += lookupTime
	
	// Update min/max
	if lookupTime < rm.MinLookupTime {
		rm.MinLookupTime = lookupTime
	}
	if lookupTime > rm.MaxLookupTime {
		rm.MaxLookupTime = lookupTime
	}
	
	// Update moving average
	rm.lookupTimeEMA.Update(float64(lookupTime.Nanoseconds()))
	
	// Add to recent history for percentile calculations
	rm.addToHistory(lookupTime)
}

// RecordFailedLookup records a failed route lookup
func (rm *RoutingMetrics) RecordFailedLookup(lookupTime time.Duration) {
	rm.mutex.Lock()
	defer rm.mutex.Unlock()
	
	rm.TotalLookups++
	rm.FailedLookups++
	rm.TotalLookupTime += lookupTime
	
	// Still update timing stats for failed lookups
	rm.addToHistory(lookupTime)
}

// RecordCacheHit records a cache hit
func (rm *RoutingMetrics) RecordCacheHit() {
	rm.mutex.Lock()
	defer rm.mutex.Unlock()
	
	rm.CacheHits++
}

// RecordCacheMiss records a cache miss
func (rm *RoutingMetrics) RecordCacheMiss() {
	rm.mutex.Lock()
	defer rm.mutex.Unlock()
	
	rm.CacheMisses++
}

// RecordRouteUpdate records a route performance update
func (rm *RoutingMetrics) RecordRouteUpdate(metrics RouteMetrics, success bool) {
	rm.mutex.Lock()
	defer rm.mutex.Unlock()
	
	rm.totalRouteUpdates++
	if success {
		rm.successfulUpdates++
	} else {
		rm.failedUpdates++
	}
}

// RecordInvalidation records a route invalidation with reason
func (rm *RoutingMetrics) RecordInvalidation(reason string) {
	rm.mutex.Lock()
	defer rm.mutex.Unlock()
	
	rm.totalInvalidations++
	rm.invalidationReasons[reason]++
}

// GetCacheHitRate returns the cache hit rate as a percentage
func (rm *RoutingMetrics) GetCacheHitRate() float64 {
	rm.mutex.RLock()
	defer rm.mutex.RUnlock()
	
	total := rm.CacheHits + rm.CacheMisses
	if total == 0 {
		return 0.0
	}
	
	return float64(rm.CacheHits) / float64(total) * 100.0
}

// GetSuccessRate returns the lookup success rate as a percentage
func (rm *RoutingMetrics) GetSuccessRate() float64 {
	rm.mutex.RLock()
	defer rm.mutex.RUnlock()
	
	if rm.TotalLookups == 0 {
		return 0.0
	}
	
	return float64(rm.SuccessfulLookups) / float64(rm.TotalLookups) * 100.0
}

// GetAverageLatency returns the average lookup latency
func (rm *RoutingMetrics) GetAverageLatency() time.Duration {
	rm.mutex.RLock()
	defer rm.mutex.RUnlock()
	
	if rm.TotalLookups == 0 {
		return 0
	}
	
	return rm.TotalLookupTime / time.Duration(rm.TotalLookups)
}

// GetInvalidationRate returns the rate of route invalidations
func (rm *RoutingMetrics) GetInvalidationRate() float64 {
	rm.mutex.RLock()
	defer rm.mutex.RUnlock()
	
	if rm.TotalLookups == 0 {
		return 0.0
	}
	
	return float64(rm.totalInvalidations) / float64(rm.TotalLookups) * 100.0
}

// CalculateLatencyPercentiles calculates latency percentiles from recent history
func (rm *RoutingMetrics) CalculateLatencyPercentiles() (p50, p90, p95, p99 time.Duration) {
	rm.mutex.RLock()
	defer rm.mutex.RUnlock()
	
	if len(rm.recentLookupTimes) == 0 {
		return 0, 0, 0, 0
	}
	
	// Create a copy and sort it
	times := make([]time.Duration, len(rm.recentLookupTimes))
	copy(times, rm.recentLookupTimes)
	
	// Simple sorting (for production, use sort.Slice)
	for i := 0; i < len(times)-1; i++ {
		for j := 0; j < len(times)-i-1; j++ {
			if times[j] > times[j+1] {
				times[j], times[j+1] = times[j+1], times[j]
			}
		}
	}
	
	// Calculate percentiles
	n := len(times)
	p50 = times[int(float64(n)*0.50)]
	p90 = times[int(float64(n)*0.90)]
	p95 = times[int(float64(n)*0.95)]
	p99 = times[int(float64(n)*0.99)]
	
	return p50, p90, p95, p99
}

// GeneratePerformanceReport creates a comprehensive performance report
func (rm *RoutingMetrics) GeneratePerformanceReport(measurementPeriod time.Duration) *RoutingPerformanceReport {
	rm.mutex.RLock()
	defer rm.mutex.RUnlock()
	
	p50, p90, p95, p99 := rm.CalculateLatencyPercentiles()
	
	return &RoutingPerformanceReport{
		TotalLookups:           rm.TotalLookups,
		SuccessRate:           rm.GetSuccessRate(),
		CacheHitRate:          rm.GetCacheHitRate(),
		AverageLatency:        rm.GetAverageLatency(),
		P50Latency:            p50,
		P90Latency:            p90,
		P95Latency:            p95,
		P99Latency:            p99,
		RouteUpdateSuccessRate: rm.getRouteUpdateSuccessRate(),
		InvalidationRate:      rm.GetInvalidationRate(),
		LookupTimeEMA:         rm.lookupTimeEMA.Value(),
		GeneratedAt:           time.Now(),
		MeasurementPeriod:     measurementPeriod,
	}
}

// GetInvalidationReasons returns a breakdown of invalidation reasons
func (rm *RoutingMetrics) GetInvalidationReasons() map[string]int64 {
	rm.mutex.RLock()
	defer rm.mutex.RUnlock()
	
	// Return a copy to prevent modification
	reasons := make(map[string]int64)
	for reason, count := range rm.invalidationReasons {
		reasons[reason] = count
	}
	
	return reasons
}

// Reset resets all metrics (useful for testing or periodic resets)
func (rm *RoutingMetrics) Reset() {
	rm.mutex.Lock()
	defer rm.mutex.Unlock()
	
	rm.TotalLookups = 0
	rm.SuccessfulLookups = 0
	rm.FailedLookups = 0
	rm.CacheHits = 0
	rm.CacheMisses = 0
	rm.TotalLookupTime = 0
	rm.MinLookupTime = time.Duration(math.MaxInt64)
	rm.MaxLookupTime = 0
	rm.totalRouteUpdates = 0
	rm.successfulUpdates = 0
	rm.failedUpdates = 0
	rm.totalInvalidations = 0
	rm.invalidationReasons = make(map[string]int64)
	rm.lookupTimeEMA = NewExponentialMovingAverage(0.1)
	rm.recentLookupTimes = rm.recentLookupTimes[:0]
}

// GetCurrentStats returns current statistics snapshot
func (rm *RoutingMetrics) GetCurrentStats() RoutingStatSnapshot {
	rm.mutex.RLock()
	defer rm.mutex.RUnlock()
	
	return RoutingStatSnapshot{
		TotalLookups:      rm.TotalLookups,
		SuccessfulLookups: rm.SuccessfulLookups,
		FailedLookups:     rm.FailedLookups,
		CacheHits:         rm.CacheHits,
		CacheMisses:       rm.CacheMisses,
		CacheHitRate:      rm.GetCacheHitRate(),
		SuccessRate:       rm.GetSuccessRate(),
		AverageLatency:    rm.GetAverageLatency(),
		MinLatency:        rm.MinLookupTime,
		MaxLatency:        rm.MaxLookupTime,
		InvalidationRate:  rm.GetInvalidationRate(),
		Timestamp:         time.Now(),
	}
}

// Helper methods

func (rm *RoutingMetrics) addToHistory(duration time.Duration) {
	if len(rm.recentLookupTimes) >= rm.maxHistorySize {
		// Remove oldest entry (FIFO)
		rm.recentLookupTimes = rm.recentLookupTimes[1:]
	}
	rm.recentLookupTimes = append(rm.recentLookupTimes, duration)
}

func (rm *RoutingMetrics) getRouteUpdateSuccessRate() float64 {
	if rm.totalRouteUpdates == 0 {
		return 0.0
	}
	
	return float64(rm.successfulUpdates) / float64(rm.totalRouteUpdates) * 100.0
}

// RoutingStatSnapshot provides a point-in-time snapshot of routing statistics
type RoutingStatSnapshot struct {
	TotalLookups      int64
	SuccessfulLookups int64
	FailedLookups     int64
	CacheHits         int64
	CacheMisses       int64
	CacheHitRate      float64
	SuccessRate       float64
	AverageLatency    time.Duration
	MinLatency        time.Duration
	MaxLatency        time.Duration
	InvalidationRate  float64
	Timestamp         time.Time
}

// Performance analysis methods

// IsPerformingWell returns whether the routing system is performing within acceptable parameters
func (rm *RoutingMetrics) IsPerformingWell() (bool, []string) {
	rm.mutex.RLock()
	defer rm.mutex.RUnlock()
	
	issues := make([]string, 0)
	
	// Check success rate
	successRate := rm.GetSuccessRate()
	if successRate < 95.0 {
		issues = append(issues, fmt.Sprintf("Low success rate: %.2f%%", successRate))
	}
	
	// Check cache hit rate
	cacheHitRate := rm.GetCacheHitRate()
	if cacheHitRate < 70.0 {
		issues = append(issues, fmt.Sprintf("Low cache hit rate: %.2f%%", cacheHitRate))
	}
	
	// Check average latency
	avgLatency := rm.GetAverageLatency()
	if avgLatency > 5*time.Millisecond {
		issues = append(issues, fmt.Sprintf("High average latency: %v", avgLatency))
	}
	
	// Check invalidation rate
	invalidationRate := rm.GetInvalidationRate()
	if invalidationRate > 10.0 {
		issues = append(issues, fmt.Sprintf("High invalidation rate: %.2f%%", invalidationRate))
	}
	
	return len(issues) == 0, issues
}

// String provides a human-readable summary of the metrics
func (rm *RoutingMetrics) String() string {
	stats := rm.GetCurrentStats()
	return fmt.Sprintf(
		"Routing Metrics: %d lookups (%.1f%% success), Cache: %.1f%% hit rate, Avg Latency: %v",
		stats.TotalLookups,
		stats.SuccessRate,
		stats.CacheHitRate,
		stats.AverageLatency,
	)
}