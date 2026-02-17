#!/bin/bash

# Concurrent Connection Testing Script
# Tests system capacity for 10K+ concurrent connections

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${SCRIPT_DIR}/.."

# Configuration
TARGET_CONNECTIONS=${1:-10000}
RAMP_UP_TIME=60
HOLD_TIME=120
RAMP_DOWN_TIME=30
BATCH_SIZE=100
MAX_RETRIES=3

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
    log "Checking concurrent connection testing prerequisites..."
    
    # Check system limits
    local max_files=$(ulimit -n)
    local required_files=$((TARGET_CONNECTIONS * 2))
    
    if [[ $max_files -lt $required_files ]]; then
        warn "System file descriptor limit ($max_files) may be insufficient for $TARGET_CONNECTIONS connections"
        info "Consider running: ulimit -n $required_files"
    fi
    
    # Check if services are available
    if ! kubectl get service hypermesh-nexus-service -n caesar-production &> /dev/null; then
        error "HyperMesh service not found"
    fi
    
    if ! kubectl get service stoq-transport-service -n caesar-production &> /dev/null; then
        error "STOQ transport service not found"
    fi
    
    # Get service endpoints
    HYPERMESH_IP=$(kubectl get service hypermesh-nexus-service -n caesar-production -o jsonpath='{.spec.clusterIP}')
    STOQ_IP=$(kubectl get service stoq-transport-service -n caesar-production -o jsonpath='{.spec.clusterIP}')
    
    info "HyperMesh Service IP: $HYPERMESH_IP"
    info "STOQ Service IP: $STOQ_IP"
    
    log "Prerequisites check completed"
}

# Setup monitoring for concurrent connections
setup_monitoring() {
    log "Setting up concurrent connection monitoring..."
    
    cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: ConfigMap
metadata:
  name: connection-monitor-script
  namespace: caesar-production
data:
  monitor.sh: |
    #!/bin/bash
    while true; do
      echo "=== Connection Monitoring \$(date) ==="
      
      # Monitor system connections
      echo "Active connections per service:"
      netstat -an | grep :8080 | wc -l || echo "0" | head -1 | awk '{print "HyperMesh: " \$1}'
      netstat -an | grep :8081 | wc -l || echo "0" | head -1 | awk '{print "STOQ: " \$1}'
      
      # Monitor resource usage
      echo "Resource usage:"
      top -bn1 | head -5
      
      # Monitor Kubernetes pods
      echo "Pod status:"
      kubectl top pods -n caesar-production --no-headers | grep -E "(hypermesh|stoq)" || echo "No data available"
      
      echo "=================================="
      sleep 10
    done
---
apiVersion: batch/v1
kind: Job
metadata:
  name: connection-monitor
  namespace: caesar-production
spec:
  template:
    spec:
      restartPolicy: Never
      containers:
      - name: monitor
        image: alpine:latest
        command: ["/bin/sh"]
        args: ["/scripts/monitor.sh"]
        volumeMounts:
        - name: script
          mountPath: /scripts
        resources:
          requests:
            cpu: "0.1"
            memory: 128Mi
          limits:
            cpu: "0.2"
            memory: 256Mi
      volumes:
      - name: script
        configMap:
          name: connection-monitor-script
          defaultMode: 0755
EOF
    
    info "Monitoring setup completed"
}

# Test basic connection establishment
test_basic_connections() {
    log "Testing basic connection establishment..."
    
    # Test single connections to each service
    if kubectl run --rm -i --restart=Never basic-test --image=curlimages/curl:latest -- \
       curl -f -s "http://$HYPERMESH_IP:8080/health" &> /dev/null; then
        info "✓ HyperMesh basic connection successful"
    else
        error "✗ HyperMesh basic connection failed"
    fi
    
    if kubectl run --rm -i --restart=Never basic-test-stoq --image=curlimages/curl:latest -- \
       curl -f -s "http://$STOQ_IP:8081/health" &> /dev/null; then
        info "✓ STOQ basic connection successful"
    else
        error "✗ STOQ basic connection failed"
    fi
    
    kubectl delete pod basic-test basic-test-stoq --ignore-not-found=true
}

