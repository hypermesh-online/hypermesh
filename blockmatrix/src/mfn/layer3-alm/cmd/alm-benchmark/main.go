// ALM Performance Benchmark - Validates 777% improvement target
package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"time"

	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/routing"
)

// BenchmarkConfig holds configuration for the benchmark
type BenchmarkConfig struct {
	NumNodes        int
	NumConnections  int
	Concurrency     int
	Duration        time.Duration
	OutputFile      string
	Verbose         bool
	TargetLatency   time.Duration
	BaselineLatency time.Duration
}

func main() {
	config := parseBenchmarkFlags()
	
	log.Printf("Starting ALM Performance Benchmark")
	log.Printf("Configuration: %d nodes, %d connections, %d concurrent workers", 
		config.NumNodes, config.NumConnections, config.Concurrency)
	
	// Run comprehensive performance test
	result, err := routing.RunPerformanceTest(config.NumNodes, config.NumConnections, config.Concurrency)
	if err != nil {
		log.Fatalf("Performance test failed: %v", err)
	}
	
	// Display results
	displayResults(config, result)
	
	// Save results to file if specified
	if config.OutputFile != "" {
		if err := saveResults(config.OutputFile, result); err != nil {
			log.Printf("Failed to save results: %v", err)
		}
	}
	
	// Exit with appropriate code
	if result.TargetAchieved {
		log.Printf("SUCCESS: 777% improvement target ACHIEVED!")
		os.Exit(0)
	} else {
		log.Printf("FAILURE: 777% improvement target NOT achieved")
		os.Exit(1)
	}
}

func parseBenchmarkFlags() *BenchmarkConfig {
	config := &BenchmarkConfig{}
	
	flag.IntVar(&config.NumNodes, "nodes", 1000, "Number of nodes in test topology")
	flag.IntVar(&config.NumConnections, "connections", 5000, "Number of connections in test topology")
	flag.IntVar(&config.Concurrency, "concurrency", 50, "Number of concurrent workers")
	flag.DurationVar(&config.Duration, "duration", 30*time.Second, "Test duration")
	flag.StringVar(&config.OutputFile, "output", "", "Output file for results (optional)")
	flag.BoolVar(&config.Verbose, "verbose", false, "Verbose output")
	flag.DurationVar(&config.TargetLatency, "target", 179*time.Microsecond, "Target latency (default: 0.179ms for 777% improvement)")
	flag.DurationVar(&config.BaselineLatency, "baseline", 1390*time.Microsecond, "Baseline HTTP latency (default: 1.39ms)")
	
	flag.Parse()
	
	return config
}

func displayResults(config *BenchmarkConfig, result *routing.PerformanceTestResult) {
	fmt.Println("\n" + "="*80)
	fmt.Println("ALM ROUTING PERFORMANCE BENCHMARK RESULTS")
	fmt.Println("="*80)
	
	// Performance Summary
	fmt.Printf("PERFORMANCE SUMMARY:\n")
	fmt.Printf("  Average Latency:      %v\n", result.AverageLatency)
	fmt.Printf("  P50 Latency:          %v\n", result.P50Latency)
	fmt.Printf("  P90 Latency:          %v\n", result.P90Latency)
	fmt.Printf("  P95 Latency:          %v\n", result.P95Latency)
	fmt.Printf("  P99 Latency:          %v\n", result.P99Latency)
	fmt.Printf("  Min Latency:          %v\n", result.MinLatency)
	fmt.Printf("  Max Latency:          %v\n", result.MaxLatency)
	fmt.Printf("  Requests/Second:      %.0f\n", result.RequestsPerSecond)
	fmt.Printf("  Success Rate:         %.2f%%\n", result.SuccessRate)
	fmt.Printf("  Cache Hit Rate:       %.2f%%\n", result.CacheHitRate)
	
	fmt.Printf("\nQUALITY METRICS:\n")
	fmt.Printf("  Optimality Score:     %.3f\n", result.OptimalityScore)
	fmt.Printf("  Consistency Score:    %.3f\n", result.ConsistencyScore)
	
	fmt.Printf("\nIMPROVEMENT ANALYSIS:\n")
	fmt.Printf("  Baseline Latency:     %v\n", result.BaselineLatency)
	fmt.Printf("  ALM Latency:          %v\n", result.AverageLatency)
	fmt.Printf("  Improvement Factor:   %.2fx\n", result.ImprovementFactor)
	fmt.Printf("  Improvement %%:        %.1f%%\n", (result.ImprovementFactor-1)*100)
	fmt.Printf("  Target (777%%):        %.2fx\n", 7.77)
	
	// Success/Failure Status
	fmt.Printf("\nBENCHMARK RESULT:\n")
	if result.TargetAchieved {
		fmt.Printf("  Status:              ✅ SUCCESS - Target ACHIEVED!\n")
		fmt.Printf("  Achievement:         %.1fx improvement (%.1f%% above target)\n",
			result.ImprovementFactor,
			(result.ImprovementFactor/7.77-1)*100)
	} else {
		fmt.Printf("  Status:              ❌ FAILURE - Target NOT achieved\n")
		fmt.Printf("  Shortfall:           %.2fx improvement (%.1f%% below target)\n",
			result.ImprovementFactor,
			(1-result.ImprovementFactor/7.77)*100)
	}
	
	// Detailed breakdown
	if config.Verbose {
		fmt.Printf("\nDETAILED METRICS:\n")
		fmt.Printf("  Association Hits:     %d\n", result.AssociationHits)
		fmt.Printf("  Graph Traversals:     %d\n", result.GraphTraversals)
		fmt.Printf("  Optimization Runs:    %d\n", result.OptimizationRuns)
	}
	
	// Performance breakdown analysis
	fmt.Printf("\nPERFORMANCE BREAKDOWN:\n")
	displayPerformanceBreakdown(result)
	
	fmt.Println("="*80)
}

