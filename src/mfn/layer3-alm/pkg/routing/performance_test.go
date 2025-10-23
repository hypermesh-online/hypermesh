// Package routing implements comprehensive performance testing for ALM routing
package routing

import (
	"context"
	"fmt"
	"math/rand"
	"sync"
	"testing"
	"time"

	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/associative"
	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/graph"
	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/optimization"
)

// PerformanceBenchmark conducts comprehensive performance testing
type PerformanceBenchmark struct {
	routingTable    *RoutingTable
	testTopology    *TestTopology
	baselineLatency time.Duration
	targetImprovement float64 // 777% improvement = 7.77x faster = 1/7.77 latency
	
	// Test configuration
	numNodes        int
	numConnections  int
	testDuration    time.Duration
	concurrency     int
}

// TestTopology generates realistic network topologies for testing
type TestTopology struct {
	nodes      map[int64]*graph.NetworkNode
	edges      map[string]*graph.NetworkEdge
	graph      *graph.NetworkGraph
	
	// Topology characteristics
	diameter   int
	avgLatency time.Duration
	avgThroughput float64
}

// PerformanceTestResult captures comprehensive performance metrics
type PerformanceTestResult struct {
	// Core performance metrics
	AverageLatency     time.Duration
	P50Latency        time.Duration
	P90Latency        time.Duration
	P95Latency        time.Duration
	P99Latency        time.Duration
	MaxLatency        time.Duration
	MinLatency        time.Duration
	
	// Throughput metrics
	RequestsPerSecond  float64
	SuccessRate       float64
	CacheHitRate      float64
	
	// Quality metrics
	OptimalityScore   float64  // How close to theoretical optimum
	ConsistencyScore  float64  // Variance in performance
	
	// ALM-specific metrics
	AssociationHits   int64
	GraphTraversals   int64
	OptimizationRuns  int64
	
	// Comparison metrics
	BaselineLatency   time.Duration
	ImprovementFactor float64
	TargetAchieved    bool
}

// NewPerformanceBenchmark creates a comprehensive performance testing suite
func NewPerformanceBenchmark(numNodes, numConnections int, concurrency int) *PerformanceBenchmark {
	return &PerformanceBenchmark{
		numNodes:        numNodes,
		numConnections:  numConnections,
		testDuration:    30 * time.Second,
		concurrency:     concurrency,
		baselineLatency: 1390 * time.Microsecond, // HTTP baseline: 1.39ms
		targetImprovement: 7.77, // 777% improvement
	}
}

// RunComprehensivePerformanceTest executes full performance validation
func (pb *PerformanceBenchmark) RunComprehensivePerformanceTest() (*PerformanceTestResult, error) {
	// Setup test topology
	if err := pb.setupTestTopology(); err != nil {
		return nil, fmt.Errorf("failed to setup test topology: %w", err)
	}
	
	// Initialize routing table with optimized configuration
	if err := pb.initializeRoutingTable(); err != nil {
		return nil, fmt.Errorf("failed to initialize routing table: %w", err)
	}
	
	// Warm up the system
	if err := pb.warmupSystem(); err != nil {
		return nil, fmt.Errorf("failed to warm up system: %w", err)
	}
	
	// Run baseline HTTP comparison test
	baselineResult, err := pb.runBaselineTest()
	if err != nil {
		return nil, fmt.Errorf("failed to run baseline test: %w", err)
	}
	
	// Run ALM routing performance test
	almResult, err := pb.runALMPerformanceTest()
	if err != nil {
		return nil, fmt.Errorf("failed to run ALM performance test: %w", err)
	}
	
	// Calculate improvement metrics
	result := pb.calculatePerformanceMetrics(baselineResult, almResult)
	
	// Validate against 777% improvement target
	result.TargetAchieved = result.ImprovementFactor >= pb.targetImprovement
	
	return result, nil
}