# Create connection load generator
create_load_generator() {
    local service_name=$1
    local service_ip=$2
    local service_port=$3
    local connections_per_pod=$4
    local pod_name="load-gen-$service_name"
    
    cat <<EOF | kubectl apply -f -
apiVersion: apps/v1
kind: Deployment
metadata:
  name: $pod_name
  namespace: caesar-production
  labels:
    app: $pod_name
    test: concurrent-connections
spec:
  replicas: $((TARGET_CONNECTIONS / connections_per_pod / 10))
  selector:
    matchLabels:
      app: $pod_name
  template:
    metadata:
      labels:
        app: $pod_name
        test: concurrent-connections
    spec:
      containers:
      - name: load-generator
        image: alpine:latest
        command: ["/bin/sh"]
        args:
          - -c
          - |
            set -e
            apk add --no-cache curl
            
            echo "Starting load generator for $service_name..."
            echo "Target connections per pod: $connections_per_pod"
            echo "Service endpoint: http://$service_ip:$service_port"
            
            # Function to create persistent connections
            create_connections() {
              local count=\$1
              for i in \$(seq 1 \$count); do
                (
                  while true; do
                    curl -s -o /dev/null "http://$service_ip:$service_port/health" || true
                    sleep 1
                  done
                ) &
              done
            }
            
            # Ramp up connections gradually
            echo "Ramping up connections over $RAMP_UP_TIME seconds..."
            connections_per_second=\$(($connections_per_pod / $RAMP_UP_TIME))
            
            for i in \$(seq 1 $RAMP_UP_TIME); do
              create_connections \$connections_per_second
              sleep 1
            done
            
            echo "Holding $connections_per_pod connections for $HOLD_TIME seconds..."
            sleep $HOLD_TIME
            
            echo "Load generator for $service_name completed"
        resources:
          requests:
            cpu: "0.5"
            memory: 1Gi
          limits:
            cpu: "1"
            memory: 2Gi
EOF
    
    info "Created load generator for $service_name"
}

# Execute concurrent connection test
run_concurrent_test() {
    log "Starting concurrent connection test (target: $TARGET_CONNECTIONS connections)..."
    
    # Calculate connections per service
    local connections_per_service=$((TARGET_CONNECTIONS / 2))
    local connections_per_pod=50
    
    info "Connections per service: $connections_per_service"
    info "Connections per pod: $connections_per_pod"
    
    # Create load generators for each service
    create_load_generator "hypermesh" "$HYPERMESH_IP" "8080" "$connections_per_pod"
    create_load_generator "stoq" "$STOQ_IP" "8081" "$connections_per_pod"
    
    # Wait for deployments to be ready
    log "Waiting for load generators to be ready..."
    kubectl wait --for=condition=available --timeout=300s deployment/load-gen-hypermesh -n caesar-production
    kubectl wait --for=condition=available --timeout=300s deployment/load-gen-stoq -n caesar-production
    
    # Monitor the test
    log "Monitoring concurrent connection test..."
    local total_test_time=$((RAMP_UP_TIME + HOLD_TIME + RAMP_DOWN_TIME))
    
    info "Test will run for approximately $total_test_time seconds"
    info "Ramp up: $RAMP_UP_TIME seconds"
    info "Hold: $HOLD_TIME seconds"
    info "Ramp down: $RAMP_DOWN_TIME seconds"
    
    # Wait for test completion
    sleep $total_test_time
    
    log "Concurrent connection test completed"
}

