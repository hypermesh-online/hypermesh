// Package routing implements intelligent load balancing for routing decisions
package routing

import (
	"math"
	"sync"
	"time"

	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/graph"
)

// LoadBalancer manages load balancing across multiple routing paths
type LoadBalancer struct {
	// Load tracking per path/node
	pathLoads    map[string]*PathLoadInfo
	nodeLoads    map[int64]*NodeLoadInfo
	
	// Configuration
	threshold    float64
	
	// Statistics
	stats        *LoadBalancerStats
	
	// Thread safety
	mutex        sync.RWMutex
}

// PathLoadInfo tracks load information for a specific path
type PathLoadInfo struct {
	PathID       string
	CurrentLoad  float64
	MaxCapacity  float64
	LastUpdated  time.Time
	
	// Moving averages
	LoadEMA      *ExponentialMovingAverage
	LatencyEMA   *ExponentialMovingAverage
	
	// Quality metrics
	SuccessRate  float64
	FailureCount int64
	TotalCount   int64
}

// NodeLoadInfo tracks load information for individual nodes
type NodeLoadInfo struct {
	NodeID       int64
	CurrentLoad  float64
	MaxCapacity  float64
	LastUpdated  time.Time
	
	// Health status
	IsHealthy    bool
	LastHealthCheck time.Time
	
	// Performance metrics
	AverageLatency time.Duration
	PacketLoss     float64
	Jitter         time.Duration
}

// LoadBalancerStats tracks load balancer performance
type LoadBalancerStats struct {
	TotalDecisions      int64
	LoadBalancedDecisions int64
	FailoverEvents      int64
	HealthCheckFailures int64
	
	mutex sync.Mutex
}

// ExponentialMovingAverage implements EMA calculation
type ExponentialMovingAverage struct {
	value  float64
	alpha  float64
	count  int64
}

// LoadBalancingDecision represents the result of load balancing
type LoadBalancingDecision struct {
	SelectedPath     *RouteEntry
	AlternativePaths []*RouteEntry
	Reason          string
	LoadFactor      float64
	Confidence      float64
}

// NewLoadBalancer creates a new load balancer
func NewLoadBalancer(threshold float64) *LoadBalancer {
	return &LoadBalancer{
		pathLoads:  make(map[string]*PathLoadInfo),
		nodeLoads:  make(map[int64]*NodeLoadInfo),
		threshold:  threshold,
		stats:     &LoadBalancerStats{},
	}
}

// GetPathLoad returns the current load for a given path
func (lb *LoadBalancer) GetPathLoad(path []*graph.NetworkNode) float64 {
	lb.mutex.RLock()
	defer lb.mutex.RUnlock()
	
	pathID := lb.generatePathID(path)
	if loadInfo, exists := lb.pathLoads[pathID]; exists {
		return loadInfo.CurrentLoad
	}
	
	// Calculate load from constituent nodes
	totalLoad := 0.0
	validNodes := 0
	
	for _, node := range path {
		if nodeInfo, exists := lb.nodeLoads[node.ID]; exists {
			totalLoad += nodeInfo.CurrentLoad
			validNodes++
		}
	}
	
	if validNodes > 0 {
		return totalLoad / float64(validNodes)
	}
	
	return 0.5 // Default moderate load
}