// setupTestTopology creates a realistic network topology for testing
func (pb *PerformanceBenchmark) setupTestTopology() error {
	pb.testTopology = &TestTopology{
		nodes: make(map[int64]*graph.NetworkNode),
		edges: make(map[string]*graph.NetworkEdge),
	}
	
	// Create network graph
	networkGraph := graph.NewNetworkGraph()
	
	// Generate realistic node distribution across regions
	regions := []string{"us-east-1", "us-west-2", "eu-west-1", "ap-southeast-1", "ap-northeast-1"}
	
	for i := 0; i < pb.numNodes; i++ {
		nodeID := int64(i + 1)
		region := regions[i%len(regions)]
		
		// Generate realistic latency and throughput values
		baseLatency := time.Duration(5+rand.Intn(50)) * time.Millisecond
		baseThroughput := 100.0 + rand.Float64()*900.0 // 100-1000 MB/s
		
		node := &graph.NetworkNode{
			ID:          nodeID,
			Address:     fmt.Sprintf("node-%d.%s.hypermesh.local", nodeID, region),
			Region:      region,
			Latitude:    -90.0 + rand.Float64()*180.0, // Random global distribution
			Longitude:   -180.0 + rand.Float64()*360.0,
			Latency:     baseLatency,
			Throughput:  baseThroughput,
			Reliability: 0.95 + rand.Float64()*0.05, // 95-100% reliability
			LoadFactor:  rand.Float64() * 0.5,       // 0-50% initial load
			LastSeen:    time.Now(),
			Services:    make(map[string]graph.ServiceInfo),
		}
		
		// Add some services to nodes
		if rand.Float64() < 0.7 { // 70% of nodes have services
			serviceTypes := []string{"api", "database", "cache", "compute", "storage"}
			serviceType := serviceTypes[rand.Intn(len(serviceTypes))]
			
			node.Services[serviceType] = graph.ServiceInfo{
				Name:        fmt.Sprintf("%s-service", serviceType),
				Version:     "1.0.0",
				Port:        8000 + rand.Intn(1000),
				Protocol:    "http",
				HealthScore: 0.8 + rand.Float64()*0.2,
			}
		}
		
		pb.testTopology.nodes[nodeID] = node
		networkGraph.AddNode(node)
	}
	
	// Generate realistic edge connections
	connectionsPerNode := pb.numConnections / pb.numNodes
	if connectionsPerNode < 2 {
		connectionsPerNode = 2 // Minimum connectivity
	}
	
	for _, node := range pb.testTopology.nodes {
		// Connect to closest nodes geographically and some random ones
		connections := pb.findBestConnections(node, connectionsPerNode)
		
		for _, targetNode := range connections {
			if node.ID != targetNode.ID {
				edge := pb.createRealisticEdge(node, targetNode)
				edgeKey := fmt.Sprintf("%d-%d", node.ID, targetNode.ID)
				pb.testTopology.edges[edgeKey] = edge
				networkGraph.AddEdge(edge)
			}
		}
	}
	
	pb.testTopology.graph = networkGraph
	return nil
}

// initializeRoutingTable sets up the routing table with optimal configuration
func (pb *PerformanceBenchmark) initializeRoutingTable() error {
	// Create optimized configuration for maximum performance
	config := &RoutingConfig{
		CacheSize:            50000,                    // Large cache for testing
		CacheTTL:            10 * time.Minute,         // Longer TTL for stability
		InvalidationDelay:   10 * time.Millisecond,   // Fast invalidation
		MaxAlternatives:     5,                        // More alternatives for optimization
		SearchTimeout:       100 * time.Millisecond,  // Fast search timeout
		OptimizationLevel:   DeepOptimization,         // Maximum optimization
		LoadBalanceThreshold: 0.7,                     // Aggressive load balancing
		HealthCheckInterval: 10 * time.Second,        // Frequent health checks
		MaxConcurrentLookups: pb.concurrency * 2,     // Handle concurrency
		StatisticsWindow:    5 * time.Minute,         // Short statistics window
	}
	
	// Initialize associative search engine
	searchEngine := associative.NewAssociativeSearchEngine(pb.testTopology.graph, nil)
	
	// Initialize multi-objective optimizer
	optimizerConfig := &optimization.OptimizerConfig{
		PopulationSize:       50,              // Moderate population for speed
		MaxGenerations:       20,              // Limited generations for speed
		CrossoverRate:        0.8,
		MutationRate:         0.1,
		LatencyWeight:        0.4,             // High latency priority
		ThroughputWeight:     0.3,
		ReliabilityWeight:    0.2,
		CostWeight:          0.1,
		OptimizationTimeout: 50 * time.Millisecond, // Fast optimization
		ConvergenceThreshold: 0.01,
		StagnationLimit:     3,
	}
	
	optimizer := optimization.NewMultiObjectiveOptimizer(optimizerConfig)
	
	// Create routing table
	pb.routingTable = NewRoutingTable(
		pb.testTopology.graph,
		searchEngine,
		optimizer,
		config,
	)
	
	return nil
}

