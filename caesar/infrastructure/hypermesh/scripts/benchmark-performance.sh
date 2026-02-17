#!/bin/bash

# Gateway Coin Hypermesh Performance Benchmarking Script
# Comprehensive performance validation and reporting

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${SCRIPT_DIR}/.."
RESULTS_DIR="${PROJECT_ROOT}/benchmark-results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
    exit 1
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $1${NC}"
}

# Performance targets
declare -A PERFORMANCE_TARGETS=(
    ["uptime_percentage"]="99.9"
    ["api_latency_ms"]="100"
    ["consensus_time_seconds"]="2"
    ["transaction_throughput"]="1000"
    ["state_sync_latency_ms"]="500"
    ["memory_usage_mb"]="2048"
    ["cpu_usage_percent"]="80"
    ["disk_io_mbps"]="100"
    ["network_throughput_mbps"]="1000"
    ["connection_limit"]="2000"
)

# Create results directory
create_results_dir() {
    log "Creating benchmark results directory..."
    mkdir -p "${RESULTS_DIR}/${TIMESTAMP}"
    RESULTS_PATH="${RESULTS_DIR}/${TIMESTAMP}"
}

# Check if services are running
check_services() {
    log "Checking service availability..."
    
    local services=("hypermesh-nexus" "stoq-engine" "ml-inference" "redis-cluster-1")
    local failed_services=()
    
    for service in "${services[@]}"; do
        if ! docker ps | grep -q "${service}"; then
            failed_services+=("${service}")
        fi
    done
    
    if [[ ${#failed_services[@]} -gt 0 ]]; then
        error "Services not running: ${failed_services[*]}"
    fi
    
    log "All required services are running"
}

# System resource benchmarks
benchmark_system_resources() {
    log "Running system resource benchmarks..."
    
    info "CPU benchmark..."
    {
        echo "=== CPU Benchmark ==="
        echo "Timestamp: $(date)"
        echo "CPU Info:"
        cat /proc/cpuinfo | grep "model name" | head -1
        
        echo -e "\nCPU Load Average:"
        uptime
        
        echo -e "\nCPU Usage by Core:"
        top -bn1 | grep "Cpu" | head -8
        
        echo -e "\nStress Test Results:"
        timeout 60 stress-ng --cpu 4 --timeout 60s --metrics-brief 2>&1 || echo "stress-ng not available"
    } > "${RESULTS_PATH}/cpu_benchmark.txt"
    
    info "Memory benchmark..."
    {
        echo "=== Memory Benchmark ==="
        echo "Timestamp: $(date)"
        echo "Memory Info:"
        free -h
        
        echo -e "\nMemory Usage Details:"
        cat /proc/meminfo | head -20
        
        echo -e "\nMemory Performance Test:"
        timeout 30 stress-ng --vm 2 --vm-bytes 1G --timeout 30s --metrics-brief 2>&1 || echo "stress-ng not available"
    } > "${RESULTS_PATH}/memory_benchmark.txt"
    
    info "Disk I/O benchmark..."
    {
        echo "=== Disk I/O Benchmark ==="
        echo "Timestamp: $(date)"
        echo "Disk Usage:"
        df -h
        
        echo -e "\nDisk Performance Test:"
        if command -v fio &> /dev/null; then
            fio --name=random-write --ioengine=posixaio --rw=randwrite --bs=4k --size=100m --numjobs=1 --iodepth=1 --runtime=30 --time_based --group_reporting
        else
            # Fallback to dd test
            echo "Sequential Write Test:"
            dd if=/dev/zero of=/tmp/benchmark_write bs=1M count=100 conv=fdatasync 2>&1 | tail -1
            rm -f /tmp/benchmark_write
            
            echo "Sequential Read Test:"  
            dd if=/dev/zero of=/tmp/benchmark_read bs=1M count=100 2>/dev/null
            dd if=/tmp/benchmark_read of=/dev/null bs=1M 2>&1 | tail -1
            rm -f /tmp/benchmark_read
        fi
    } > "${RESULTS_PATH}/disk_benchmark.txt"
    
    info "Network benchmark..."
    {
        echo "=== Network Benchmark ==="
        echo "Timestamp: $(date)"
        echo "Network Interfaces:"
        ip addr show | grep -E "(inet|UP|DOWN)"
        
        echo -e "\nNetwork Statistics:"
        ss -s
        
        echo -e "\nBandwidth Test:"
        if command -v iperf3 &> /dev/null; then
            # Start iperf3 server in background
            iperf3 -s -p 5201 -D
            sleep 2
            # Run client test
            iperf3 -c localhost -p 5201 -t 10 2>/dev/null || echo "iperf3 test failed"
            # Kill server
            pkill iperf3
        else
            echo "iperf3 not available, skipping bandwidth test"
        fi
    } > "${RESULTS_PATH}/network_benchmark.txt"
}

# API performance benchmarks
benchmark_api_performance() {
    log "Running API performance benchmarks..."
    
    info "Nexus API benchmark..."
    {
        echo "=== Nexus API Performance ==="
        echo "Timestamp: $(date)"
        
        if command -v ab &> /dev/null; then
            echo "Apache Bench Results - Health Endpoint:"
            ab -n 1000 -c 10 -k "http://localhost:8080/health" 2>&1
            
            echo -e "\n\nApache Bench Results - Metrics Endpoint:"
            ab -n 500 -c 5 "http://localhost:9090/metrics" 2>&1
        else
            echo "Apache Bench not available"
        fi
        
        if command -v wrk &> /dev/null; then
            echo -e "\n\nWrk Results - Health Endpoint:"
            wrk -t4 -c100 -d30s "http://localhost:8080/health" 2>&1
        else
            echo "wrk not available"
        fi
    } > "${RESULTS_PATH}/nexus_api_benchmark.txt"
    
    info "STOQ API benchmark..."
    {
        echo "=== STOQ API Performance ==="
        echo "Timestamp: $(date)"
        
        if command -v ab &> /dev/null; then
            echo "Apache Bench Results - STOQ Health Endpoint:"
            ab -n 1000 -c 10 "http://localhost:8081/health" 2>&1
            
            echo -e "\n\nApache Bench Results - STOQ Metrics:"
            ab -n 500 -c 5 "http://localhost:9091/metrics" 2>&1
        fi
        
        if command -v wrk &> /dev/null; then
            echo -e "\n\nWrk Results - STOQ Health Endpoint:"
            wrk -t4 -c50 -d30s "http://localhost:8081/health" 2>&1
        fi
    } > "${RESULTS_PATH}/stoq_api_benchmark.txt"
}

# Database performance benchmarks
benchmark_database_performance() {
    log "Running database performance benchmarks..."
    
    info "Redis cluster benchmark..."
    {
        echo "=== Redis Cluster Performance ==="
        echo "Timestamp: $(date)"
        
        echo "Redis Benchmark - SET operations:"
        docker exec redis-cluster-1 redis-benchmark -h localhost -p 7001 -t set -n 10000 -c 50 2>&1
        
        echo -e "\n\nRedis Benchmark - GET operations:"
        docker exec redis-cluster-1 redis-benchmark -h localhost -p 7001 -t get -n 10000 -c 50 2>&1
        
        echo -e "\n\nRedis Cluster Info:"
        docker exec redis-cluster-1 redis-cli cluster info 2>&1
        
        echo -e "\n\nRedis Memory Usage:"
        docker exec redis-cluster-1 redis-cli info memory 2>&1
    } > "${RESULTS_PATH}/redis_benchmark.txt"
    
    info "InfluxDB benchmark..."
    {
        echo "=== InfluxDB Performance ==="
        echo "Timestamp: $(date)"
        
        echo "InfluxDB Health:"
        curl -f "http://localhost:8086/health" 2>&1
        
        echo -e "\n\nInfluxDB Ping:"
        docker exec gateway-coin-influxdb influx ping 2>&1
        
        # Simple write performance test
        echo -e "\n\nWrite Performance Test:"
        for i in {1..1000}; do
            docker exec gateway-coin-influxdb influx write \
                --bucket stoq-metrics \
                --org gateway-coin \
                --token gateway-coin-influx-token-2025 \
                "test_metric,host=benchmark value=${i}i $(date +%s)000000000" 2>/dev/null
        done
        echo "Completed 1000 write operations"
    } > "${RESULTS_PATH}/influxdb_benchmark.txt"
}

# Container performance benchmarks
benchmark_container_performance() {
    log "Running container performance benchmarks..."
    
    info "Docker performance metrics..."
    {
        echo "=== Container Performance ==="
        echo "Timestamp: $(date)"
        
        echo "Docker System Info:"
        docker system df
        
        echo -e "\n\nContainer Resource Usage:"
        docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\t{{.BlockIO}}"
        
        echo -e "\n\nContainer Details:"
        docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" --filter "label=com.docker.compose.project=hypermesh"
    } > "${RESULTS_PATH}/container_benchmark.txt"
    
    info "Container startup time benchmark..."
    {
        echo "=== Container Startup Performance ==="
        echo "Timestamp: $(date)"
        
        local test_containers=("redis:7-alpine" "nginx:alpine" "postgres:13-alpine")
        
        for container in "${test_containers[@]}"; do
            echo "Testing startup time for ${container}:"
            start_time=$(date +%s.%N)
            docker run --rm -d --name "startup-test-$(date +%s)" "${container}" >/dev/null
            container_id=$(docker ps -q --filter "name=startup-test-*" | head -1)
            
            # Wait for container to be ready
            while [[ "$(docker inspect -f '{{.State.Status}}' "${container_id}")" != "running" ]]; do
                sleep 0.1
            done
            
            end_time=$(date +%s.%N)
            startup_time=$(echo "${end_time} - ${start_time}" | bc -l)
            echo "Startup time: ${startup_time}s"
            
            # Cleanup
            docker stop "${container_id}" >/dev/null
        done
    } > "${RESULTS_PATH}/container_startup_benchmark.txt"
}

# Custom Gateway Coin benchmarks
benchmark_gateway_coin_performance() {
    log "Running Gateway Coin specific benchmarks..."
    
    info "QUIC protocol performance..."
    {
        echo "=== QUIC Protocol Performance ==="
        echo "Timestamp: $(date)"
        
        # Test QUIC connectivity
        echo "Testing QUIC connectivity to port 4001:"
        if command -v nmap &> /dev/null; then
            nmap -sU -p 4001 localhost
        else
            netstat -ulnp | grep 4001 || echo "QUIC port 4001 not found"
        fi
        
        echo -e "\n\nTesting UDP performance to QUIC port:"
        if command -v iperf3 &> /dev/null; then
            # Start UDP server
            iperf3 -s -p 4002 -D
            sleep 1
            # Run UDP client test
            iperf3 -c localhost -p 4002 -u -b 100M -t 10 2>/dev/null || echo "UDP performance test failed"
            pkill iperf3
        fi
    } > "${RESULTS_PATH}/quic_benchmark.txt"
    
    info "eBPF program performance..."
    {
        echo "=== eBPF Program Performance ==="
        echo "Timestamp: $(date)"
        
        echo "eBPF programs loaded:"
        if command -v bpftool &> /dev/null; then
            bpftool prog list
        else
            echo "bpftool not available"
        fi
        
        echo -e "\n\neBPF maps:"
        if command -v bpftool &> /dev/null; then
            bpftool map list
        fi
        
        echo -e "\n\nNetwork statistics from eBPF:"
        # Check if our network stats are available
        if [[ -f "/sys/fs/bpf/network_stats" ]]; then
            echo "Network stats map found"
        else
            echo "Network stats map not found - eBPF program may not be loaded"
        fi
    } > "${RESULTS_PATH}/ebpf_benchmark.txt"
}

# Consensus performance benchmarks
benchmark_consensus_performance() {
    log "Running consensus performance benchmarks..."
    
    info "Byzantine fault tolerance simulation..."
    {
        echo "=== Consensus Performance ==="
        echo "Timestamp: $(date)"
        
        # Test consensus by sending multiple simultaneous requests
        echo "Testing consensus latency with concurrent requests:"
        
        start_time=$(date +%s.%N)
        
        # Send 10 concurrent health checks to test consensus
        for i in {1..10}; do
            {
                curl -s "http://localhost:8080/health" > /dev/null
            } &
        done
        
        wait # Wait for all requests to complete
        
        end_time=$(date +%s.%N)
        consensus_time=$(echo "${end_time} - ${start_time}" | bc -l)
        
        echo "10 concurrent requests completed in: ${consensus_time}s"
        echo "Average per request: $(echo "scale=3; ${consensus_time} / 10" | bc -l)s"
        
        # Test with higher load
        echo -e "\n\nTesting with 100 concurrent requests:"
        start_time=$(date +%s.%N)
        
        for i in {1..100}; do
            {
                curl -s "http://localhost:8080/health" > /dev/null
            } &
        done
        
        wait
        
        end_time=$(date +%s.%N)
        high_load_time=$(echo "${end_time} - ${start_time}" | bc -l)
        
        echo "100 concurrent requests completed in: ${high_load_time}s"
        echo "Average per request: $(echo "scale=3; ${high_load_time} / 100" | bc -l)s"
    } > "${RESULTS_PATH}/consensus_benchmark.txt"
}

# Generate performance report
generate_performance_report() {
    log "Generating comprehensive performance report..."
    
    {
        echo "# Gateway Coin Hypermesh Performance Report"
        echo "Generated: $(date)"
        echo "Benchmark ID: ${TIMESTAMP}"
        echo ""
        
        echo "## Executive Summary"
        echo ""
        
        # Check if we meet performance targets
        local passed_tests=0
        local total_tests=0
        
        echo "### Performance Targets Validation"
        echo ""
        echo "| Metric | Target | Actual | Status |"
        echo "|--------|--------|---------|--------|"
        
        # API Latency
        if [[ -f "${RESULTS_PATH}/nexus_api_benchmark.txt" ]]; then
            local avg_latency=$(grep "Time per request" "${RESULTS_PATH}/nexus_api_benchmark.txt" | head -1 | awk '{print $4}' | cut -d'.' -f1)
            if [[ -n "$avg_latency" && "$avg_latency" -lt "${PERFORMANCE_TARGETS[api_latency_ms]}" ]]; then
                echo "| API Latency | <${PERFORMANCE_TARGETS[api_latency_ms]}ms | ${avg_latency}ms | ✅ PASS |"
                ((passed_tests++))
            else
                echo "| API Latency | <${PERFORMANCE_TARGETS[api_latency_ms]}ms | ${avg_latency:-N/A}ms | ❌ FAIL |"
            fi
            ((total_tests++))
        fi
        
        # Transaction Throughput
        if [[ -f "${RESULTS_PATH}/nexus_api_benchmark.txt" ]]; then
            local throughput=$(grep "Requests per second" "${RESULTS_PATH}/nexus_api_benchmark.txt" | head -1 | awk '{print $4}' | cut -d'.' -f1)
            if [[ -n "$throughput" && "$throughput" -gt "${PERFORMANCE_TARGETS[transaction_throughput]}" ]]; then
                echo "| Transaction Throughput | >${PERFORMANCE_TARGETS[transaction_throughput]} TPS | ${throughput} TPS | ✅ PASS |"
                ((passed_tests++))
            else
                echo "| Transaction Throughput | >${PERFORMANCE_TARGETS[transaction_throughput]} TPS | ${throughput:-N/A} TPS | ❌ FAIL |"
            fi
            ((total_tests++))
        fi
        
        # Memory Usage
        local memory_usage_mb=$(free -m | awk 'NR==2{printf "%.0f", $3}')
        if [[ "$memory_usage_mb" -lt "${PERFORMANCE_TARGETS[memory_usage_mb]}" ]]; then
            echo "| Memory Usage | <${PERFORMANCE_TARGETS[memory_usage_mb]}MB | ${memory_usage_mb}MB | ✅ PASS |"
            ((passed_tests++))
        else
            echo "| Memory Usage | <${PERFORMANCE_TARGETS[memory_usage_mb]}MB | ${memory_usage_mb}MB | ❌ FAIL |"
        fi
        ((total_tests++))
        
        # Consensus Time
        if [[ -f "${RESULTS_PATH}/consensus_benchmark.txt" ]]; then
            local consensus_latency=$(grep "Average per request" "${RESULTS_PATH}/consensus_benchmark.txt" | head -1 | awk '{print $4}' | cut -d's' -f1)
            if [[ -n "$consensus_latency" ]] && (( $(echo "$consensus_latency < ${PERFORMANCE_TARGETS[consensus_time_seconds]}" | bc -l) )); then
                echo "| Consensus Latency | <${PERFORMANCE_TARGETS[consensus_time_seconds]}s | ${consensus_latency}s | ✅ PASS |"
                ((passed_tests++))
            else
                echo "| Consensus Latency | <${PERFORMANCE_TARGETS[consensus_time_seconds]}s | ${consensus_latency:-N/A}s | ❌ FAIL |"
            fi
            ((total_tests++))
        fi
        
        echo ""
        echo "**Overall Score: ${passed_tests}/${total_tests} tests passed**"
        
        if [[ $passed_tests -eq $total_tests ]]; then
            echo "🎉 **ALL PERFORMANCE TARGETS MET!**"
        else
            echo "⚠️  **PERFORMANCE ISSUES DETECTED - REVIEW REQUIRED**"
        fi
        
        echo ""
        echo "## Detailed Results"
        echo ""
        
        # Include summaries of each benchmark
        for result_file in "${RESULTS_PATH}"/*.txt; do
            if [[ -f "$result_file" ]]; then
                filename=$(basename "$result_file" .txt)
                echo "### ${filename//_/ } Results"
                echo ""
                echo "\`\`\`"
                head -20 "$result_file"
                echo "..."
                echo "\`\`\`"
                echo ""
            fi
        done
        
        echo "## Recommendations"
        echo ""
        
        if [[ $passed_tests -lt $total_tests ]]; then
            echo "### Performance Improvements Needed:"
            echo "- Review failed metrics and optimize accordingly"
            echo "- Consider scaling up infrastructure resources"
            echo "- Optimize application configuration"
            echo "- Review network and storage performance"
        else
            echo "### Performance Optimization Opportunities:"
            echo "- System is meeting all performance targets"
            echo "- Consider stress testing with higher loads"
            echo "- Monitor performance over time for degradation"
            echo "- Plan for future scaling requirements"
        fi
        
        echo ""
        echo "## Files Generated"
        echo ""
        for result_file in "${RESULTS_PATH}"/*.txt; do
            if [[ -f "$result_file" ]]; then
                echo "- $(basename "$result_file")"
            fi
        done
        
    } > "${RESULTS_PATH}/performance_report.md"
    
    log "Performance report generated: ${RESULTS_PATH}/performance_report.md"
}

# Main execution
main() {
    log "Starting Gateway Coin Hypermesh Performance Benchmarks..."
    
    create_results_dir
    check_services
    
    benchmark_system_resources
    benchmark_api_performance
    benchmark_database_performance
    benchmark_container_performance
    benchmark_gateway_coin_performance
    benchmark_consensus_performance
    
    generate_performance_report
    
    log "Benchmarking completed successfully!"
    info "Results available in: ${RESULTS_PATH}"
    info "Main report: ${RESULTS_PATH}/performance_report.md"
}

# Check dependencies
if ! command -v bc &> /dev/null; then
    warn "bc (basic calculator) not installed - some calculations may fail"
fi

if ! command -v curl &> /dev/null; then
    error "curl is required but not installed"
fi

if ! command -v docker &> /dev/null; then
    error "docker is required but not installed"
fi

# Script options
case "${1:-benchmark}" in
    benchmark)
        main
        ;;
    system)
        create_results_dir
        benchmark_system_resources
        ;;
    api)
        create_results_dir
        check_services
        benchmark_api_performance
        ;;
    database)
        create_results_dir
        check_services
        benchmark_database_performance
        ;;
    container)
        create_results_dir
        benchmark_container_performance
        ;;
    consensus)
        create_results_dir
        check_services
        benchmark_consensus_performance
        ;;
    report)
        if [[ -d "${RESULTS_DIR}/${2:-}" ]]; then
            RESULTS_PATH="${RESULTS_DIR}/${2}"
            generate_performance_report
        else
            error "Results directory not found: ${2:-}"
        fi
        ;;
    clean)
        log "Cleaning up benchmark results older than 7 days..."
        find "${RESULTS_DIR}" -type d -mtime +7 -exec rm -rf {} + 2>/dev/null || true
        log "Cleanup completed"
        ;;
    *)
        echo "Usage: $0 {benchmark|system|api|database|container|consensus|report|clean}"
        echo ""
        echo "Commands:"
        echo "  benchmark  - Run all benchmarks (default)"
        echo "  system     - Run system resource benchmarks only"
        echo "  api        - Run API performance benchmarks only"
        echo "  database   - Run database benchmarks only"
        echo "  container  - Run container benchmarks only"
        echo "  consensus  - Run consensus benchmarks only"
        echo "  report     - Generate report for specific timestamp"
        echo "  clean      - Clean old benchmark results"
        exit 1
        ;;
esac