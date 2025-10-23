#!/bin/bash

# HyperMesh Benchmark Runner - Phase 1: Performance Baseline
# This script attempts to run available benchmarks and collect metrics

echo "=========================================="
echo "HyperMesh Performance Baseline Collection"
echo "=========================================="
echo "Date: $(date)"
echo "System Info:"
uname -a
echo ""

RESULTS_DIR="benchmark_results_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"

echo "Results will be saved to: $RESULTS_DIR"
echo ""

# Function to run a benchmark if it compiles
run_benchmark() {
    local bench_name=$1
    local bench_path=$2

    echo "----------------------------------------"
    echo "Testing: $bench_name"
    echo "Path: $bench_path"

    if cargo test --manifest-path="$bench_path/Cargo.toml" --no-run 2>/dev/null; then
        echo "✓ Compilation successful"

        # Try to run benchmarks
        if cargo bench --manifest-path="$bench_path/Cargo.toml" 2>&1 | tee "$RESULTS_DIR/${bench_name}_results.txt"; then
            echo "✓ Benchmark completed"
        else
            echo "✗ Benchmark failed to run"
        fi
    else
        echo "✗ Compilation failed - skipping"
    fi
    echo ""
}

# Try MFN benchmarks
if [ -d "benchmarks/mfn" ]; then
    echo "Checking MFN Benchmarks..."
    run_benchmark "mfn" "benchmarks/mfn"
fi

# Try transport benchmarks
if [ -d "src/transport" ]; then
    echo "Checking Transport Benchmarks..."
    run_benchmark "transport" "src/transport"
fi

# Try consensus benchmarks
if [ -d "src/consensus" ]; then
    echo "Checking Consensus Benchmarks..."
    run_benchmark "consensus" "src/consensus"
fi

# Try integration benchmarks
if [ -d "src/integration" ]; then
    echo "Checking Integration Benchmarks..."
    run_benchmark "integration" "src/integration"
fi

# Summary
echo "=========================================="
echo "Benchmark Collection Summary"
echo "=========================================="
echo "Results saved in: $RESULTS_DIR"
echo ""

if ls "$RESULTS_DIR"/*.txt 2>/dev/null; then
    echo "Successfully collected:"
    ls -la "$RESULTS_DIR"/*.txt
else
    echo "No benchmarks could be run successfully."
    echo "Project requires compilation fixes before benchmarks can be established."
fi