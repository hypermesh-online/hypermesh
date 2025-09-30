#!/bin/bash

#############################################################
# Deployment Orchestrator for HyperMesh Ecosystem
# Manages coordinated deployment of all components
#############################################################

set -euo pipefail

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
readonly COMPONENTS=("stoq" "trustchain" "catalog" "caesar" "hypermesh")
readonly DEPLOYMENT_TIMEOUT=1800  # 30 minutes

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Deployment environment
ENVIRONMENT="${1:-staging}"
VERSION="${2:-latest}"
DRY_RUN="${3:-false}"

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# Validation functions
validate_environment() {
    case "$ENVIRONMENT" in
        development|staging|production)
            log_info "Deploying to environment: $ENVIRONMENT"
            ;;
        *)
            log_error "Invalid environment: $ENVIRONMENT"
            log_error "Valid environments: development, staging, production"
            exit 1
            ;;
    esac
}

validate_prerequisites() {
    log_info "Validating prerequisites..."

    # Check required tools
    local required_tools=("docker" "kubectl" "helm" "curl" "jq")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "Required tool not found: $tool"
            exit 1
        fi
    done

    # Check Kubernetes connectivity
    if ! kubectl cluster-info &> /dev/null; then
        log_error "Cannot connect to Kubernetes cluster"
        exit 1
    fi

    # Check namespace exists
    local namespace="hypermesh-$ENVIRONMENT"
    if ! kubectl get namespace "$namespace" &> /dev/null; then
        log_warning "Namespace $namespace does not exist, creating..."
        kubectl create namespace "$namespace"
    fi

    log_success "Prerequisites validated"
}

# Build functions
build_component() {
    local component=$1
    log_info "Building $component..."

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would build $component"
        return 0
    fi

    cd "$PROJECT_ROOT/$component"

    # Build release binary
    cargo build --release --locked

    # Build Docker image
    local image_tag="ghcr.io/hypermesh-online/$component:$VERSION"
    docker build -t "$image_tag" .

    # Push to registry
    docker push "$image_tag"

    cd "$PROJECT_ROOT"
    log_success "Built $component successfully"
}

build_all_components() {
    log_info "Building all components..."

    for component in "${COMPONENTS[@]}"; do
        build_component "$component" &
    done

    # Wait for all builds to complete
    wait

    log_success "All components built successfully"
}

# Deployment functions
deploy_component() {
    local component=$1
    local namespace="hypermesh-$ENVIRONMENT"

    log_info "Deploying $component to $namespace..."

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would deploy $component to $namespace"
        return 0
    fi

    # Check if deployment exists
    if kubectl get deployment "$component" -n "$namespace" &> /dev/null; then
        # Update existing deployment
        kubectl set image "deployment/$component" \
            "$component=ghcr.io/hypermesh-online/$component:$VERSION" \
            -n "$namespace"
    else
        # Create new deployment
        kubectl create deployment "$component" \
            --image="ghcr.io/hypermesh-online/$component:$VERSION" \
            -n "$namespace"
    fi

    # Wait for rollout to complete
    kubectl rollout status "deployment/$component" -n "$namespace" \
        --timeout="${DEPLOYMENT_TIMEOUT}s"

    log_success "Deployed $component successfully"
}

deploy_in_order() {
    log_info "Deploying components in dependency order..."

    # Deploy foundation components first
    deploy_component "trustchain"
    deploy_component "stoq"

    # Deploy core components
    deploy_component "catalog"

    # Deploy application components
    deploy_component "hypermesh"
    deploy_component "caesar"

    log_success "All components deployed successfully"
}

# Health check functions
check_component_health() {
    local component=$1
    local namespace="hypermesh-$ENVIRONMENT"
    local max_retries=30
    local retry=0

    log_info "Checking health of $component..."

    while [ $retry -lt $max_retries ]; do
        # Get pod status
        local pod_status=$(kubectl get pods -n "$namespace" \
            -l "app=$component" \
            -o jsonpath='{.items[0].status.phase}' 2>/dev/null || echo "Unknown")

        if [ "$pod_status" == "Running" ]; then
            # Check if pod is ready
            local ready=$(kubectl get pods -n "$namespace" \
                -l "app=$component" \
                -o jsonpath='{.items[0].status.containerStatuses[0].ready}' 2>/dev/null || echo "false")

            if [ "$ready" == "true" ]; then
                log_success "$component is healthy"
                return 0
            fi
        fi

        log_warning "$component not ready yet (attempt $((retry+1))/$max_retries)"
        sleep 10
        retry=$((retry + 1))
    done

    log_error "$component failed health check"
    return 1
}

verify_deployment() {
    log_info "Verifying deployment..."

    local all_healthy=true

    for component in "${COMPONENTS[@]}"; do
        if ! check_component_health "$component"; then
            all_healthy=false
        fi
    done

    if [ "$all_healthy" == "true" ]; then
        log_success "All components are healthy"
        return 0
    else
        log_error "Some components are unhealthy"
        return 1
    fi
}

# Rollback functions
rollback_component() {
    local component=$1
    local namespace="hypermesh-$ENVIRONMENT"

    log_warning "Rolling back $component..."

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would rollback $component"
        return 0
    fi

    kubectl rollout undo "deployment/$component" -n "$namespace"
    kubectl rollout status "deployment/$component" -n "$namespace" \
        --timeout="${DEPLOYMENT_TIMEOUT}s"

    log_success "Rolled back $component"
}

