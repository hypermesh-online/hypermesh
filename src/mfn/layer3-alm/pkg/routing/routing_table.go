// Package routing implements high-performance routing table with intelligent caching
package routing

import (
	"context"
	"fmt"
	"sync"
	"time"

	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/graph"
	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/associative"
	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/optimization"
)

// RoutingTable implements an intelligent routing table with associative search
type RoutingTable struct {
	// Core components
	networkGraph  *graph.NetworkGraph
	searchEngine  *associative.SimpleAssociativeSearchEngine
	optimizer     *optimization.MultiObjectiveOptimizer
	
	// Routing cache with intelligent invalidation
	routeCache    *RouteCache
	
	// Load balancing
	loadBalancer  *LoadBalancer
	
	// Performance monitoring
	metrics       *RoutingMetrics
	
	// Configuration
	config        *RoutingConfig
	
	// Thread safety
	mutex         sync.RWMutex
}

// RouteEntry represents a cached routing entry
type RouteEntry struct {
	Destination    int64
	NextHop        int64
	Path           []*graph.NetworkNode
	Metrics        RouteMetrics
	QualityScore   float64
	CreatedAt      time.Time
	LastUsed       time.Time
	UseCount       int64
	
	// Associative data
	Associations   []associative.Association
	Confidence     float64
}

// RouteMetrics contains detailed routing metrics
type RouteMetrics struct {
	Latency       time.Duration
	Throughput    float64
	Reliability   float64
	Cost          float64
	HopCount      int
	Load          float64
	Jitter        time.Duration
	PacketLoss    float64
}

// RoutingRequest defines parameters for route lookup
type RoutingRequest struct {
	Source      int64
	Destination int64
	ServiceType string
	QoSClass    QoSClass
	Constraints RouteConstraints
	Context     context.Context
}

// RouteConstraints define hard limits for routing
type RouteConstraints struct {
	MaxLatency    time.Duration
	MinThroughput float64
	MinReliability float64
	MaxCost       float64
	MaxHops       int
	AvoidNodes    []int64
	PreferRegions []string
}

// QoSClass defines Quality of Service requirements
type QoSClass int

const (
	BestEffort QoSClass = iota
	LowLatency
	HighThroughput
	HighReliability
	CriticalMission
)

// RoutingResponse contains the routing decision
type RoutingResponse struct {
	Route          *RouteEntry
	Alternatives   []*RouteEntry
	DecisionTime   time.Duration
	CacheHit       bool
	Confidence     float64
	
	// Load balancing info
	LoadBalanced   bool
	SelectedReason string
}

// RoutingConfig configures the routing table
type RoutingConfig struct {
	// Cache settings
	CacheSize         int
	CacheTTL          time.Duration
	InvalidationDelay time.Duration
	
	// Route discovery
	MaxAlternatives   int
	SearchTimeout     time.Duration
	OptimizationLevel OptimizationLevel
	
	// Load balancing
	LoadBalanceThreshold float64
	HealthCheckInterval  time.Duration
	
	// Performance tuning
	MaxConcurrentLookups int
	StatisticsWindow     time.Duration
}

type OptimizationLevel int

const (
	FastLookup OptimizationLevel = iota
	BalancedOptimization
	DeepOptimization
)

// NewRoutingTable creates a new intelligent routing table
func NewRoutingTable(
	networkGraph *graph.NetworkGraph,
	searchEngine *associative.SimpleAssociativeSearchEngine,
	optimizer *optimization.MultiObjectiveOptimizer,
	config *RoutingConfig,
) *RoutingTable {
	if config == nil {
		config = DefaultRoutingConfig()
	}
	
	return &RoutingTable{
		networkGraph:  networkGraph,
		searchEngine:  searchEngine,
		optimizer:     optimizer,
		routeCache:    NewRouteCache(config.CacheSize, config.CacheTTL),
		loadBalancer:  NewLoadBalancer(config.LoadBalanceThreshold),
		metrics:       NewRoutingMetrics(),
		config:        config,
	}
}