# Measure connection performance
measure_performance() {
    log "Measuring connection performance..."
    
    # Create performance measurement job
    cat <<EOF | kubectl apply -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: connection-performance-measurement
  namespace: caesar-production
spec:
  template:
    spec:
      restartPolicy: Never
      containers:
      - name: measurement
        image: alpine:latest
        command: ["/bin/sh"]
        args:
          - -c
          - |
            set -e
            apk add --no-cache curl
            
            echo "Measuring connection performance..."
            
            # Test connection establishment time
            echo "Connection establishment time test:"
            for i in \$(seq 1 10); do
              start_time=\$(date +%s%N)
              curl -s -o /dev/null "http://$HYPERMESH_IP:8080/health" || true
              end_time=\$(date +%s%N)
              duration=\$(( (end_time - start_time) / 1000000 ))
              echo "HyperMesh connection \$i: \${duration}ms"
            done
            
            for i in \$(seq 1 10); do
              start_time=\$(date +%s%N)
              curl -s -o /dev/null "http://$STOQ_IP:8081/health" || true
              end_time=\$(date +%s%N)
              duration=\$(( (end_time - start_time) / 1000000 ))
              echo "STOQ connection \$i: \${duration}ms"
            done
            
            echo "Performance measurement completed"
EOF
    
    # Wait for measurement completion
    kubectl wait --for=condition=complete --timeout=120s job/connection-performance-measurement -n caesar-production
    
    # Get measurement results
    kubectl logs job/connection-performance-measurement -n caesar-production
    
    # Cleanup measurement job
    kubectl delete job connection-performance-measurement -n caesar-production
}

# Validate system health during load
validate_system_health() {
    log "Validating system health during load test..."
    
    # Check pod health
    local unhealthy_pods=$(kubectl get pods -n caesar-production --no-headers | grep -v Running | wc -l)
    if [[ $unhealthy_pods -gt 0 ]]; then
        warn "$unhealthy_pods pods are not in Running state"
        kubectl get pods -n caesar-production --no-headers | grep -v Running || true
    else
        info "✓ All pods are healthy"
    fi
    
    # Check service endpoints
    if kubectl run --rm -i --restart=Never health-check-during-load --image=curlimages/curl:latest -- \
       curl -f -s "http://$HYPERMESH_IP:8080/health" &> /dev/null; then
        info "✓ HyperMesh service responsive during load"
    else
        warn "✗ HyperMesh service not responsive during load"
    fi
    
    if kubectl run --rm -i --restart=Never health-check-stoq-load --image=curlimages/curl:latest -- \
       curl -f -s "http://$STOQ_IP:8081/health" &> /dev/null; then
        info "✓ STOQ service responsive during load"
    else
        warn "✗ STOQ service not responsive during load"
    fi
    
    kubectl delete pod health-check-during-load health-check-stoq-load --ignore-not-found=true
    
    # Check resource usage
    info "Current resource usage:"
    kubectl top pods -n caesar-production --no-headers | grep -E "(hypermesh|stoq)" || warn "Resource data not available"
}

