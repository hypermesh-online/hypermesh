#!/bin/bash
# Nexus Deployment Script
# Supports multiple deployment targets: docker-compose, kubernetes, bare-metal

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEPLOY_DIR="$PROJECT_ROOT/deploy"

# Default values
DEPLOYMENT_TYPE="${DEPLOYMENT_TYPE:-docker-compose}"
ENVIRONMENT="${ENVIRONMENT:-development}"
NAMESPACE="${NAMESPACE:-nexus-system}"
IMAGE_TAG="${IMAGE_TAG:-latest}"
REPLICAS="${REPLICAS:-3}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $*${NC}" >&2
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $*${NC}" >&2
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $*${NC}" >&2
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $*${NC}" >&2
}

# Show usage
usage() {
    cat << EOF
Nexus Deployment Script

Usage: $0 [OPTIONS] COMMAND

Commands:
    build       Build Docker images
    deploy      Deploy to target environment
    destroy     Remove deployment
    status      Show deployment status
    logs        Show logs
    test        Run deployment tests
    upgrade     Upgrade deployment

Options:
    -t, --type      Deployment type (docker-compose|kubernetes|bare-metal)
    -e, --env       Environment (development|staging|production)
    -n, --namespace Kubernetes namespace
    -i, --image     Docker image tag
    -r, --replicas  Number of replicas
    -h, --help      Show this help

Examples:
    $0 build
    $0 deploy --type kubernetes --env production
    $0 status --type docker-compose
    $0 logs --type kubernetes --namespace nexus-prod

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -t|--type)
                DEPLOYMENT_TYPE="$2"
                shift 2
                ;;
            -e|--env)
                ENVIRONMENT="$2"
                shift 2
                ;;
            -n|--namespace)
                NAMESPACE="$2"
                shift 2
                ;;
            -i|--image)
                IMAGE_TAG="$2"
                shift 2
                ;;
            -r|--replicas)
                REPLICAS="$2"
                shift 2
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            -*)
                error "Unknown option: $1"
                usage
                exit 1
                ;;
            *)
                COMMAND="$1"
                shift
                ;;
        esac
    done
}

# Validate prerequisites
validate_prerequisites() {
    log "Validating prerequisites for $DEPLOYMENT_TYPE deployment"
    
    case $DEPLOYMENT_TYPE in
        docker-compose)
            if ! command -v docker >/dev/null 2>&1; then
                error "Docker is required but not installed"
                exit 1
            fi
            if ! command -v docker-compose >/dev/null 2>&1; then
                error "Docker Compose is required but not installed"
                exit 1
            fi
            ;;
        kubernetes)
            if ! command -v kubectl >/dev/null 2>&1; then
                error "kubectl is required but not installed"
                exit 1
            fi
            if ! kubectl cluster-info >/dev/null 2>&1; then
                error "Cannot connect to Kubernetes cluster"
                exit 1
            fi
            ;;
        bare-metal)
            if ! command -v systemctl >/dev/null 2>&1; then
                warn "systemctl not available, service management may be limited"
            fi
            ;;
    esac
}

# Build Docker images
build_images() {
    log "Building Nexus Docker images"
    
    cd "$PROJECT_ROOT"
    
    # Build main image
    docker build \
        --tag "nexus:${IMAGE_TAG}" \
        --tag "nexus:latest" \
        --target runtime \
        .
    
    # Build development image
    if [[ "$ENVIRONMENT" == "development" ]]; then
        docker build \
            --tag "nexus:dev-${IMAGE_TAG}" \
            --tag "nexus:dev" \
            --target development \
            .
    fi
    
    log "Docker images built successfully"
    docker images | grep nexus
}

# Deploy with Docker Compose
deploy_docker_compose() {
    log "Deploying with Docker Compose"
    
    cd "$PROJECT_ROOT"
    
    # Set environment variables
    export IMAGE_TAG
    export ENVIRONMENT
    export REPLICAS
    
    # Create necessary directories
    mkdir -p logs/{node-1,node-2,node-3}
    
    # Deploy
    if [[ "$ENVIRONMENT" == "development" ]]; then
        docker-compose --profile dev up -d
    else
        docker-compose up -d
    fi
    
    # Wait for services to be ready
    log "Waiting for services to be ready..."
    sleep 10
    
    # Check health
    check_docker_health
}

# Deploy to Kubernetes
deploy_kubernetes() {
    log "Deploying to Kubernetes"
    
    # Create namespace
    kubectl apply -f "$DEPLOY_DIR/kubernetes/namespace.yaml"
    
    # Apply configurations
    kubectl apply -f "$DEPLOY_DIR/kubernetes/configmap.yaml"
    
    # Deploy Nexus cluster
    envsubst < "$DEPLOY_DIR/kubernetes/nexus-cluster.yaml" | kubectl apply -f -
    
    # Wait for rollout
    kubectl rollout status statefulset/nexus-cluster -n "$NAMESPACE" --timeout=300s
    
    # Check status
    kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=nexus
    
    log "Kubernetes deployment completed"
}

# Deploy to bare metal
deploy_bare_metal() {
    log "Deploying to bare metal"
    
    # Create system user
    if ! id nexus >/dev/null 2>&1; then
        sudo useradd -r -s /bin/false -d /var/lib/nexus nexus
    fi
    
    # Create directories
    sudo mkdir -p /etc/nexus /var/lib/nexus /var/log/nexus
    sudo chown nexus:nexus /var/lib/nexus /var/log/nexus
    
    # Copy binaries
    sudo cp "$PROJECT_ROOT/target/release/nexus-"* /usr/local/bin/
    
    # Copy configuration
    sudo cp "$DEPLOY_DIR/config/nexus.toml" /etc/nexus/
    
    # Create systemd service
    sudo tee /etc/systemd/system/nexus.service > /dev/null << EOF
[Unit]
Description=Nexus Hypermesh Node
After=network.target
Wants=network.target

[Service]
Type=simple
User=nexus
Group=nexus
ExecStart=/usr/local/bin/nexus-coordinator --config /etc/nexus/nexus.toml
Restart=always
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF
    
    # Enable and start service
    sudo systemctl daemon-reload
    sudo systemctl enable nexus
    sudo systemctl start nexus
    
    log "Bare metal deployment completed"
}