// LookupRoute finds the optimal route for a destination
func (rt *RoutingTable) LookupRoute(request RoutingRequest) (*RoutingResponse, error) {
	startTime := time.Now()
	
	// Validate request
	if err := rt.validateRequest(request); err != nil {
		return nil, fmt.Errorf("invalid routing request: %w", err)
	}
	
	// Check cache first
	cacheKey := rt.createCacheKey(request)
	if cached := rt.routeCache.Get(cacheKey); cached != nil {
		rt.metrics.RecordCacheHit()
		
		// Verify route is still valid
		if rt.isRouteValid(cached, request) {
			response := &RoutingResponse{
				Route:        cached,
				DecisionTime: time.Since(startTime),
				CacheHit:     true,
				Confidence:   cached.Confidence,
			}
			
			cached.LastUsed = time.Now()
			cached.UseCount++
			return response, nil
		} else {
			rt.routeCache.Invalidate(cacheKey)
		}
	}
	
	rt.metrics.RecordCacheMiss()
	
	// Perform route discovery based on optimization level
	routes, err := rt.discoverRoutes(request)
	if err != nil {
		return nil, fmt.Errorf("route discovery failed: %w", err)
	}
	
	if len(routes) == 0 {
		return nil, fmt.Errorf("no valid routes found to destination %d", request.Destination)
	}
	
	// Select best route using load balancing
	selectedRoute, alternatives := rt.selectOptimalRoute(routes, request)
	
	// Cache the result
	rt.routeCache.Put(cacheKey, selectedRoute)
	
	// Update metrics
	rt.metrics.RecordSuccessfulLookup(time.Since(startTime))
	
	response := &RoutingResponse{
		Route:          selectedRoute,
		Alternatives:   alternatives,
		DecisionTime:   time.Since(startTime),
		CacheHit:       false,
		Confidence:     selectedRoute.Confidence,
		LoadBalanced:   len(alternatives) > 0,
		SelectedReason: rt.getSelectionReason(selectedRoute, alternatives),
	}
	
	return response, nil
}

// discoverRoutes finds candidate routes using different algorithms based on optimization level
func (rt *RoutingTable) discoverRoutes(request RoutingRequest) ([]*RouteEntry, error) {
	_, cancel := context.WithTimeout(request.Context, rt.config.SearchTimeout)
	defer cancel()
	
	var routes []*RouteEntry
	
	switch rt.config.OptimizationLevel {
	case FastLookup:
		// Use simple graph search for speed
		route, err := rt.fastGraphSearch(request)
		if err == nil {
			routes = append(routes, route)
		}
		
	case BalancedOptimization:
		// Use associative search for better results
		searchReq := rt.createSearchRequest(request)
		result, err := rt.searchEngine.Search(searchReq)
		if err == nil {
			route := rt.convertSearchResult(result, request)
			routes = append(routes, route)
			
			// Find alternatives using different preferences
			alternatives, _ := rt.findAlternativeRoutes(request, 2)
			routes = append(routes, alternatives...)
		}
		
	case DeepOptimization:
		// Use multi-objective optimization for best results
		optReq := rt.createOptimizationRequest(request)
		result, err := rt.optimizer.Optimize(optReq)
		if err == nil {
			for _, solution := range result.ParetoSolutions {
				route := rt.convertOptimizationSolution(solution, request)
				routes = append(routes, route)
			}
			
			// Limit to MaxAlternatives
			if len(routes) > rt.config.MaxAlternatives {
				routes = routes[:rt.config.MaxAlternatives]
			}
		}
	}
	
	// Filter routes by constraints
	validRoutes := rt.filterRoutesByConstraints(routes, request.Constraints)
	
	return validRoutes, nil
}

// selectOptimalRoute chooses the best route considering load balancing
func (rt *RoutingTable) selectOptimalRoute(routes []*RouteEntry, request RoutingRequest) (*RouteEntry, []*RouteEntry) {
	if len(routes) == 0 {
		return nil, nil
	}
	
	if len(routes) == 1 {
		return routes[0], nil
	}
	
	// Check if load balancing is needed
	primaryRoute := routes[0]
	currentLoad := rt.loadBalancer.GetPathLoad(primaryRoute.Path)
	
	if currentLoad > rt.config.LoadBalanceThreshold {
		// Select alternative route with lower load
		for i := 1; i < len(routes); i++ {
			altLoad := rt.loadBalancer.GetPathLoad(routes[i].Path)
			if altLoad < currentLoad {
				// Use alternative route
				alternatives := make([]*RouteEntry, 0, len(routes)-1)
				alternatives = append(alternatives, primaryRoute)
				for j, route := range routes {
					if j != i {
						alternatives = append(alternatives, route)
					}
				}
				return routes[i], alternatives
			}
		}
	}
	
	// Use primary route, return others as alternatives
	alternatives := routes[1:]
	return primaryRoute, alternatives
}

