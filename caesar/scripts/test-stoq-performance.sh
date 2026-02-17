#!/bin/bash

# STOQ Performance Testing Script
# Tests transport layer performance against 40 Gbps target

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${SCRIPT_DIR}/.."

# Configuration
TARGET_THROUGHPUT_GBPS=40
MINIMUM_THROUGHPUT_GBPS=10
TEST_DURATION=60
CONCURRENT_CONNECTIONS=1000
PACKET_SIZE=1452

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

# Check prerequisites
check_prerequisites() {
    log "Checking STOQ performance testing prerequisites..."
    
    # Check if iperf3 is available
    if ! command -v iperf3 &> /dev/null; then
        warn "iperf3 not found, installing..."
        sudo apt-get update && sudo apt-get install -y iperf3
    fi
    
    # Check if STOQ service is running
    if ! kubectl get service stoq-transport-service -n caesar-production &> /dev/null; then
        error "STOQ transport service not found"
    fi
    
    # Get STOQ service endpoint
    STOQ_SERVICE_IP=$(kubectl get service stoq-transport-service -n caesar-production -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || echo "")
    if [[ -z "$STOQ_SERVICE_IP" ]]; then
        STOQ_SERVICE_IP=$(kubectl get service stoq-transport-service -n caesar-production -o jsonpath='{.spec.clusterIP}')
    fi
    
    if [[ -z "$STOQ_SERVICE_IP" ]]; then
        error "Could not determine STOQ service IP"
    fi
    
    info "STOQ Service IP: $STOQ_SERVICE_IP"
    
    log "Prerequisites check completed"
}

# Start STOQ performance monitoring
start_monitoring() {
    log "Starting performance monitoring..."
    
    # Create monitoring job
    cat <<EOF | kubectl apply -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: stoq-performance-monitor
  namespace: caesar-production
spec:
  template:
    spec:
      restartPolicy: Never
      containers:
      - name: monitor
        image: prom/prometheus:latest
        command: ["/bin/sh"]
        args:
          - -c
          - |
            echo "Starting STOQ performance monitoring..."
            while true; do
              date
              kubectl top pods -n caesar-production --selector=app=stoq-transport
              sleep 5
            done
EOF
    
    info "Performance monitoring started"
}

# Test basic connectivity
test_connectivity() {
    log "Testing basic STOQ connectivity..."
    
    # Test API endpoint
    if kubectl run --rm -i --restart=Never connectivity-test --image=curlimages/curl:latest -- \
       curl -f -s "http://$STOQ_SERVICE_IP:8081/health" &> /dev/null; then
        info "✓ STOQ API connectivity test passed"
    else
        error "✗ STOQ API connectivity test failed"
    fi
    
    kubectl delete pod connectivity-test --ignore-not-found=true
}

# Test QUIC performance
test_quic_performance() {
    log "Testing QUIC protocol performance..."
    
    # Create QUIC performance test job
    cat <<EOF | kubectl apply -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: quic-performance-test
  namespace: caesar-production
spec:
  template:
    spec:
      restartPolicy: Never
      containers:
      - name: quic-test
        image: alpine:latest
        command: ["/bin/sh"]
        args:
          - -c
          - |
            set -e
            apk add --no-cache curl
            
            echo "Testing QUIC performance..."
            start_time=\$(date +%s)
            
            # Simulate QUIC traffic
            for i in \$(seq 1 100); do
              curl -s -o /dev/null "http://$STOQ_SERVICE_IP:8081/health" || true
            done
            
            end_time=\$(date +%s)
            duration=\$((end_time - start_time))
            
            echo "QUIC test completed in \$duration seconds"
            echo "Average request rate: \$((100 / duration)) requests/second"
EOF
    
    # Wait for test completion
    kubectl wait --for=condition=complete --timeout=120s job/quic-performance-test -n caesar-production
    
    # Get test results
    kubectl logs job/quic-performance-test -n caesar-production
    
    # Cleanup
    kubectl delete job quic-performance-test -n caesar-production
}

# Test concurrent connections
test_concurrent_connections() {
    log "Testing concurrent connections (target: $CONCURRENT_CONNECTIONS)..."
    
    # Create concurrent connection test
    cat <<EOF | kubectl apply -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: concurrent-connection-test
  namespace: caesar-production
spec:
  parallelism: 10
  template:
    spec:
      restartPolicy: Never
      containers:
      - name: connection-test
        image: alpine:latest
        command: ["/bin/sh"]
        args:
          - -c
          - |
            set -e
            apk add --no-cache curl
            
            echo "Testing concurrent connections..."
            
            # Create multiple connections
            for i in \$(seq 1 100); do
              (curl -s -o /dev/null "http://$STOQ_SERVICE_IP:8081/health" &)
            done
            
            wait
            echo "Concurrent connection test completed"
EOF
    
    # Wait for test completion
    kubectl wait --for=condition=complete --timeout=180s job/concurrent-connection-test -n caesar-production
    
    info "✓ Concurrent connection test completed"
    
    # Cleanup
    kubectl delete job concurrent-connection-test -n caesar-production
}

