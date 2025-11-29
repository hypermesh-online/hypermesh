/*!
# MFN Performance Benchmarking Framework

Comprehensive benchmarking suite for validating MFN (Multi-layer Flow Network) performance 
improvements across all four layers and integrated HyperMesh functionality.

## Architecture

The framework provides:
- **Layer-specific benchmarks** for IFR, DSR, ALM, and CPE
- **Integration benchmarks** for end-to-end HyperMesh performance
- **Baseline measurements** without MFN optimizations
- **Statistical analysis** with significance testing
- **Regression detection** and automated validation
- **Performance visualization** and reporting

## Performance Targets

1. **Layer 1 (IFR)**: <0.1ms exact matching, 88.6% latency improvement
2. **Layer 2 (DSR)**: <1ms neural similarity detection  
3. **Layer 3 (ALM)**: 777% routing improvement over HTTP baseline
4. **Layer 4 (CPE)**: <2ms context prediction
5. **Overall**: adaptive network tiers throughput with <5% MFN overhead

## Usage

```rust
use mfn_benchmarks::*;

// Run layer-specific benchmarks
let layer1_results = layer1::run_ifr_benchmarks(config)?;
let layer2_results = layer2::run_dsr_benchmarks(config)?;

// Run integration benchmarks
let integration_results = integration::run_hypermesh_benchmarks(config)?;

// Generate performance reports
reporting::generate_comprehensive_report(&results)?;
```
*/

pub mod common;
pub mod layer1;
pub mod layer2; 
pub mod layer3;
pub mod layer4;
pub mod integration;
pub mod baseline;
pub mod analysis;
pub mod reporting;
pub mod regression;
pub mod memory;
pub mod network;
pub mod dashboard;

pub use common::*;

/// Re-export key types and functions for convenience
pub use analysis::{StatisticalAnalysis, PerformanceComparison, RegressionDetection};
pub use baseline::{BaselineGenerator, HyperMeshBaseline};
pub use integration::{EndToEndBenchmark, HyperMeshIntegration};
pub use regression::{RegressionTest, PerformanceRegression};
pub use reporting::{PerformanceReport, BenchmarkVisualization};

/// MFN Benchmarking Framework version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default benchmark configuration
pub const DEFAULT_CONFIG: BenchmarkConfig = BenchmarkConfig {
    warmup_iterations: 1000,
    measurement_iterations: 10000,
    statistical_confidence: 0.95,
    regression_threshold: 0.05, // 5% performance degradation threshold
    memory_limit_mb: 512,
    timeout_seconds: 300,
    parallel_workers: num_cpus::get(),
    output_format: OutputFormat::Json,
    enable_flamegraph: false,
    enable_perf_counters: true,
};