// UpdateRouteMetrics updates metrics for a route based on actual performance
func (rt *RoutingTable) UpdateRouteMetrics(destination int64, actualMetrics RouteMetrics, success bool) {
	rt.mutex.Lock()
	defer rt.mutex.Unlock()
	
	// Update route in cache if it exists
	cacheKey := fmt.Sprintf("dest-%d", destination)
	if route := rt.routeCache.GetByKey(cacheKey); route != nil {
		rt.updateRouteMetricsInternal(route, actualMetrics, success)
	}
	
	// Update associative search engine with feedback
	if rt.searchEngine != nil {
		reward := rt.calculateLearningReward(actualMetrics, success)
		// Update associations based on performance
		rt.updateAssociativeLearning(destination, actualMetrics, reward)
	}
	
	// Update load balancer
	rt.loadBalancer.UpdateMetrics(destination, actualMetrics, success)
	
	// Record metrics
	rt.metrics.RecordRouteUpdate(actualMetrics, success)
}

// InvalidateRoute removes a route from the cache
func (rt *RoutingTable) InvalidateRoute(destination int64, reason string) {
	rt.mutex.Lock()
	defer rt.mutex.Unlock()
	
	cacheKey := fmt.Sprintf("dest-%d", destination)
	rt.routeCache.Invalidate(cacheKey)
	
	rt.metrics.RecordInvalidation(reason)
}

// GetRoutingStats returns current routing table statistics
func (rt *RoutingTable) GetRoutingStats() RoutingStats {
	rt.mutex.RLock()
	defer rt.mutex.RUnlock()
	
	return RoutingStats{
		TotalLookups:      rt.metrics.TotalLookups,
		CacheHitRate:     rt.metrics.GetCacheHitRate(),
		AverageLatency:   rt.metrics.GetAverageLatency(),
		SuccessRate:      rt.metrics.GetSuccessRate(),
		CachedRoutes:     rt.routeCache.Size(),
		InvalidationRate: rt.metrics.GetInvalidationRate(),
		LoadBalanceRate:  rt.loadBalancer.GetLoadBalanceRate(),
	}
}

// Helper methods

func (rt *RoutingTable) validateRequest(request RoutingRequest) error {
	if request.Source == request.Destination {
		return fmt.Errorf("source and destination cannot be the same")
	}
	
	if request.Source <= 0 || request.Destination <= 0 {
		return fmt.Errorf("invalid node IDs")
	}
	
	return nil
}

func (rt *RoutingTable) createCacheKey(request RoutingRequest) string {
	return fmt.Sprintf("%d-%d-%s-%d", request.Source, request.Destination, 
		request.ServiceType, int(request.QoSClass))
}

func (rt *RoutingTable) isRouteValid(route *RouteEntry, request RoutingRequest) bool {
	// Check if route is too old
	if time.Since(route.CreatedAt) > rt.config.CacheTTL {
		return false
	}
	
	// Check if route meets current constraints
	return rt.meetsConstraints(route, request.Constraints)
}

func (rt *RoutingTable) meetsConstraints(route *RouteEntry, constraints RouteConstraints) bool {
	metrics := route.Metrics
	
	if constraints.MaxLatency > 0 && metrics.Latency > constraints.MaxLatency {
		return false
	}
	
	if constraints.MinThroughput > 0 && metrics.Throughput < constraints.MinThroughput {
		return false
	}
	
	if constraints.MinReliability > 0 && metrics.Reliability < constraints.MinReliability {
		return false
	}
	
	if constraints.MaxCost > 0 && metrics.Cost > constraints.MaxCost {
		return false
	}
	
	if constraints.MaxHops > 0 && metrics.HopCount > constraints.MaxHops {
		return false
	}
	
	// Check avoided nodes
	for _, nodeID := range route.Path {
		for _, avoidID := range constraints.AvoidNodes {
			if nodeID.ID == avoidID {
				return false
			}
		}
	}
	
	return true
}

// RoutingStats provides routing table statistics
type RoutingStats struct {
	TotalLookups     int64
	CacheHitRate     float64
	AverageLatency   time.Duration
	SuccessRate      float64
	CachedRoutes     int
	InvalidationRate float64
	LoadBalanceRate  float64
}

// DefaultRoutingConfig returns default routing configuration
func DefaultRoutingConfig() *RoutingConfig {
	return &RoutingConfig{
		CacheSize:            10000,
		CacheTTL:            5 * time.Minute,
		InvalidationDelay:   100 * time.Millisecond,
		MaxAlternatives:     3,
		SearchTimeout:       1 * time.Second,
		OptimizationLevel:   BalancedOptimization,
		LoadBalanceThreshold: 0.8,
		HealthCheckInterval: 30 * time.Second,
		MaxConcurrentLookups: 100,
		StatisticsWindow:    1 * time.Hour,
	}
}