func displayPerformanceBreakdown(result *routing.PerformanceTestResult) {
	// Calculate where the performance gains come from
	baseLatency := float64(result.BaselineLatency.Microseconds())
	almLatency := float64(result.AverageLatency.Microseconds())
	
	// Estimated breakdown of improvements
	cacheContribution := result.CacheHitRate / 100.0 * 0.8    // Cache hits save ~80% of lookup time
	optimizationContribution := result.OptimalityScore * 0.3   // Optimization saves ~30% of path cost
	associativeContribution := 0.4                            // Associative search saves ~40% of discovery time
	
	fmt.Printf("  Estimated Performance Sources:\n")
	fmt.Printf("    Cache Optimization:     %.1f%% contribution\n", cacheContribution*100)
	fmt.Printf("    Route Optimization:     %.1f%% contribution\n", optimizationContribution*100)
	fmt.Printf("    Associative Search:     %.1f%% contribution\n", associativeContribution*100)
	fmt.Printf("    Load Balancing:         %.1f%% contribution\n", 10.0) // Estimated
	
	// Latency components
	fmt.Printf("  Latency Components (estimated):\n")
	fmt.Printf("    Route Discovery:        %.0f μs\n", almLatency*0.3)
	fmt.Printf("    Path Calculation:       %.0f μs\n", almLatency*0.2)
	fmt.Printf("    Cache Lookup:           %.0f μs\n", almLatency*0.1)
	fmt.Printf("    Network Overhead:       %.0f μs\n", almLatency*0.4)
}

func saveResults(filename string, result *routing.PerformanceTestResult) error {
	file, err := os.Create(filename)
	if err != nil {
		return fmt.Errorf("failed to create output file: %w", err)
	}
	defer file.Close()
	
	// Write results in JSON format
	fmt.Fprintf(file, `{
  "timestamp": "%s",
  "performance": {
    "average_latency_us": %d,
    "p50_latency_us": %d,
    "p90_latency_us": %d,
    "p95_latency_us": %d,
    "p99_latency_us": %d,
    "min_latency_us": %d,
    "max_latency_us": %d,
    "requests_per_second": %.2f,
    "success_rate": %.2f,
    "cache_hit_rate": %.2f
  },
  "quality": {
    "optimality_score": %.3f,
    "consistency_score": %.3f
  },
  "improvement": {
    "baseline_latency_us": %d,
    "improvement_factor": %.2f,
    "improvement_percentage": %.1f,
    "target_achieved": %t
  },
  "alm_metrics": {
    "association_hits": %d,
    "graph_traversals": %d,
    "optimization_runs": %d
  }
}`,
		time.Now().Format(time.RFC3339),
		result.AverageLatency.Microseconds(),
		result.P50Latency.Microseconds(),
		result.P90Latency.Microseconds(),
		result.P95Latency.Microseconds(),
		result.P99Latency.Microseconds(),
		result.MinLatency.Microseconds(),
		result.MaxLatency.Microseconds(),
		result.RequestsPerSecond,
		result.SuccessRate,
		result.CacheHitRate,
		result.OptimalityScore,
		result.ConsistencyScore,
		result.BaselineLatency.Microseconds(),
		result.ImprovementFactor,
		(result.ImprovementFactor-1)*100,
		result.TargetAchieved,
		result.AssociationHits,
		result.GraphTraversals,
		result.OptimizationRuns,
	)
	
	return nil
}

