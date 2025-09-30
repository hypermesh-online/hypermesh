#!/bin/bash
# Phoenix Production Monitoring Script
# Real-time monitoring and health checks for Phoenix infrastructure

set -euo pipefail

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
ENVIRONMENT="${1:-production}"
NAMESPACE="phoenix-system"
CHECK_INTERVAL="${CHECK_INTERVAL:-30}"
ALERT_THRESHOLD_CPU=80
ALERT_THRESHOLD_MEMORY=80
ALERT_THRESHOLD_LATENCY=100  # milliseconds

# Functions
log_info() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} ${CYAN}INFO${NC} $1"
}

log_success() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} ${GREEN}SUCCESS${NC} $1"
}

log_warning() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} ${YELLOW}WARNING${NC} $1"
}

log_error() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} ${RED}ERROR${NC} $1"
}

# Check cluster health
check_cluster_health() {
    log_info "Checking cluster health..."

    # Check nodes
    local node_count=$(kubectl get nodes --no-headers | wc -l)
    local ready_nodes=$(kubectl get nodes --no-headers | grep -c Ready || true)

    if [ "$node_count" -eq "$ready_nodes" ]; then
        log_success "All $node_count nodes are ready"
    else
        log_warning "Only $ready_nodes of $node_count nodes are ready"
        kubectl get nodes
    fi

    # Check system pods
    local system_pods=$(kubectl get pods -n kube-system --no-headers | wc -l)
    local running_system_pods=$(kubectl get pods -n kube-system --field-selector=status.phase=Running --no-headers | wc -l)

    if [ "$system_pods" -eq "$running_system_pods" ]; then
        log_success "All system pods are running"
    else
        log_warning "System pods issues: $running_system_pods/$system_pods running"
    fi
}

# Check Phoenix services
check_phoenix_services() {
    log_info "Checking Phoenix services..."

    local services=("phoenix-transport" "phoenix-certs" "phoenix-dashboard")

    for service in "${services[@]}"; do
        local replicas=$(kubectl get deployment -n "$NAMESPACE" "$service" -o jsonpath='{.spec.replicas}' 2>/dev/null || echo "0")
        local ready=$(kubectl get deployment -n "$NAMESPACE" "$service" -o jsonpath='{.status.readyReplicas}' 2>/dev/null || echo "0")

        if [ "$replicas" -eq "$ready" ] && [ "$replicas" -gt 0 ]; then
            log_success "$service: $ready/$replicas replicas ready"
        else
            log_error "$service: $ready/$replicas replicas ready"
            kubectl describe deployment -n "$NAMESPACE" "$service" | tail -20
        fi
    done
}

# Check resource usage
check_resource_usage() {
    log_info "Checking resource usage..."

    # Get pod metrics
    local pods=$(kubectl get pods -n "$NAMESPACE" --no-headers -o custom-columns=":metadata.name")

    for pod in $pods; do
        # Get CPU and memory usage
        local metrics=$(kubectl top pod "$pod" -n "$NAMESPACE" --no-headers 2>/dev/null || echo "N/A N/A N/A")

        if [ "$metrics" != "N/A N/A N/A" ]; then
            local cpu=$(echo "$metrics" | awk '{print $2}' | sed 's/m//')
            local memory=$(echo "$metrics" | awk '{print $3}' | sed 's/Mi//')

            # Convert to percentage (assuming 2000m CPU and 4096Mi memory limits)
            local cpu_percent=$((cpu * 100 / 2000))
            local memory_percent=$((memory * 100 / 4096))

            if [ "$cpu_percent" -gt "$ALERT_THRESHOLD_CPU" ]; then
                log_warning "$pod: High CPU usage (${cpu_percent}%)"
            fi

            if [ "$memory_percent" -gt "$ALERT_THRESHOLD_MEMORY" ]; then
                log_warning "$pod: High memory usage (${memory_percent}%)"
            fi

            echo "  $pod: CPU ${cpu}m (${cpu_percent}%), Memory ${memory}Mi (${memory_percent}%)"
        fi
    done
}

