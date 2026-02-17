#!/bin/bash

# Caesar Enterprise Blue-Green Deployment Script
# Zero-downtime production deployment with automated rollback

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${SCRIPT_DIR}/.."

# Configuration
NAMESPACE="caesar-production"
NEW_VERSION="${1:-$(git rev-parse --short HEAD)}"
HEALTH_CHECK_RETRIES=30
HEALTH_CHECK_INTERVAL=10
ROLLBACK_ON_FAILURE=${ROLLBACK_ON_FAILURE:-true}

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
    log "Checking prerequisites..."
    
    # Check kubectl
    if ! command -v kubectl &> /dev/null; then
        error "kubectl is required but not installed"
    fi
    
    # Check cluster connectivity
    if ! kubectl cluster-info &> /dev/null; then
        error "Cannot connect to Kubernetes cluster"
    fi
    
    # Check namespace exists
    if ! kubectl get namespace "$NAMESPACE" &> /dev/null; then
        error "Namespace $NAMESPACE does not exist"
    fi
    
    # Check current deployment
    if ! kubectl get deployment -n "$NAMESPACE" hypermesh-nexus &> /dev/null; then
        error "Current deployment not found in $NAMESPACE"
    fi
    
    log "Prerequisites check completed"
}

# Get current deployment state
get_current_state() {
    log "Getting current deployment state..."
    
    CURRENT_VERSION=$(kubectl get deployment hypermesh-nexus -n "$NAMESPACE" -o jsonpath='{.metadata.labels.version}' 2>/dev/null || echo "unknown")
    CURRENT_REPLICAS=$(kubectl get deployment hypermesh-nexus -n "$NAMESPACE" -o jsonpath='{.spec.replicas}')
    
    info "Current version: $CURRENT_VERSION"
    info "Current replicas: $CURRENT_REPLICAS"
    info "New version: $NEW_VERSION"
}

# Create green deployment
create_green_deployment() {
    log "Creating green deployment (version: $NEW_VERSION)..."
    
    # Update deployment manifests with new version
    sed "s/:latest/:$NEW_VERSION/g" "$PROJECT_ROOT/k8s/production/hypermesh-deployment.yaml" > "/tmp/hypermesh-green.yaml"
    sed -i "s/name: hypermesh-nexus/name: hypermesh-nexus-green/g" "/tmp/hypermesh-green.yaml"
    sed -i "s/app: hypermesh-nexus/app: hypermesh-nexus-green/g" "/tmp/hypermesh-green.yaml"
    
    # Add version label
    sed -i "/labels:/a\\    version: $NEW_VERSION" "/tmp/hypermesh-green.yaml"
    
    # Deploy green version
    kubectl apply -f "/tmp/hypermesh-green.yaml"
    
    # Wait for green deployment to be ready
    log "Waiting for green deployment to be ready..."
    kubectl wait --for=condition=available --timeout=600s deployment/hypermesh-nexus-green -n "$NAMESPACE"
    
    # Create temporary green service for testing
    kubectl get service hypermesh-nexus-service -n "$NAMESPACE" -o yaml | \
        sed 's/name: hypermesh-nexus-service/name: hypermesh-nexus-green-service/' | \
        sed 's/app: hypermesh-nexus/app: hypermesh-nexus-green/' | \
        kubectl apply -f -
}

# Health check green deployment
health_check_green() {
    log "Performing health checks on green deployment..."
    
    local retries=$HEALTH_CHECK_RETRIES
    local service_ip=""
    
    # Get service IP
    while [[ $retries -gt 0 ]]; do
        service_ip=$(kubectl get service hypermesh-nexus-green-service -n "$NAMESPACE" -o jsonpath='{.spec.clusterIP}' 2>/dev/null || echo "")
        if [[ -n "$service_ip" && "$service_ip" != "None" ]]; then
            break
        fi
        sleep $HEALTH_CHECK_INTERVAL
        ((retries--))
    done
    
    if [[ -z "$service_ip" || "$service_ip" == "None" ]]; then
        error "Could not get green service IP"
    fi
    
    info "Green service IP: $service_ip"
    
    # Health check loop
    retries=$HEALTH_CHECK_RETRIES
    while [[ $retries -gt 0 ]]; do
        if kubectl run --rm -i --restart=Never health-check-pod --image=curlimages/curl:latest -- \
           curl -f -s "http://$service_ip:8080/health" &> /dev/null; then
            log "Health check passed for green deployment"
            kubectl delete pod health-check-pod --ignore-not-found=true
            return 0
        fi
        
        info "Health check failed, retrying... ($retries attempts remaining)"
        sleep $HEALTH_CHECK_INTERVAL
        ((retries--))
    done
    
    error "Health checks failed for green deployment"
}