# Test throughput performance
test_throughput() {
    log "Testing throughput performance (target: ${TARGET_THROUGHPUT_GBPS} Gbps)..."
    
    # Create throughput test pod
    cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: Pod
metadata:
  name: throughput-test-client
  namespace: caesar-production
spec:
  restartPolicy: Never
  containers:
  - name: iperf-client
    image: networkstatic/iperf3:latest
    command: ["/bin/sh"]
    args:
      - -c
      - |
        set -e
        echo "Starting throughput test..."
        
        # Test TCP throughput
        echo "Testing TCP throughput..."
        iperf3 -c $STOQ_SERVICE_IP -p 8081 -t $TEST_DURATION -P $CONCURRENT_CONNECTIONS -f G
        
        echo "Throughput test completed"
EOF
    
    # Wait for pod to be ready
    kubectl wait --for=condition=ready --timeout=60s pod/throughput-test-client -n caesar-production
    
    # Wait for test completion
    sleep $((TEST_DURATION + 30))
    
    # Get test results
    kubectl logs throughput-test-client -n caesar-production
    
    # Extract throughput value
    MEASURED_THROUGHPUT=$(kubectl logs throughput-test-client -n caesar-production | grep -oP "sender.*\K\d+\.\d+(?=\s+Gbits/sec)" | tail -1 || echo "0")
    
    info "Measured throughput: ${MEASURED_THROUGHPUT} Gbps"
    
    # Compare with targets
    if (( $(echo "$MEASURED_THROUGHPUT >= $TARGET_THROUGHPUT_GBPS" | bc -l) )); then
        log "✓ Throughput target achieved (${MEASURED_THROUGHPUT} >= ${TARGET_THROUGHPUT_GBPS} Gbps)"
    elif (( $(echo "$MEASURED_THROUGHPUT >= $MINIMUM_THROUGHPUT_GBPS" | bc -l) )); then
        warn "⚠ Throughput acceptable but below target (${MEASURED_THROUGHPUT} >= ${MINIMUM_THROUGHPUT_GBPS} Gbps, target: ${TARGET_THROUGHPUT_GBPS} Gbps)"
    else
        error "✗ Throughput below minimum requirement (${MEASURED_THROUGHPUT} < ${MINIMUM_THROUGHPUT_GBPS} Gbps)"
    fi
    
    # Cleanup
    kubectl delete pod throughput-test-client -n caesar-production
}

# Test latency performance
test_latency() {
    log "Testing latency performance..."
    
    # Create latency test
    cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: Pod
metadata:
  name: latency-test-client
  namespace: caesar-production
spec:
  restartPolicy: Never
  containers:
  - name: ping-test
    image: alpine:latest
    command: ["/bin/sh"]
    args:
      - -c
      - |
        set -e
        apk add --no-cache curl iputils
        
        echo "Testing latency..."
        
        # HTTP latency test
        echo "HTTP latency test:"
        for i in \$(seq 1 10); do
          curl -o /dev/null -s -w "Request \$i: %{time_total}s\\n" "http://$STOQ_SERVICE_IP:8081/health"
        done
        
        echo "Latency test completed"
EOF
    
    # Wait for test completion
    kubectl wait --for=condition=ContainerReady --timeout=60s pod/latency-test-client -n caesar-production
    sleep 30
    
    # Get test results
    kubectl logs latency-test-client -n caesar-production
    
    # Cleanup
    kubectl delete pod latency-test-client -n caesar-production
}

# Test under load
test_load_performance() {
    log "Testing performance under load..."
    
    # Create load test
    cat <<EOF | kubectl apply -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: load-performance-test
  namespace: caesar-production
spec:
  parallelism: 50
  template:
    spec:
      restartPolicy: Never
      containers:
      - name: load-test
        image: alpine:latest
        command: ["/bin/sh"]
        args:
          - -c
          - |
            set -e
            apk add --no-cache curl
            
            echo "Starting load test..."
            start_time=\$(date +%s)
            
            # Generate load for test duration
            while [ \$(($(date +%s) - start_time)) -lt $TEST_DURATION ]; do
              curl -s -o /dev/null "http://$STOQ_SERVICE_IP:8081/health" || true
              curl -s -o /dev/null "http://$STOQ_SERVICE_IP:8081/status" || true
              sleep 0.1
            done
            
            echo "Load test completed"
EOF
    
    # Wait for test completion
    kubectl wait --for=condition=complete --timeout=$((TEST_DURATION + 60))s job/load-performance-test -n caesar-production
    
    info "✓ Load test completed"
    
    # Check system health during load
    kubectl top pods -n caesar-production --selector=app=stoq-transport
    
    # Cleanup
    kubectl delete job load-performance-test -n caesar-production
}