# Check service endpoints
check_endpoints() {
    log_info "Checking service endpoints..."

    # Get load balancer endpoints
    local transport_lb=$(kubectl get service -n "$NAMESPACE" phoenix-transport -o jsonpath='{.status.loadBalancer.ingress[0].hostname}' 2>/dev/null || echo "")
    local dashboard_ingress=$(kubectl get ingress -n "$NAMESPACE" phoenix-dashboard -o jsonpath='{.spec.rules[0].host}' 2>/dev/null || echo "")

    if [ -n "$transport_lb" ]; then
        log_info "Phoenix Transport endpoint: $transport_lb:9292"

        # Test QUIC endpoint (using nc for UDP)
        if nc -u -z -w 1 "$transport_lb" 9292 2>/dev/null; then
            log_success "Phoenix Transport is accessible"
        else
            log_warning "Phoenix Transport endpoint not responding"
        fi
    fi

    if [ -n "$dashboard_ingress" ]; then
        log_info "Phoenix Dashboard: https://$dashboard_ingress"

        # Test HTTPS endpoint
        if curl -sSf "https://$dashboard_ingress/health" -o /dev/null 2>/dev/null; then
            log_success "Phoenix Dashboard is healthy"
        else
            log_warning "Phoenix Dashboard not responding"
        fi
    fi
}

