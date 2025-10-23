// Simple ALM Performance Benchmark - Validates 777% improvement target
package main

import (
	"context"
	"fmt"
	"log"
	"math/rand"
	"os"
	"sync"
	"time"

	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/associative"
	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/graph"
	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/optimization"
	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/routing"
)

const (
	// Performance targets
	BaselineLatency    = 1390 * time.Microsecond // HTTP baseline: 1.39ms
	TargetLatency      = 179 * time.Microsecond  // Target: 0.179ms (777% improvement)
	TargetImprovement  = 7.77                    // 777% improvement factor
	
	// Test configuration
	NumNodes           = 100
	NumConnections     = 500
	TestRequests       = 10000
	ConcurrentWorkers  = 20
)

type BenchmarkResult struct {
	AverageLatency    time.Duration
	P50Latency       time.Duration
	P90Latency       time.Duration
	P95Latency       time.Duration
	P99Latency       time.Duration
	RequestsPerSecond float64
	SuccessRate      float64
	CacheHitRate     float64
	ImprovementFactor float64
	TargetAchieved   bool
}

func main() {
	log.Printf("Starting ALM Performance Benchmark")
	log.Printf("Target: %v latency (%.2fx improvement over %v baseline)", 
		TargetLatency, TargetImprovement, BaselineLatency)
	
	// Create test network topology
	networkGraph := createTestTopology(NumNodes, NumConnections)
	
	// Initialize ALM components
	searchEngine := associative.NewAssociativeSearchEngine(networkGraph, nil)
	optimizer := optimization.NewMultiObjectiveOptimizer(createOptimizerConfig())
	routingTable := routing.NewRoutingTable(networkGraph, searchEngine, optimizer, createRoutingConfig())
	
	// Warm up the system
	warmupSystem(routingTable)
	
	// Run performance benchmark
	result := runBenchmark(routingTable)
	
	// Display results
	displayResults(result)
	
	// Exit with appropriate code
	if result.TargetAchieved {
		log.Printf("SUCCESS: 777% improvement target ACHIEVED!")
		os.Exit(0)
	} else {
		log.Printf("FAILURE: 777% improvement target NOT achieved")
		os.Exit(1)
	}
}

func createTestTopology(numNodes, numConnections int) *graph.NetworkGraph {
	networkGraph := graph.NewNetworkGraph()
	
	// Create nodes
	for i := 1; i <= numNodes; i++ {
		node := &graph.NetworkNode{
			ID:          int64(i),
			Address:     fmt.Sprintf("node-%d", i),
			Region:      fmt.Sprintf("region-%d", (i-1)%5+1),
			Latency:     time.Duration(5+rand.Intn(45)) * time.Millisecond,
			Throughput:  100.0 + rand.Float64()*900.0,
			Reliability: 0.95 + rand.Float64()*0.05,
			LoadFactor:  rand.Float64() * 0.5,
			LastSeen:    time.Now(),
			Services:    make(map[string]graph.ServiceInfo),
		}
		
		networkGraph.AddNode(node)
	}
	
	return networkGraph
}

func createRoutingConfig() *routing.RoutingConfig {
	return &routing.RoutingConfig{
		CacheSize:            10000,
		CacheTTL:            5 * time.Minute,
		InvalidationDelay:   10 * time.Millisecond,
		MaxAlternatives:     3,
		SearchTimeout:       100 * time.Millisecond,
		OptimizationLevel:   routing.DeepOptimization,
		LoadBalanceThreshold: 0.7,
		HealthCheckInterval: 30 * time.Second,
		MaxConcurrentLookups: ConcurrentWorkers * 2,
		StatisticsWindow:    5 * time.Minute,
	}
}

func createOptimizerConfig() *optimization.OptimizerConfig {
	return &optimization.OptimizerConfig{
		PopulationSize:       50,
		MaxGenerations:       20,
		CrossoverRate:        0.8,
		MutationRate:         0.1,
		LatencyWeight:        0.4,
		ThroughputWeight:     0.3,
		ReliabilityWeight:    0.2,
		CostWeight:          0.1,
		OptimizationTimeout: 50 * time.Millisecond,
		ConvergenceThreshold: 0.01,
		StagnationLimit:     3,
	}
}

func warmupSystem(routingTable *routing.RoutingTable) {
	log.Printf("Warming up system...")
	
	for i := 0; i < 1000; i++ {
		source := int64(1 + rand.Intn(NumNodes))
		dest := int64(1 + rand.Intn(NumNodes))
		
		if source != dest {
			request := routing.RoutingRequest{
				Source:      source,
				Destination: dest,
				ServiceType: "api",
				QoSClass:    routing.LowLatency,
				Context:     context.Background(),
			}
			
			routingTable.LookupRoute(request)
		}
	}
}