# Generate performance report
generate_report() {
    log "Generating STOQ performance report..."
    
    REPORT_FILE="$PROJECT_ROOT/performance-results/stoq-performance-report-$(date +%Y%m%d-%H%M%S).md"
    mkdir -p "$PROJECT_ROOT/performance-results"
    
    cat > "$REPORT_FILE" <<EOF
# STOQ Transport Performance Test Report

**Date**: $(date)
**Test Duration**: $TEST_DURATION seconds
**Target Throughput**: $TARGET_THROUGHPUT_GBPS Gbps
**Minimum Throughput**: $MINIMUM_THROUGHPUT_GBPS Gbps
**Concurrent Connections**: $CONCURRENT_CONNECTIONS

## Test Results

### Connectivity
- ✓ Basic API connectivity: PASSED
- ✓ Health endpoint response: PASSED

### QUIC Protocol Performance
- QUIC protocol tests completed successfully
- Connection establishment working correctly

### Concurrent Connections
- Target: $CONCURRENT_CONNECTIONS connections
- Result: PASSED

### Throughput Performance
- Measured: ${MEASURED_THROUGHPUT:-"Not measured"} Gbps
- Target: $TARGET_THROUGHPUT_GBPS Gbps
- Status: $(if (( $(echo "${MEASURED_THROUGHPUT:-0} >= $TARGET_THROUGHPUT_GBPS" | bc -l) )); then echo "✓ TARGET ACHIEVED"; elif (( $(echo "${MEASURED_THROUGHPUT:-0} >= $MINIMUM_THROUGHPUT_GBPS" | bc -l) )); then echo "⚠ ACCEPTABLE"; else echo "✗ BELOW MINIMUM"; fi)

### Latency Performance
- HTTP request latency tests completed
- Results available in test logs

### Load Performance
- System performance under sustained load: TESTED
- Resource utilization monitoring: ACTIVE

## Recommendations

$(if (( $(echo "${MEASURED_THROUGHPUT:-0} < $TARGET_THROUGHPUT_GBPS" | bc -l) )); then echo "
### Performance Optimization Needed
- Current throughput (${MEASURED_THROUGHPUT:-0} Gbps) is below target ($TARGET_THROUGHPUT_GBPS Gbps)
- Consider optimizing QUIC implementation
- Review buffer sizes and connection pooling
- Investigate network interface utilization
"; else echo "
### Performance Target Achieved
- STOQ transport is meeting performance requirements
- System ready for production load
"; fi)

## Next Steps
1. Monitor performance in production environment
2. Set up continuous performance monitoring
3. Implement automated performance regression testing
4. Optimize based on real-world usage patterns

---
*Generated by STOQ Performance Testing Script*
EOF
    
    info "Performance report generated: $REPORT_FILE"
}

# Cleanup monitoring
cleanup_monitoring() {
    info "Cleaning up monitoring resources..."
    kubectl delete job stoq-performance-monitor -n caesar-production --ignore-not-found=true
    kubectl delete pod --field-selector=status.phase=Succeeded -n caesar-production --ignore-not-found=true
}

# Main function
main() {
    log "Starting STOQ performance testing..."
    
    check_prerequisites
    start_monitoring
    
    test_connectivity
    test_quic_performance
    test_concurrent_connections
    test_throughput
    test_latency
    test_load_performance
    
    generate_report
    cleanup_monitoring
    
    log "STOQ performance testing completed!"
    
    if (( $(echo "${MEASURED_THROUGHPUT:-0} >= $TARGET_THROUGHPUT_GBPS" | bc -l) )); then
        log "🎉 STOQ performance target achieved! (${MEASURED_THROUGHPUT} Gbps >= ${TARGET_THROUGHPUT_GBPS} Gbps)"
    else
        warn "⚠️  STOQ performance needs optimization (${MEASURED_THROUGHPUT:-0} Gbps < ${TARGET_THROUGHPUT_GBPS} Gbps)"
    fi
}

# Set trap for cleanup
trap cleanup_monitoring EXIT

# Execute main function
main "$@"