// fastGraphSearch performs fast single-path search
func (rt *RoutingTable) fastGraphSearch(request RoutingRequest) (*RouteEntry, error) {
	path, err := rt.networkGraph.FindShortestPath(request.Source, request.Destination)
	if err != nil {
		return nil, err
	}
	
	// Calculate route metrics
	metrics := rt.calculatePathMetrics(path)
	
	return &RouteEntry{
		Destination:  request.Destination,
		NextHop:     path.NodeIDs[1], // First hop after source
		Path:        path.Nodes,
		Metrics:     metrics,
		QualityScore: rt.calculateQualityScore(metrics, request.QoSClass),
		CreatedAt:   time.Now(),
		LastUsed:    time.Now(),
		UseCount:    0,
		Confidence:  0.8, // High confidence for fast search
	}, nil
}

// createSearchRequest converts routing request to search request
func (rt *RoutingTable) createSearchRequest(request RoutingRequest) *associative.SearchRequest {
	return &associative.SearchRequest{
		SourceID:    request.Source,
		DestinationID: request.Destination,
		ServiceType: request.ServiceType,
		QoSClass:    int(request.QoSClass),
		MaxResults:  rt.config.MaxAlternatives,
		Timeout:     rt.config.SearchTimeout,
	}
}

// convertSearchResult converts search result to route entry
func (rt *RoutingTable) convertSearchResult(result *associative.SearchResult, request RoutingRequest) *RouteEntry {
	if result == nil || len(result.BestPath.Nodes) == 0 {
		return nil
	}
	
	metrics := rt.calculatePathMetrics(result.BestPath)
	
	return &RouteEntry{
		Destination:  request.Destination,
		NextHop:     result.BestPath.NodeIDs[1],
		Path:        result.BestPath.Nodes,
		Metrics:     metrics,
		QualityScore: rt.calculateQualityScore(metrics, request.QoSClass),
		CreatedAt:   time.Now(),
		LastUsed:    time.Now(),
		UseCount:    0,
		Associations: result.Associations,
		Confidence:  result.Confidence,
	}
}

// findAlternativeRoutes finds alternative routing paths
func (rt *RoutingTable) findAlternativeRoutes(request RoutingRequest, maxAlternatives int) ([]*RouteEntry, error) {
	alternatives := make([]*RouteEntry, 0, maxAlternatives)
	
	// Find alternative paths using different preferences
	for i := 0; i < maxAlternatives; i++ {
		// Modify preferences slightly for diversity
		modifiedRequest := request
		// Add some randomization or different weightings
		
		route, err := rt.fastGraphSearch(modifiedRequest)
		if err == nil {
			alternatives = append(alternatives, route)
		}
	}
	
	return alternatives, nil
}

// createOptimizationRequest converts routing request to optimization request
func (rt *RoutingTable) createOptimizationRequest(request RoutingRequest) *optimization.OptimizationRequest {
	return &optimization.OptimizationRequest{
		SourceID:     request.Source,
		TargetID:     request.Destination,
		Objectives:   nil, // Use default objectives
		Constraints:  rt.convertConstraints(request.Constraints),
		MaxSolutions: rt.config.MaxAlternatives,
		TimeLimit:    rt.config.SearchTimeout,
		Context:      request.Context,
	}
}

// convertOptimizationSolution converts optimization solution to route entry
func (rt *RoutingTable) convertOptimizationSolution(solution *optimization.RoutingSolution, request RoutingRequest) *RouteEntry {
	if solution == nil || len(solution.Path) == 0 {
		return nil
	}
	
	metrics := RouteMetrics{
		Latency:     solution.TotalLatency,
		Throughput:  solution.MinThroughput,
		Reliability: solution.AvgReliability,
		Cost:        solution.TotalCost,
		HopCount:    solution.HopCount,
	}
	
	return &RouteEntry{
		Destination:  request.Destination,
		NextHop:     solution.Path[1].ID, // First hop after source
		Path:        solution.Path,
		Metrics:     metrics,
		QualityScore: solution.Fitness,
		CreatedAt:   time.Now(),
		LastUsed:    time.Now(),
		UseCount:    0,
		Confidence:  0.95, // High confidence for optimized solutions
	}
}