func runBenchmark(routingTable *routing.RoutingTable) *BenchmarkResult {
	log.Printf("Running performance benchmark...")
	
	var latencies []time.Duration
	var mutex sync.Mutex
	var wg sync.WaitGroup
	
	totalRequests := int64(0)
	successfulRequests := int64(0)
	cacheHits := int64(0)
	
	startTime := time.Now()
	
	// Run concurrent workers
	requestsPerWorker := TestRequests / ConcurrentWorkers
	
	for worker := 0; worker < ConcurrentWorkers; worker++ {
		wg.Add(1)
		
		go func() {
			defer wg.Done()
			
			workerLatencies := make([]time.Duration, 0, requestsPerWorker)
			
			for i := 0; i < requestsPerWorker; i++ {
				source := int64(1 + rand.Intn(NumNodes))
				dest := int64(1 + rand.Intn(NumNodes))
				
				if source != dest {
					request := routing.RoutingRequest{
						Source:      source,
						Destination: dest,
						ServiceType: "api",
						QoSClass:    routing.LowLatency,
						Constraints: routing.RouteConstraints{
							MaxLatency: 10 * time.Millisecond,
							MaxHops:    10,
						},
						Context: context.Background(),
					}
					
					requestStart := time.Now()
					response, err := routingTable.LookupRoute(request)
					latency := time.Since(requestStart)
					
					mutex.Lock()
					totalRequests++
					if err == nil && response != nil {
						successfulRequests++
						workerLatencies = append(workerLatencies, latency)
						
						if response.CacheHit {
							cacheHits++
						}
					}
					mutex.Unlock()
				}
			}
			
			// Merge worker latencies
			mutex.Lock()
			latencies = append(latencies, workerLatencies...)
			mutex.Unlock()
		}()
	}
	
	wg.Wait()
	endTime := time.Now()
	
	// Calculate metrics
	avgLatency := calculateAverageLatency(latencies)
	p50, p90, p95, p99 := calculatePercentiles(latencies)
	
	testDuration := endTime.Sub(startTime).Seconds()
	rps := float64(successfulRequests) / testDuration
	successRate := float64(successfulRequests) / float64(totalRequests) * 100.0
	cacheHitRate := float64(cacheHits) / float64(totalRequests) * 100.0
	
	improvementFactor := float64(BaselineLatency) / float64(avgLatency)
	targetAchieved := improvementFactor >= TargetImprovement
	
	return &BenchmarkResult{
		AverageLatency:    avgLatency,
		P50Latency:       p50,
		P90Latency:       p90,
		P95Latency:       p95,
		P99Latency:       p99,
		RequestsPerSecond: rps,
		SuccessRate:      successRate,
		CacheHitRate:     cacheHitRate,
		ImprovementFactor: improvementFactor,
		TargetAchieved:   targetAchieved,
	}
}

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
	
	// Simple sorting for percentiles
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

func displayResults(result *BenchmarkResult) {
	fmt.Println("\n" + "="*80)
	fmt.Println("ALM ROUTING PERFORMANCE BENCHMARK RESULTS")
	fmt.Println("="*80)
	
	fmt.Printf("PERFORMANCE SUMMARY:\n")
	fmt.Printf("  Average Latency:      %v\n", result.AverageLatency)
	fmt.Printf("  P50 Latency:          %v\n", result.P50Latency)
	fmt.Printf("  P90 Latency:          %v\n", result.P90Latency)
	fmt.Printf("  P95 Latency:          %v\n", result.P95Latency)
	fmt.Printf("  P99 Latency:          %v\n", result.P99Latency)
	fmt.Printf("  Requests/Second:      %.0f\n", result.RequestsPerSecond)
	fmt.Printf("  Success Rate:         %.2f%%\n", result.SuccessRate)
	fmt.Printf("  Cache Hit Rate:       %.2f%%\n", result.CacheHitRate)
	
	fmt.Printf("\nIMPROVEMENT ANALYSIS:\n")
	fmt.Printf("  Baseline Latency:     %v\n", BaselineLatency)
	fmt.Printf("  ALM Latency:          %v\n", result.AverageLatency)
	fmt.Printf("  Improvement Factor:   %.2fx\n", result.ImprovementFactor)
	fmt.Printf("  Improvement %%:        %.1f%%\n", (result.ImprovementFactor-1)*100)
	fmt.Printf("  Target (777%%):        %.2fx\n", TargetImprovement)
	
	fmt.Printf("\nBENCHMARK RESULT:\n")
	if result.TargetAchieved {
		fmt.Printf("  Status:              ✅ SUCCESS - Target ACHIEVED!\n")
		fmt.Printf("  Achievement:         %.1fx improvement (%.1f%% above target)\n",
			result.ImprovementFactor,
			(result.ImprovementFactor/TargetImprovement-1)*100)
	} else {
		fmt.Printf("  Status:              ❌ FAILURE - Target NOT achieved\n")
		fmt.Printf("  Shortfall:           %.2fx improvement (%.1f%% below target)\n",
			result.ImprovementFactor,
			(1-result.ImprovementFactor/TargetImprovement)*100)
	}
	
	fmt.Println("="*80)
}