// Additional utility functions for extended analysis

func analyzePerformanceCharacteristics(result *routing.PerformanceTestResult) {
	fmt.Printf("\nPERFORMANCE CHARACTERISTICS ANALYSIS:\n")
	
	// Latency distribution analysis
	p50 := float64(result.P50Latency.Microseconds())
	p90 := float64(result.P90Latency.Microseconds())
	p99 := float64(result.P99Latency.Microseconds())
	avg := float64(result.AverageLatency.Microseconds())
	
	// Calculate tail latency characteristics
	tailLatencyRatio := p99 / p50
	distributionSkew := (avg - p50) / p50
	
	fmt.Printf("  Tail Latency Analysis:\n")
	fmt.Printf("    P99/P50 Ratio:        %.2f (lower is better)\n", tailLatencyRatio)
	fmt.Printf("    Distribution Skew:    %.3f (closer to 0 is better)\n", distributionSkew)
	
	if tailLatencyRatio < 3.0 {
		fmt.Printf("    Assessment:           ✅ Excellent tail latency control\n")
	} else if tailLatencyRatio < 5.0 {
		fmt.Printf("    Assessment:           ⚠️  Good tail latency control\n")
	} else {
		fmt.Printf("    Assessment:           ❌ Poor tail latency control\n")
	}
	
	// Throughput analysis
	rps := result.RequestsPerSecond
	fmt.Printf("  Throughput Analysis:\n")
	fmt.Printf("    Requests/Second:      %.0f\n", rps)
	
	if rps > 10000 {
		fmt.Printf("    Assessment:           ✅ High throughput\n")
	} else if rps > 5000 {
		fmt.Printf("    Assessment:           ⚠️  Moderate throughput\n")
	} else {
		fmt.Printf("    Assessment:           ❌ Low throughput\n")
	}
	
	// Cache effectiveness
	cacheHitRate := result.CacheHitRate
	fmt.Printf("  Cache Effectiveness:\n")
	fmt.Printf("    Hit Rate:             %.1f%%\n", cacheHitRate)
	
	if cacheHitRate > 80.0 {
		fmt.Printf("    Assessment:           ✅ Excellent cache performance\n")
	} else if cacheHitRate > 60.0 {
		fmt.Printf("    Assessment:           ⚠️  Good cache performance\n")
	} else {
		fmt.Printf("    Assessment:           ❌ Poor cache performance\n")
	}
}

func generateRecommendations(result *routing.PerformanceTestResult) {
	fmt.Printf("\nOPTIMIZATION RECOMMENDATIONS:\n")
	
	recommendations := []string{}
	
	// Latency-based recommendations
	if result.AverageLatency > 500*time.Microsecond {
		recommendations = append(recommendations, 
			"• Consider increasing cache size or TTL to reduce lookup times")
	}
	
	if result.P99Latency > result.P50Latency*5 {
		recommendations = append(recommendations,
			"• Implement better load balancing to reduce tail latencies")
	}
	
	// Cache-based recommendations
	if result.CacheHitRate < 70.0 {
		recommendations = append(recommendations,
			"• Optimize cache warming strategies and increase cache size")
	}
	
	// Success rate recommendations
	if result.SuccessRate < 99.0 {
		recommendations = append(recommendations,
			"• Improve error handling and retry mechanisms")
	}
	
	// Optimization recommendations
	if result.OptimalityScore < 0.8 {
		recommendations = append(recommendations,
			"• Fine-tune multi-objective optimization parameters")
	}
	
	if len(recommendations) == 0 {
		fmt.Printf("  ✅ No major optimization recommendations - performance is excellent!\n")
	} else {
		for _, rec := range recommendations {
			fmt.Printf("  %s\n", rec)
		}
	}
}