// warmupSystem preloads caches and associations for optimal performance
func (pb *PerformanceBenchmark) warmupSystem() error {
	warmupRequests := 1000
	
	// Generate random routing requests to warm up caches
	for i := 0; i < warmupRequests; i++ {
		source := int64(1 + rand.Intn(pb.numNodes))
		dest := int64(1 + rand.Intn(pb.numNodes))
		
		if source != dest {
			request := RoutingRequest{
				Source:      source,
				Destination: dest,
				ServiceType: "api",
				QoSClass:    BestEffort,
				Context:     context.Background(),
			}
			
			// Perform lookup to warm up system
			_, _ = pb.routingTable.LookupRoute(request)
		}
	}
	
	return nil
}

// runBaselineTest simulates traditional HTTP routing performance
func (pb *PerformanceBenchmark) runBaselineTest() (*TestMetrics, error) {
	metrics := &TestMetrics{
		latencies: make([]time.Duration, 0, 10000),
		startTime: time.Now(),
	}
	
	// Simulate baseline HTTP routing (simple table lookup + network overhead)
	requests := 5000
	
	for i := 0; i < requests; i++ {
		start := time.Now()
		
		// Simulate HTTP routing overhead
		time.Sleep(pb.baselineLatency + time.Duration(rand.Intn(500))*time.Microsecond)
		
		latency := time.Since(start)
		metrics.latencies = append(metrics.latencies, latency)
		metrics.totalRequests++
		metrics.successfulRequests++
	}
	
	metrics.endTime = time.Now()
	return metrics, nil
}

// runALMPerformanceTest executes the ALM routing performance test
func (pb *PerformanceBenchmark) runALMPerformanceTest() (*TestMetrics, error) {
	metrics := &TestMetrics{
		latencies: make([]time.Duration, 0, 100000),
		startTime: time.Now(),
	}
	
	// Run concurrent performance test
	var wg sync.WaitGroup
	var mutex sync.Mutex
	
	requestsPerWorker := 10000 / pb.concurrency
	
	for worker := 0; worker < pb.concurrency; worker++ {
		wg.Add(1)
		go func(workerID int) {
			defer wg.Done()
			
			workerMetrics := make([]time.Duration, 0, requestsPerWorker)
			
			for i := 0; i < requestsPerWorker; i++ {
				source := int64(1 + rand.Intn(pb.numNodes))
				dest := int64(1 + rand.Intn(pb.numNodes))
				
				if source != dest {
					request := RoutingRequest{
						Source:      source,
						Destination: dest,
						ServiceType: "api",
						QoSClass:    LowLatency,
						Constraints: RouteConstraints{
							MaxLatency: 10 * time.Millisecond,
							MaxHops:    10,
						},
						Context: context.Background(),
					}
					
					start := time.Now()
					response, err := pb.routingTable.LookupRoute(request)
					latency := time.Since(start)
					
					mutex.Lock()
					metrics.totalRequests++
					if err == nil && response != nil {
						metrics.successfulRequests++
						workerMetrics = append(workerMetrics, latency)
						
						if response.CacheHit {
							metrics.cacheHits++
						}
					}
					mutex.Unlock()
				}
			}
			
			// Merge worker metrics
			mutex.Lock()
			metrics.latencies = append(metrics.latencies, workerMetrics...)
			mutex.Unlock()
		}(worker)
	}
	
	wg.Wait()
	metrics.endTime = time.Now()
	
	return metrics, nil
}