// filterRoutesByConstraints filters routes that don't meet constraints
func (rt *RoutingTable) filterRoutesByConstraints(routes []*RouteEntry, constraints RouteConstraints) []*RouteEntry {
	filtered := make([]*RouteEntry, 0, len(routes))
	
	for _, route := range routes {
		if rt.meetsConstraints(route, constraints) {
			filtered = append(filtered, route)
		}
	}
	
	return filtered
}

// calculatePathMetrics calculates metrics for a path
func (rt *RoutingTable) calculatePathMetrics(path *graph.OptimalPath) RouteMetrics {
	return RouteMetrics{
		Latency:     time.Duration(path.TotalLatency) * time.Microsecond,
		Throughput:  path.MinThroughput,
		Reliability: path.AvgReliability,
		Cost:        path.TotalCost,
		HopCount:    len(path.NodeIDs) - 1,
		Load:        rt.calculatePathLoad(path),
		Jitter:      time.Duration(path.TotalJitter) * time.Microsecond,
		PacketLoss:  path.AvgPacketLoss,
	}
}

// calculateQualityScore calculates route quality score based on QoS class
func (rt *RoutingTable) calculateQualityScore(metrics RouteMetrics, qosClass QoSClass) float64 {
	switch qosClass {
	case LowLatency:
		return 1.0 / (1.0 + float64(metrics.Latency.Microseconds())/1000.0)
	case HighThroughput:
		return metrics.Throughput / 1000.0 // Normalize to 0-1
	case HighReliability:
		return metrics.Reliability
	case CriticalMission:
		return (metrics.Reliability * 0.5) + (1.0/(1.0+float64(metrics.Latency.Microseconds())/1000.0) * 0.5)
	default: // BestEffort
		return 0.8 // Default score
	}
}

// calculatePathLoad calculates current load on a path
func (rt *RoutingTable) calculatePathLoad(path *graph.OptimalPath) float64 {
	// This would integrate with load balancer to get real load
	return 0.5 // Placeholder
}

// convertConstraints converts routing constraints to optimization constraints
func (rt *RoutingTable) convertConstraints(constraints RouteConstraints) []optimization.OptimizationConstraint {
	return []optimization.OptimizationConstraint{} // Implementation needed
}

// updateRouteMetricsInternal updates route metrics internally
func (rt *RoutingTable) updateRouteMetricsInternal(route *RouteEntry, actualMetrics RouteMetrics, success bool) {
	// Update metrics with exponential moving average
	alpha := 0.1 // Learning rate
	
	route.Metrics.Latency = time.Duration(float64(route.Metrics.Latency)*(1-alpha) + float64(actualMetrics.Latency)*alpha)
	route.Metrics.Throughput = route.Metrics.Throughput*(1-alpha) + actualMetrics.Throughput*alpha
	route.Metrics.Reliability = route.Metrics.Reliability*(1-alpha) + actualMetrics.Reliability*alpha
	route.Metrics.Cost = route.Metrics.Cost*(1-alpha) + actualMetrics.Cost*alpha
	
	if !success {
		// Penalize route for failure
		route.Metrics.Reliability *= 0.9
		route.Confidence *= 0.95
	}
}

// calculateLearningReward calculates reward for associative learning
func (rt *RoutingTable) calculateLearningReward(metrics RouteMetrics, success bool) float64 {
	if !success {
		return -1.0
	}
	
	// Reward based on performance metrics
	reward := 0.0
	reward += 1.0 - float64(metrics.Latency.Microseconds())/10000.0 // Lower latency = higher reward
	reward += metrics.Throughput / 1000.0                            // Higher throughput = higher reward
	reward += metrics.Reliability                                    // Higher reliability = higher reward
	reward -= metrics.Cost / 100.0                                  // Lower cost = higher reward
	
	return reward / 4.0 // Normalize
}

// updateAssociativeLearning updates associative search engine with performance feedback
func (rt *RoutingTable) updateAssociativeLearning(destination int64, metrics RouteMetrics, reward float64) {
	// This would update the associative search engine with feedback
	// Implementation depends on the search engine's learning interface
}

// getSelectionReason returns reason for route selection
func (rt *RoutingTable) getSelectionReason(selected *RouteEntry, alternatives []*RouteEntry) string {
	if len(alternatives) == 0 {
		return "only_option"
	}
	
	if selected.QualityScore > alternatives[0].QualityScore+0.1 {
		return "best_quality"
	}
	
	return "load_balanced"
}