// SelectOptimalPath selects the best path considering load balancing
func (lb *LoadBalancer) SelectOptimalPath(candidates []*RouteEntry) *LoadBalancingDecision {
	lb.mutex.Lock()
	defer lb.mutex.Unlock()
	
	lb.stats.recordDecision()
	
	if len(candidates) == 0 {
		return &LoadBalancingDecision{
			Reason: "no_candidates",
		}
	}
	
	if len(candidates) == 1 {
		return &LoadBalancingDecision{
			SelectedPath: candidates[0],
			Reason:      "single_option",
			LoadFactor:  lb.calculatePathLoad(candidates[0]),
			Confidence:  candidates[0].Confidence,
		}
	}
	
	// Calculate load scores for all candidates
	pathScores := make([]pathScore, len(candidates))
	
	for i, candidate := range candidates {
		load := lb.calculatePathLoad(candidate)
		health := lb.calculatePathHealth(candidate)
		quality := candidate.QualityScore
		
		// Combined score considering load, health, and quality
		score := (quality * 0.4) + ((1.0 - load) * 0.4) + (health * 0.2)
		
		pathScores[i] = pathScore{
			route: candidate,
			score: score,
			load:  load,
		}
	}
	
	// Sort by score (highest first)
	for i := 0; i < len(pathScores)-1; i++ {
		for j := 0; j < len(pathScores)-i-1; j++ {
			if pathScores[j].score < pathScores[j+1].score {
				pathScores[j], pathScores[j+1] = pathScores[j+1], pathScores[j]
			}
		}
	}
	
	selectedPath := pathScores[0].route
	selectedLoad := pathScores[0].load
	
	// Check if load balancing was triggered
	wasLoadBalanced := false
	reason := "best_score"
	
	if selectedLoad > lb.threshold && len(pathScores) > 1 {
		// Check if we selected a different path due to load balancing
		bestQualityRoute := candidates[0] // Assume first is highest quality
		if selectedPath != bestQualityRoute {
			wasLoadBalanced = true
			reason = "load_balanced"
			lb.stats.recordLoadBalance()
		}
	}
	
	// Prepare alternatives
	alternatives := make([]*RouteEntry, 0, len(candidates)-1)
	for _, ps := range pathScores[1:] {
		alternatives = append(alternatives, ps.route)
	}
	
	return &LoadBalancingDecision{
		SelectedPath:     selectedPath,
		AlternativePaths: alternatives,
		Reason:          reason,
		LoadFactor:      selectedLoad,
		Confidence:      selectedPath.Confidence,
	}
}

// UpdateMetrics updates load balancer metrics with actual performance data
func (lb *LoadBalancer) UpdateMetrics(destination int64, metrics RouteMetrics, success bool) {
	lb.mutex.Lock()
	defer lb.mutex.Unlock()
	
	// Update path load information based on actual metrics
	// This is a simplified implementation - in production would track specific paths
	
	// Update node loads based on latency and throughput
	loadFactor := lb.calculateLoadFromMetrics(metrics)
	
	// Update moving averages and statistics
	// Implementation would depend on specific path tracking
}

// GetLoadBalanceRate returns the percentage of decisions that involved load balancing
func (lb *LoadBalancer) GetLoadBalanceRate() float64 {
	lb.stats.mutex.Lock()
	defer lb.stats.mutex.Unlock()
	
	if lb.stats.TotalDecisions == 0 {
		return 0.0
	}
	
	return float64(lb.stats.LoadBalancedDecisions) / float64(lb.stats.TotalDecisions) * 100.0
}

// UpdateNodeHealth updates the health status of a node
func (lb *LoadBalancer) UpdateNodeHealth(nodeID int64, isHealthy bool, metrics NodeHealthMetrics) {
	lb.mutex.Lock()
	defer lb.mutex.Unlock()
	
	if nodeInfo, exists := lb.nodeLoads[nodeID]; exists {
		nodeInfo.IsHealthy = isHealthy
		nodeInfo.LastHealthCheck = time.Now()
		nodeInfo.AverageLatency = metrics.Latency
		nodeInfo.PacketLoss = metrics.PacketLoss
		nodeInfo.Jitter = metrics.Jitter
	} else {
		lb.nodeLoads[nodeID] = &NodeLoadInfo{
			NodeID:          nodeID,
			IsHealthy:       isHealthy,
			LastHealthCheck: time.Now(),
			AverageLatency:  metrics.Latency,
			PacketLoss:      metrics.PacketLoss,
			Jitter:          metrics.Jitter,
		}
	}
	
	if !isHealthy {
		lb.stats.recordHealthCheckFailure()
	}
}

// GetNodeHealth returns the health status of a node
func (lb *LoadBalancer) GetNodeHealth(nodeID int64) (bool, *NodeLoadInfo) {
	lb.mutex.RLock()
	defer lb.mutex.RUnlock()
	
	if nodeInfo, exists := lb.nodeLoads[nodeID]; exists {
		return nodeInfo.IsHealthy, nodeInfo
	}
	
	return true, nil // Assume healthy if no info
}

// GetLoadBalancerStats returns current load balancer statistics
func (lb *LoadBalancer) GetLoadBalancerStats() LoadBalancerStatistics {
	lb.stats.mutex.Lock()
	defer lb.stats.mutex.Unlock()
	
	return LoadBalancerStatistics{
		TotalDecisions:        lb.stats.TotalDecisions,
		LoadBalancedDecisions: lb.stats.LoadBalancedDecisions,
		LoadBalanceRate:       lb.GetLoadBalanceRate(),
		FailoverEvents:        lb.stats.FailoverEvents,
		HealthCheckFailures:   lb.stats.HealthCheckFailures,
		TrackedPaths:         len(lb.pathLoads),
		TrackedNodes:         len(lb.nodeLoads),
	}
}