# Check metrics and performance
check_performance() {
    log_info "Checking performance metrics..."

    # Query Prometheus for metrics (if available)
    local prometheus_pod=$(kubectl get pod -n monitoring -l app.kubernetes.io/name=prometheus -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")

    if [ -n "$prometheus_pod" ]; then
        # Port forward to Prometheus
        kubectl port-forward -n monitoring "pod/$prometheus_pod" 9090:9090 &
        local pf_pid=$!
        sleep 2

        # Query metrics
        local latency=$(curl -s "http://localhost:9090/api/v1/query?query=histogram_quantile(0.95,phoenix_transport_latency_seconds_bucket)" | \
            jq -r '.data.result[0].value[1]' 2>/dev/null || echo "N/A")

        if [ "$latency" != "N/A" ] && [ "$latency" != "null" ]; then
            local latency_ms=$(echo "$latency * 1000" | bc)
            if (( $(echo "$latency_ms > $ALERT_THRESHOLD_LATENCY" | bc -l) )); then
                log_warning "High latency detected: ${latency_ms}ms (P95)"
            else
                log_success "Latency within limits: ${latency_ms}ms (P95)"
            fi
        fi

        # Kill port forward
        kill $pf_pid 2>/dev/null || true
    fi
}

# Check logs for errors
check_logs() {
    log_info "Checking logs for errors..."

    local services=("phoenix-transport" "phoenix-certs" "phoenix-dashboard")

    for service in "${services[@]}"; do
        local pod=$(kubectl get pod -n "$NAMESPACE" -l app="$service" -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")

        if [ -n "$pod" ]; then
            local error_count=$(kubectl logs -n "$NAMESPACE" "$pod" --tail=100 2>/dev/null | grep -ciE "error|panic|fatal" || echo "0")

            if [ "$error_count" -gt 0 ]; then
                log_warning "$service: Found $error_count error messages in recent logs"
                kubectl logs -n "$NAMESPACE" "$pod" --tail=20 | grep -iE "error|panic|fatal" || true
            else
                log_success "$service: No errors in recent logs"
            fi
        fi
    done
}

# Check persistent volumes
check_storage() {
    log_info "Checking persistent storage..."

    local pvcs=$(kubectl get pvc -n "$NAMESPACE" --no-headers -o custom-columns=":metadata.name,:status.phase")

    if [ -n "$pvcs" ]; then
        echo "$pvcs" | while read -r pvc status; do
            if [ "$status" == "Bound" ]; then
                log_success "PVC $pvc is bound"
            else
                log_error "PVC $pvc is in $status state"
            fi
        done
    else
        log_info "No persistent volume claims found"
    fi
}

# Check certificates
check_certificates() {
    log_info "Checking TLS certificates..."

    local secrets=$(kubectl get secret -n "$NAMESPACE" -o json | jq -r '.items[] | select(.type=="kubernetes.io/tls") | .metadata.name')

    for secret in $secrets; do
        local cert=$(kubectl get secret -n "$NAMESPACE" "$secret" -o jsonpath='{.data.tls\.crt}' | base64 -d)
        local expiry=$(echo "$cert" | openssl x509 -noout -enddate 2>/dev/null | cut -d= -f2)

        if [ -n "$expiry" ]; then
            local expiry_epoch=$(date -d "$expiry" +%s)
            local now_epoch=$(date +%s)
            local days_left=$(( (expiry_epoch - now_epoch) / 86400 ))

            if [ "$days_left" -lt 30 ]; then
                log_warning "Certificate $secret expires in $days_left days"
            else
                log_success "Certificate $secret valid for $days_left days"
            fi
        fi
    done
}

# Generate status report
generate_report() {
    log_info "Generating status report..."

    cat << EOF

================================================================================
Phoenix Infrastructure Status Report
Environment: $ENVIRONMENT
Timestamp: $(date '+%Y-%m-%d %H:%M:%S')
================================================================================

CLUSTER STATUS:
$(kubectl get nodes --no-headers | awk '{print "  Node: " $1 " - Status: " $2}')

PHOENIX SERVICES:
$(kubectl get deployment -n "$NAMESPACE" --no-headers | awk '{print "  " $1 ": " $3 "/" $4 " replicas ready"}')

RESOURCE USAGE:
$(kubectl top nodes --no-headers 2>/dev/null | awk '{print "  Node " $1 ": CPU " $2 " (" $3 "), Memory " $4 " (" $5 ")"}' || echo "  Metrics not available")

ENDPOINTS:
  Transport: $(kubectl get service -n "$NAMESPACE" phoenix-transport -o jsonpath='{.status.loadBalancer.ingress[0].hostname}' 2>/dev/null || echo "Not available"):9292
  Dashboard: https://$(kubectl get ingress -n "$NAMESPACE" phoenix-dashboard -o jsonpath='{.spec.rules[0].host}' 2>/dev/null || echo "Not available")

RECENT EVENTS:
$(kubectl get events -n "$NAMESPACE" --sort-by='.lastTimestamp' | tail -5)

================================================================================
EOF
}

# Continuous monitoring loop
monitor_loop() {
    log_info "Starting continuous monitoring (interval: ${CHECK_INTERVAL}s)"
    log_info "Press Ctrl+C to stop"

    while true; do
        clear
        echo -e "${CYAN}Phoenix Infrastructure Monitor - $(date '+%Y-%m-%d %H:%M:%S')${NC}"
        echo "================================================================"

        check_cluster_health
        check_phoenix_services
        check_resource_usage
        check_endpoints
        check_performance
        check_storage
        check_certificates

        echo "================================================================"
        echo "Next check in ${CHECK_INTERVAL} seconds..."

        sleep "$CHECK_INTERVAL"
    done
}

# Main execution
main() {
    log_info "Phoenix Production Monitoring"
    log_info "Environment: $ENVIRONMENT"

    # Check prerequisites
    if ! command -v kubectl &> /dev/null; then
        log_error "kubectl not found. Please install kubectl."
        exit 1
    fi

    # Update kubeconfig
    aws eks update-kubeconfig --region us-east-1 --name "phoenix-$ENVIRONMENT" 2>/dev/null || true

    # Check if we can connect to cluster
    if ! kubectl cluster-info &> /dev/null; then
        log_error "Cannot connect to Kubernetes cluster"
        exit 1
    fi

    # Parse command
    case "${2:-monitor}" in
        check)
            check_cluster_health
            check_phoenix_services
            check_endpoints
            ;;
        report)
            generate_report
            ;;
        logs)
            check_logs
            ;;
        monitor)
            monitor_loop
            ;;
        *)
            echo "Usage: $0 [environment] [check|report|logs|monitor]"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"