# Run smoke tests on green deployment
run_smoke_tests() {
    log "Running smoke tests on green deployment..."
    
    # Create test job
    cat <<EOF | kubectl apply -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: smoke-test-green-$NEW_VERSION
  namespace: $NAMESPACE
spec:
  template:
    spec:
      restartPolicy: Never
      containers:
      - name: smoke-test
        image: curlimages/curl:latest
        command: ["/bin/sh"]
        args:
          - -c
          - |
            set -e
            echo "Running smoke tests..."
            
            # Test health endpoint
            curl -f -s http://hypermesh-nexus-green-service:8080/health
            echo "✓ Health endpoint working"
            
            # Test ready endpoint
            curl -f -s http://hypermesh-nexus-green-service:8080/ready
            echo "✓ Ready endpoint working"
            
            # Test basic API functionality
            curl -f -s http://hypermesh-nexus-green-service:8080/status
            echo "✓ Status endpoint working"
            
            echo "All smoke tests passed!"
      backoffLimit: 3
EOF
    
    # Wait for smoke tests to complete
    kubectl wait --for=condition=complete --timeout=300s job/smoke-test-green-$NEW_VERSION -n "$NAMESPACE"
    
    # Check smoke test results
    if kubectl get job smoke-test-green-$NEW_VERSION -n "$NAMESPACE" -o jsonpath='{.status.conditions[?(@.type=="Complete")].status}' | grep -q "True"; then
        log "Smoke tests passed"
        kubectl delete job smoke-test-green-$NEW_VERSION -n "$NAMESPACE"
    else
        error "Smoke tests failed"
    fi
}

# Switch traffic to green (blue-green cutover)
switch_traffic() {
    log "Switching traffic to green deployment..."
    
    # Update service selector to point to green deployment
    kubectl patch service hypermesh-nexus-service -n "$NAMESPACE" -p '{"spec":{"selector":{"app":"hypermesh-nexus-green"}}}'
    
    # Wait a moment for traffic to switch
    sleep 5
    
    # Verify traffic switch
    log "Verifying traffic switch..."
    local retries=10
    while [[ $retries -gt 0 ]]; do
        if kubectl run --rm -i --restart=Never traffic-test-pod --image=curlimages/curl:latest -- \
           curl -f -s "http://hypermesh-nexus-service:8080/health" &> /dev/null; then
            log "Traffic successfully switched to green deployment"
            kubectl delete pod traffic-test-pod --ignore-not-found=true
            return 0
        fi
        
        warn "Traffic switch verification failed, retrying..."
        sleep 5
        ((retries--))
    done
    
    error "Traffic switch verification failed"
}

# Monitor new deployment
monitor_deployment() {
    log "Monitoring new deployment for $((HEALTH_CHECK_INTERVAL * 3)) seconds..."
    
    local monitor_duration=$((HEALTH_CHECK_INTERVAL * 3))
    local start_time=$(date +%s)
    
    while [[ $(($(date +%s) - start_time)) -lt $monitor_duration ]]; do
        if ! kubectl run --rm -i --restart=Never monitor-pod --image=curlimages/curl:latest -- \
           curl -f -s "http://hypermesh-nexus-service:8080/health" &> /dev/null; then
            error "Health check failed during monitoring period"
        fi
        
        kubectl delete pod monitor-pod --ignore-not-found=true
        sleep 10
    done
    
    log "Monitoring completed successfully"
}

# Clean up blue deployment
cleanup_blue() {
    log "Cleaning up blue deployment..."
    
    # Remove blue deployment (old version)
    kubectl delete deployment hypermesh-nexus -n "$NAMESPACE" --ignore-not-found=true
    
    # Rename green to blue (standard naming)
    kubectl patch deployment hypermesh-nexus-green -n "$NAMESPACE" --type='merge' -p='{"metadata":{"name":"hypermesh-nexus"}}'
    kubectl patch deployment hypermesh-nexus-green -n "$NAMESPACE" -p '{"spec":{"selector":{"matchLabels":{"app":"hypermesh-nexus"}},"template":{"metadata":{"labels":{"app":"hypermesh-nexus"}}}}}'
    
    # Remove temporary green service
    kubectl delete service hypermesh-nexus-green-service -n "$NAMESPACE" --ignore-not-found=true
    
    log "Cleanup completed"
}

# Rollback function
rollback_deployment() {
    warn "Rolling back deployment..."
    
    # Switch traffic back to blue
    kubectl patch service hypermesh-nexus-service -n "$NAMESPACE" -p '{"spec":{"selector":{"app":"hypermesh-nexus"}}}'
    
    # Remove failed green deployment
    kubectl delete deployment hypermesh-nexus-green -n "$NAMESPACE" --ignore-not-found=true
    kubectl delete service hypermesh-nexus-green-service -n "$NAMESPACE" --ignore-not-found=true
    kubectl delete job --selector="app=smoke-test" -n "$NAMESPACE" --ignore-not-found=true
    
    error "Deployment rolled back due to failure"
}

# Cleanup function for script exit
cleanup() {
    info "Cleaning up temporary resources..."
    kubectl delete pod --field-selector=status.phase=Succeeded -n "$NAMESPACE" --ignore-not-found=true
    rm -f /tmp/hypermesh-green.yaml
}

# Set trap for cleanup
trap cleanup EXIT

# Main deployment function
main() {
    log "Starting blue-green deployment for Caesar Enterprise..."
    log "Deploying version: $NEW_VERSION to namespace: $NAMESPACE"
    
    check_prerequisites
    get_current_state
    
    # Set error handling for rollback
    if [[ "$ROLLBACK_ON_FAILURE" == "true" ]]; then
        trap 'rollback_deployment' ERR
    fi
    
    create_green_deployment
    health_check_green
    run_smoke_tests
    switch_traffic
    monitor_deployment
    cleanup_blue
    
    log "Blue-green deployment completed successfully!"
    log "New version $NEW_VERSION is now serving production traffic"
    
    # Update deployment status
    kubectl annotate deployment hypermesh-nexus -n "$NAMESPACE" \
        deployment.kubernetes.io/last-deployment="$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        deployment.kubernetes.io/version="$NEW_VERSION" \
        --overwrite
}

# Execute main function
main "$@"