rollback_all() {
    log_warning "Rolling back all components..."

    for component in "${COMPONENTS[@]}"; do
        rollback_component "$component"
    done

    log_success "Rollback complete"
}

# Monitoring functions
enable_monitoring() {
    log_info "Enabling monitoring for deployment..."

    local namespace="hypermesh-$ENVIRONMENT"

    # Create ServiceMonitor for Prometheus
    cat <<EOF | kubectl apply -f -
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: hypermesh-monitoring
  namespace: $namespace
spec:
  selector:
    matchLabels:
      app: hypermesh
  endpoints:
  - port: metrics
    interval: 30s
EOF

    log_success "Monitoring enabled"
}

# Blue-Green deployment
blue_green_deployment() {
    log_info "Starting blue-green deployment..."

    local namespace="hypermesh-$ENVIRONMENT"

    for component in "${COMPONENTS[@]}"; do
        log_info "Blue-green deployment for $component..."

        # Deploy green version
        kubectl create deployment "$component-green" \
            --image="ghcr.io/hypermesh-online/$component:$VERSION" \
            -n "$namespace"

        # Wait for green deployment to be ready
        kubectl rollout status "deployment/$component-green" -n "$namespace" \
            --timeout="${DEPLOYMENT_TIMEOUT}s"

        # Switch traffic to green
        kubectl patch service "$component" -n "$namespace" \
            -p '{"spec":{"selector":{"version":"green"}}}'

        # Verify green deployment
        if check_component_health "$component-green"; then
            # Delete old blue deployment
            kubectl delete deployment "$component" -n "$namespace" --ignore-not-found=true

            # Rename green to primary
            kubectl patch deployment "$component-green" -n "$namespace" \
                -p '{"metadata":{"name":"'$component'"}}'
        else
            # Rollback to blue
            kubectl patch service "$component" -n "$namespace" \
                -p '{"spec":{"selector":{"version":"blue"}}}'
            kubectl delete deployment "$component-green" -n "$namespace"
            log_error "Blue-green deployment failed for $component"
            return 1
        fi
    done

    log_success "Blue-green deployment complete"
}

# Canary deployment
canary_deployment() {
    log_info "Starting canary deployment..."

    local namespace="hypermesh-$ENVIRONMENT"
    local canary_weight=10  # Start with 10% traffic

    for component in "${COMPONENTS[@]}"; do
        log_info "Canary deployment for $component (${canary_weight}% traffic)..."

        # Deploy canary version
        kubectl create deployment "$component-canary" \
            --image="ghcr.io/hypermesh-online/$component:$VERSION" \
            -n "$namespace" \
            --replicas=1

        # Wait for canary to be ready
        kubectl rollout status "deployment/$component-canary" -n "$namespace" \
            --timeout="${DEPLOYMENT_TIMEOUT}s"

        # Configure traffic splitting (requires service mesh like Istio)
        # This is a simplified example
        log_info "Routing ${canary_weight}% traffic to canary..."

        # Monitor canary metrics
        sleep 300  # Monitor for 5 minutes

        # If canary is healthy, gradually increase traffic
        for weight in 25 50 75 100; do
            log_info "Increasing canary traffic to ${weight}%..."
            sleep 300

            # Check canary health
            if ! check_component_health "$component-canary"; then
                log_error "Canary deployment failed for $component"
                kubectl delete deployment "$component-canary" -n "$namespace"
                return 1
            fi
        done

        # Promote canary to stable
        kubectl delete deployment "$component" -n "$namespace" --ignore-not-found=true
        kubectl patch deployment "$component-canary" -n "$namespace" \
            -p '{"metadata":{"name":"'$component'"}}'
    done

    log_success "Canary deployment complete"
}

# Main deployment flow
main() {
    log_info "HyperMesh Deployment Orchestrator"
    log_info "================================"
    log_info "Environment: $ENVIRONMENT"
    log_info "Version: $VERSION"
    log_info "Dry Run: $DRY_RUN"

    # Validate environment
    validate_environment

    # Check prerequisites
    validate_prerequisites

    # Build components
    if [[ "$ENVIRONMENT" == "production" ]]; then
        log_info "Production deployment - using pre-built images"
    else
        build_all_components
    fi

    # Deploy based on environment
    case "$ENVIRONMENT" in
        development)
            deploy_in_order
            ;;
        staging)
            blue_green_deployment
            ;;
        production)
            canary_deployment
            ;;
    esac

    # Verify deployment
    if ! verify_deployment; then
        log_error "Deployment verification failed"
        if [[ "$ENVIRONMENT" == "production" ]]; then
            log_warning "Initiating automatic rollback..."
            rollback_all
        fi
        exit 1
    fi

    # Enable monitoring
    enable_monitoring

    # Generate deployment report
    cat <<EOF > deployment-report.txt
Deployment Report
================
Date: $(date)
Environment: $ENVIRONMENT
Version: $VERSION
Status: SUCCESS

Components Deployed:
$(for c in "${COMPONENTS[@]}"; do echo "- $c"; done)

Next Steps:
1. Monitor application metrics
2. Check error rates
3. Verify user experience
EOF

    log_success "Deployment complete!"
    log_info "Report saved to deployment-report.txt"
}

# Handle script interruption
trap 'log_error "Deployment interrupted"; exit 1' INT TERM

# Run main function
main "$@"