# Check Docker health
check_docker_health() {
    log "Checking container health"
    
    local max_attempts=30
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        if docker-compose ps | grep -q "Up (healthy)"; then
            log "All services are healthy"
            return 0
        fi
        
        info "Health check attempt $attempt/$max_attempts"
        sleep 5
        ((attempt++))
    done
    
    warn "Some services may not be healthy"
    docker-compose ps
}

# Show deployment status
show_status() {
    case $DEPLOYMENT_TYPE in
        docker-compose)
            docker-compose ps
            echo
            docker-compose logs --tail=10
            ;;
        kubernetes)
            kubectl get all -n "$NAMESPACE"
            echo
            kubectl top pods -n "$NAMESPACE" || true
            ;;
        bare-metal)
            systemctl status nexus
            ;;
    esac
}

# Show logs
show_logs() {
    case $DEPLOYMENT_TYPE in
        docker-compose)
            docker-compose logs -f "$@"
            ;;
        kubernetes)
            kubectl logs -f -l app.kubernetes.io/name=nexus -n "$NAMESPACE" "$@"
            ;;
        bare-metal)
            journalctl -u nexus -f "$@"
            ;;
    esac
}

# Run tests
run_tests() {
    log "Running deployment tests"
    
    case $DEPLOYMENT_TYPE in
        docker-compose)
            # Test container health
            if ! docker-compose ps | grep -q "Up (healthy)"; then
                error "Containers are not healthy"
                return 1
            fi
            
            # Test API endpoints
            for port in 8080 8081 8082; do
                if ! curl -sf "http://localhost:$port/health" >/dev/null; then
                    error "Health check failed for port $port"
                    return 1
                fi
            done
            ;;
        kubernetes)
            # Test pod readiness
            if ! kubectl wait --for=condition=ready pod -l app.kubernetes.io/name=nexus -n "$NAMESPACE" --timeout=300s; then
                error "Pods are not ready"
                return 1
            fi
            
            # Test service endpoints
            local service_ip
            service_ip=$(kubectl get svc nexus-api -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
            if [[ -n "$service_ip" ]]; then
                if ! curl -sf "http://$service_ip/health" >/dev/null; then
                    warn "External health check failed"
                fi
            fi
            ;;
        bare-metal)
            if ! systemctl is-active --quiet nexus; then
                error "Nexus service is not active"
                return 1
            fi
            
            if ! curl -sf "http://localhost:8080/health" >/dev/null; then
                error "Health check failed"
                return 1
            fi
            ;;
    esac
    
    log "All tests passed"
}

# Destroy deployment
destroy_deployment() {
    warn "Destroying $DEPLOYMENT_TYPE deployment"
    read -p "Are you sure? (y/N) " -n 1 -r
    echo
    
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        info "Deployment destruction cancelled"
        return 0
    fi
    
    case $DEPLOYMENT_TYPE in
        docker-compose)
            docker-compose down -v --remove-orphans
            docker system prune -f
            ;;
        kubernetes)
            kubectl delete namespace "$NAMESPACE" --ignore-not-found
            ;;
        bare-metal)
            sudo systemctl stop nexus
            sudo systemctl disable nexus
            sudo rm -f /etc/systemd/system/nexus.service
            sudo systemctl daemon-reload
            sudo userdel nexus || true
            sudo rm -rf /var/lib/nexus /var/log/nexus
            ;;
    esac
    
    log "Deployment destroyed"
}

# Upgrade deployment
upgrade_deployment() {
    log "Upgrading $DEPLOYMENT_TYPE deployment"
    
    case $DEPLOYMENT_TYPE in
        docker-compose)
            build_images
            docker-compose pull
            docker-compose up -d --force-recreate
            ;;
        kubernetes)
            build_images
            kubectl set image statefulset/nexus-cluster nexus="nexus:${IMAGE_TAG}" -n "$NAMESPACE"
            kubectl rollout status statefulset/nexus-cluster -n "$NAMESPACE" --timeout=300s
            ;;
        bare-metal)
            sudo systemctl stop nexus
            sudo cp "$PROJECT_ROOT/target/release/nexus-"* /usr/local/bin/
            sudo systemctl start nexus
            ;;
    esac
    
    log "Upgrade completed"
}

# Main function
main() {
    local command="${1:-}"
    
    if [[ -z "$command" ]]; then
        usage
        exit 1
    fi
    
    validate_prerequisites
    
    case $command in
        build)
            build_images
            ;;
        deploy)
            case $DEPLOYMENT_TYPE in
                docker-compose)
                    build_images
                    deploy_docker_compose
                    ;;
                kubernetes)
                    build_images
                    deploy_kubernetes
                    ;;
                bare-metal)
                    deploy_bare_metal
                    ;;
                *)
                    error "Unknown deployment type: $DEPLOYMENT_TYPE"
                    exit 1
                    ;;
            esac
            ;;
        status)
            show_status
            ;;
        logs)
            shift
            show_logs "$@"
            ;;
        test)
            run_tests
            ;;
        destroy)
            destroy_deployment
            ;;
        upgrade)
            upgrade_deployment
            ;;
        *)
            error "Unknown command: $command"
            usage
            exit 1
            ;;
    esac
}

# Parse arguments and run main
parse_args "$@"
main "$COMMAND"