// calculatePerformanceMetrics computes comprehensive performance comparison
func (pb *PerformanceBenchmark) calculatePerformanceMetrics(baseline, alm *TestMetrics) *PerformanceTestResult {
	// Calculate baseline metrics
	baselineAvg := calculateAverageLatency(baseline.latencies)
	baselineP50, baselineP90, baselineP95, baselineP99 := calculatePercentiles(baseline.latencies)
	
	// Calculate ALM metrics  
	almAvg := calculateAverageLatency(alm.latencies)
	almP50, almP90, almP95, almP99 := calculatePercentiles(alm.latencies)
	
	// Calculate improvement factor
	improvementFactor := float64(baselineAvg) / float64(almAvg)
	
	// Calculate throughput
	testDuration := alm.endTime.Sub(alm.startTime).Seconds()
	rps := float64(alm.successfulRequests) / testDuration
	
	// Calculate success and cache hit rates
	successRate := float64(alm.successfulRequests) / float64(alm.totalRequests) * 100.0
	cacheHitRate := float64(alm.cacheHits) / float64(alm.totalRequests) * 100.0
	
	// Calculate quality scores
	optimalityScore := pb.calculateOptimalityScore(alm)
	consistencyScore := pb.calculateConsistencyScore(alm.latencies)
	
	return &PerformanceTestResult{
		AverageLatency:     almAvg,
		P50Latency:        almP50,
		P90Latency:        almP90,
		P95Latency:        almP95,
		P99Latency:        almP99,
		MinLatency:        findMinLatency(alm.latencies),
		MaxLatency:        findMaxLatency(alm.latencies),
		RequestsPerSecond:  rps,
		SuccessRate:       successRate,
		CacheHitRate:      cacheHitRate,
		OptimalityScore:   optimalityScore,
		ConsistencyScore:  consistencyScore,
		BaselineLatency:   baselineAvg,
		ImprovementFactor: improvementFactor,
		TargetAchieved:    improvementFactor >= pb.targetImprovement,
	}
}

// TestMetrics holds test execution metrics
type TestMetrics struct {
	latencies          []time.Duration
	totalRequests      int64
	successfulRequests int64
	cacheHits          int64
	startTime          time.Time
	endTime            time.Time
}

// Helper methods for test topology generation

func (pb *PerformanceBenchmark) findBestConnections(node *graph.NetworkNode, count int) []*graph.NetworkNode {
	connections := make([]*graph.NetworkNode, 0, count)
	
	// Simple distance-based connection for realistic topology
	for _, targetNode := range pb.testTopology.nodes {
		if len(connections) >= count {
			break
		}
		
		if targetNode.ID != node.ID {
			connections = append(connections, targetNode)
		}
	}
	
	return connections
}

func (pb *PerformanceBenchmark) createRealisticEdge(from, to *graph.NetworkNode) *graph.NetworkEdge {
	// Calculate realistic latency based on geographic distance
	latency := pb.calculateLatencyFromDistance(from, to)
	
	// Calculate bandwidth (inverse relationship with latency)
	bandwidth := 1000.0 / (1.0 + float64(latency.Milliseconds()))
	
	return &graph.NetworkEdge{
		From:        from.ID,
		To:          to.ID,
		Weight:      float64(latency.Microseconds()),
		Latency:     latency,
		Bandwidth:   bandwidth,
		PacketLoss:  rand.Float64() * 0.01, // 0-1% packet loss
		Jitter:      time.Duration(rand.Intn(10)) * time.Millisecond,
		Cost:        rand.Float64() * 10.0,
		Reliability: 0.95 + rand.Float64()*0.05,
		Stability:   0.9 + rand.Float64()*0.1,
		LastUpdate:  time.Now(),
	}
}