// Helper types and methods

type pathScore struct {
	route *RouteEntry
	score float64
	load  float64
}

type NodeHealthMetrics struct {
	Latency    time.Duration
	PacketLoss float64
	Jitter     time.Duration
}

type LoadBalancerStatistics struct {
	TotalDecisions        int64
	LoadBalancedDecisions int64
	LoadBalanceRate       float64
	FailoverEvents        int64
	HealthCheckFailures   int64
	TrackedPaths         int
	TrackedNodes         int
}

// generatePathID creates a unique identifier for a path
func (lb *LoadBalancer) generatePathID(path []*graph.NetworkNode) string {
	if len(path) == 0 {
		return ""
	}
	
	pathID := ""
	for i, node := range path {
		if i > 0 {
			pathID += "-"
		}
		pathID += string(rune(node.ID))
	}
	return pathID
}

// calculatePathLoad calculates the current load for a path
func (lb *LoadBalancer) calculatePathLoad(route *RouteEntry) float64 {
	if route == nil || len(route.Path) == 0 {
		return 0.5
	}
	
	// Calculate load based on metrics
	latencyLoad := float64(route.Metrics.Latency.Microseconds()) / 10000.0 // Normalize
	throughputLoad := 1.0 - (route.Metrics.Throughput / 1000.0)           // Invert for load
	reliabilityLoad := 1.0 - route.Metrics.Reliability                    // Invert for load
	
	// Combine loads
	combinedLoad := (latencyLoad*0.4 + throughputLoad*0.4 + reliabilityLoad*0.2)
	
	// Clamp to 0-1 range
	return math.Max(0.0, math.Min(1.0, combinedLoad))
}

// calculatePathHealth calculates the overall health score for a path
func (lb *LoadBalancer) calculatePathHealth(route *RouteEntry) float64 {
	if route == nil || len(route.Path) == 0 {
		return 0.5
	}
	
	totalHealth := 0.0
	healthyNodes := 0
	
	for _, node := range route.Path {
		if nodeInfo, exists := lb.nodeLoads[node.ID]; exists {
			if nodeInfo.IsHealthy {
				totalHealth += 1.0
			}
			healthyNodes++
		} else {
			// Assume healthy if no info
			totalHealth += 1.0
			healthyNodes++
		}
	}
	
	if healthyNodes == 0 {
		return 0.5
	}
	
	return totalHealth / float64(healthyNodes)
}

// calculateLoadFromMetrics calculates load factor from route metrics
func (lb *LoadBalancer) calculateLoadFromMetrics(metrics RouteMetrics) float64 {
	// Simple load calculation based on latency and reliability
	latencyComponent := float64(metrics.Latency.Microseconds()) / 10000.0
	reliabilityComponent := 1.0 - metrics.Reliability
	
	return (latencyComponent + reliabilityComponent) / 2.0
}

// NewExponentialMovingAverage creates a new EMA calculator
func NewExponentialMovingAverage(alpha float64) *ExponentialMovingAverage {
	return &ExponentialMovingAverage{
		alpha: alpha,
	}
}

// Update updates the EMA with a new value
func (ema *ExponentialMovingAverage) Update(value float64) {
	if ema.count == 0 {
		ema.value = value
	} else {
		ema.value = ema.alpha*value + (1.0-ema.alpha)*ema.value
	}
	ema.count++
}

// Value returns the current EMA value
func (ema *ExponentialMovingAverage) Value() float64 {
	return ema.value
}

// Statistics recording methods

func (lbs *LoadBalancerStats) recordDecision() {
	lbs.mutex.Lock()
	defer lbs.mutex.Unlock()
	lbs.TotalDecisions++
}

func (lbs *LoadBalancerStats) recordLoadBalance() {
	lbs.mutex.Lock()
	defer lbs.mutex.Unlock()
	lbs.LoadBalancedDecisions++
}

func (lbs *LoadBalancerStats) recordFailover() {
	lbs.mutex.Lock()
	defer lbs.mutex.Unlock()
	lbs.FailoverEvents++
}

func (lbs *LoadBalancerStats) recordHealthCheckFailure() {
	lbs.mutex.Lock()
	defer lbs.mutex.Unlock()
	lbs.HealthCheckFailures++
}