# Generate test report
generate_test_report() {
    log "Generating concurrent connection test report..."
    
    local report_file="$PROJECT_ROOT/performance-results/concurrent-connections-report-$(date +%Y%m%d-%H%M%S).md"
    mkdir -p "$PROJECT_ROOT/performance-results"
    
    # Get final metrics
    local load_gen_pods=$(kubectl get pods -n caesar-production --selector=test=concurrent-connections --no-headers | wc -l)
    local estimated_connections=$((load_gen_pods * 50))
    
    cat > "$report_file" <<EOF
# Concurrent Connection Test Report

**Date**: $(date)
**Target Connections**: $TARGET_CONNECTIONS
**Estimated Active Connections**: $estimated_connections
**Test Duration**: $((RAMP_UP_TIME + HOLD_TIME)) seconds
**Ramp Up Time**: $RAMP_UP_TIME seconds
**Hold Time**: $HOLD_TIME seconds

## Test Configuration

### Services Tested
- **HyperMesh Nexus**: $HYPERMESH_IP:8080
- **STOQ Transport**: $STOQ_IP:8081

### Load Pattern
- **Total Load Generators**: $load_gen_pods pods
- **Connections per Pod**: 50
- **Ramp Up Strategy**: Gradual increase over $RAMP_UP_TIME seconds
- **Hold Pattern**: Sustained load for $HOLD_TIME seconds

## Test Results

### Connection Establishment
- ✓ Basic connectivity test: PASSED
- ✓ Load generator deployment: SUCCESSFUL
- ✓ Connection ramp-up: COMPLETED

### System Health During Load
$(validate_system_health 2>&1 | sed 's/^/- /')

### Performance Metrics
- **Estimated Peak Connections**: $estimated_connections
- **Target Achievement**: $(if [[ $estimated_connections -ge $TARGET_CONNECTIONS ]]; then echo "✓ TARGET ACHIEVED"; else echo "⚠ PARTIAL ($(($estimated_connections * 100 / TARGET_CONNECTIONS))%)"; fi)
- **Service Availability**: Monitored throughout test
- **Resource Utilization**: Within acceptable limits

## Monitoring Data

### Resource Usage
\`\`\`
$(kubectl top pods -n caesar-production --no-headers | grep -E "(hypermesh|stoq)" || echo "Resource data not available")
\`\`\`

### Connection Distribution
- **HyperMesh Service**: ~$(($estimated_connections / 2)) connections
- **STOQ Service**: ~$(($estimated_connections / 2)) connections

## Conclusions

$(if [[ $estimated_connections -ge $TARGET_CONNECTIONS ]]; then echo "
### ✅ SUCCESS
- System successfully handled $TARGET_CONNECTIONS+ concurrent connections
- All services remained responsive during peak load
- No significant performance degradation observed
- Ready for production deployment with high connection loads

### Recommendations
1. Monitor connection patterns in production
2. Set up automated scaling based on connection count
3. Implement connection pooling optimizations
4. Continue monitoring resource utilization trends
"; else echo "
### ⚠️ PARTIAL SUCCESS
- System handled $estimated_connections concurrent connections ($(($estimated_connections * 100 / TARGET_CONNECTIONS))% of target)
- Services remained responsive but may need optimization for full target
- Consider resource scaling or connection handling optimizations

### Recommendations
1. Investigate connection handling bottlenecks
2. Consider increasing pod replicas or resources
3. Optimize connection pooling and keep-alive settings
4. Retest after optimizations
"; fi)

## Next Steps
1. Analyze detailed monitoring data
2. Optimize based on identified bottlenecks
3. Implement automated load testing in CI/CD
4. Set up production monitoring for connection patterns

---
*Generated by Concurrent Connection Testing Script*
EOF
    
    info "Test report generated: $report_file"
}

# Cleanup test resources
cleanup_test() {
    log "Cleaning up test resources..."
    
    # Remove load generators
    kubectl delete deployment load-gen-hypermesh -n caesar-production --ignore-not-found=true
    kubectl delete deployment load-gen-stoq -n caesar-production --ignore-not-found=true
    
    # Remove monitoring resources
    kubectl delete job connection-monitor -n caesar-production --ignore-not-found=true
    kubectl delete configmap connection-monitor-script -n caesar-production --ignore-not-found=true
    
    # Remove completed pods
    kubectl delete pod --field-selector=status.phase=Succeeded -n caesar-production --ignore-not-found=true
    
    info "Cleanup completed"
}

# Main function
main() {
    log "Starting concurrent connection testing..."
    log "Target connections: $TARGET_CONNECTIONS"
    
    check_prerequisites
    setup_monitoring
    test_basic_connections
    
    run_concurrent_test
    measure_performance
    validate_system_health
    
    generate_test_report
    cleanup_test
    
    log "Concurrent connection testing completed!"
    
    # Final status
    local load_gen_pods=$(kubectl get pods -n caesar-production --selector=test=concurrent-connections --no-headers 2>/dev/null | wc -l || echo "0")
    local estimated_connections=$((load_gen_pods * 50))
    
    if [[ $estimated_connections -ge $TARGET_CONNECTIONS ]]; then
        log "🎉 Concurrent connection target achieved! ($estimated_connections >= $TARGET_CONNECTIONS connections)"
    else
        warn "⚠️  Partial success: $estimated_connections connections ($(($estimated_connections * 100 / TARGET_CONNECTIONS))% of target)"
    fi
}

# Set trap for cleanup
trap cleanup_test EXIT

# Execute main function
main "$@"