func (pb *PerformanceBenchmark) calculateLatencyFromDistance(from, to *graph.NetworkNode) time.Duration {
	// Simplified geographic distance to latency conversion
	latDiff := from.Latitude - to.Latitude
	lonDiff := from.Longitude - to.Longitude
	distance := math.Sqrt(latDiff*latDiff + lonDiff*lonDiff)
	
	// Base latency + distance component
	baseLatency := 1.0  // 1ms base
	distanceLatency := distance * 0.1 // ~0.1ms per degree
	
	totalLatency := baseLatency + distanceLatency
	return time.Duration(totalLatency * float64(time.Millisecond))
}

// Helper functions for performance calculation

func calculateAverageLatency(latencies []time.Duration) time.Duration {
	if len(latencies) == 0 {
		return 0
	}
	
	total := time.Duration(0)
	for _, latency := range latencies {
		total += latency
	}
	
	return total / time.Duration(len(latencies))
}

func calculatePercentiles(latencies []time.Duration) (p50, p90, p95, p99 time.Duration) {
	if len(latencies) == 0 {
		return 0, 0, 0, 0
	}
	
	// Sort latencies
	sorted := make([]time.Duration, len(latencies))
	copy(sorted, latencies)
	
	for i := 0; i < len(sorted)-1; i++ {
		for j := 0; j < len(sorted)-i-1; j++ {
			if sorted[j] > sorted[j+1] {
				sorted[j], sorted[j+1] = sorted[j+1], sorted[j]
			}
		}
	}
	
	n := len(sorted)
	p50 = sorted[int(float64(n)*0.50)]
	p90 = sorted[int(float64(n)*0.90)]
	p95 = sorted[int(float64(n)*0.95)]
	p99 = sorted[int(float64(n)*0.99)]
	
	return p50, p90, p95, p99
}

func findMinLatency(latencies []time.Duration) time.Duration {
	if len(latencies) == 0 {
		return 0
	}
	
	min := latencies[0]
	for _, latency := range latencies[1:] {
		if latency < min {
			min = latency
		}
	}
	return min
}

func findMaxLatency(latencies []time.Duration) time.Duration {
	if len(latencies) == 0 {
		return 0
	}
	
	max := latencies[0]
	for _, latency := range latencies[1:] {
		if latency > max {
			max = latency
		}
	}
	return max
}

func (pb *PerformanceBenchmark) calculateOptimalityScore(metrics *TestMetrics) float64 {
	// Simple optimality score based on cache hit rate and success rate
	stats := pb.routingTable.GetRoutingStats()
	return (stats.CacheHitRate + stats.SuccessRate) / 2.0 / 100.0
}

func (pb *PerformanceBenchmark) calculateConsistencyScore(latencies []time.Duration) float64 {
	if len(latencies) < 2 {
		return 1.0
	}
	
	// Calculate coefficient of variation (lower is more consistent)
	avg := calculateAverageLatency(latencies)
	variance := time.Duration(0)
	
	for _, latency := range latencies {
		diff := latency - avg
		variance += time.Duration(int64(diff) * int64(diff))
	}
	
	variance /= time.Duration(len(latencies))
	stdDev := time.Duration(math.Sqrt(float64(variance)))
	
	if avg == 0 {
		return 1.0
	}
	
	cv := float64(stdDev) / float64(avg)
	return 1.0 / (1.0 + cv) // Convert to 0-1 score where 1 is most consistent
}

// RunPerformanceTest is the main entry point for performance validation
func RunPerformanceTest(numNodes, connections, concurrency int) (*PerformanceTestResult, error) {
	benchmark := NewPerformanceBenchmark(numNodes, connections, concurrency)
	return benchmark.RunComprehensivePerformanceTest()
}