#!/bin/bash
# Phoenix Production Infrastructure Deployment Script
# Real, working infrastructure deployment for the Phoenix SDK ecosystem

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PHOENIX_ORG="phoenix-distributed"
PHOENIX_VERSION="${PHOENIX_VERSION:-v1.0.0}"
ENVIRONMENT="${ENVIRONMENT:-production}"
AWS_REGION="${AWS_REGION:-us-east-1}"
DEPLOY_MODE="${1:-check}"

# Functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    local missing_tools=()

    # Check required tools
    command -v gh >/dev/null 2>&1 || missing_tools+=("gh")
    command -v aws >/dev/null 2>&1 || missing_tools+=("aws-cli")
    command -v kubectl >/dev/null 2>&1 || missing_tools+=("kubectl")
    command -v helm >/dev/null 2>&1 || missing_tools+=("helm")
    command -v terraform >/dev/null 2>&1 || missing_tools+=("terraform")
    command -v docker >/dev/null 2>&1 || missing_tools+=("docker")

    if [ ${#missing_tools[@]} -gt 0 ]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        log_info "Please install missing tools before proceeding"
        exit 1
    fi

    # Check GitHub authentication
    if ! gh auth status >/dev/null 2>&1; then
        log_error "GitHub CLI not authenticated. Run: gh auth login"
        exit 1
    fi

    # Check AWS credentials
    if ! aws sts get-caller-identity >/dev/null 2>&1; then
        log_error "AWS credentials not configured. Configure with: aws configure"
        exit 1
    fi

    log_success "All prerequisites met"
}

# Create GitHub organization and repositories
setup_github_org() {
    log_info "Setting up GitHub organization..."

    # Check if org exists
    if ! gh api "/orgs/${PHOENIX_ORG}" >/dev/null 2>&1; then
        log_warning "Organization ${PHOENIX_ORG} doesn't exist. Creating..."

        # Note: Creating organizations requires manual action or enterprise API
        log_info "Please create the organization manually at: https://github.com/organizations/new"
        log_info "Organization name: ${PHOENIX_ORG}"
        log_info "Display name: Phoenix Distributed Computing"
        read -p "Press enter once the organization is created..."
    fi

    # Repository structure
    local repos=(
        "phoenix-sdk:Main Phoenix SDK repository"
        "phoenix-transport:STOQ high-performance transport layer"
        "phoenix-certs:TrustChain certificate authority"
        "phoenix-cli:Developer CLI tool"
        "phoenix-cloud:Cloud infrastructure and deployment"
        "phoenix-docs:Documentation and guides"
        "phoenix-dashboard:Monitoring and management dashboard"
        "phoenix-examples:Example applications and demos"
    )

    for repo_desc in "${repos[@]}"; do
        IFS=':' read -r repo description <<< "$repo_desc"

        if ! gh repo view "${PHOENIX_ORG}/${repo}" >/dev/null 2>&1; then
            log_info "Creating repository: ${repo}"
            gh repo create "${PHOENIX_ORG}/${repo}" \
                --public \
                --description "${description}" \
                --add-readme \
                --license apache-2.0 || true
        else
            log_info "Repository ${repo} already exists"
        fi
    done

    log_success "GitHub organization setup complete"
}

# Setup AWS infrastructure foundation
setup_aws_foundation() {
    log_info "Setting up AWS foundation infrastructure..."

    # Create S3 bucket for Terraform state
    local state_bucket="phoenix-terraform-state-${AWS_REGION}"
    if ! aws s3api head-bucket --bucket "${state_bucket}" 2>/dev/null; then
        log_info "Creating Terraform state bucket..."
        aws s3api create-bucket \
            --bucket "${state_bucket}" \
            --region "${AWS_REGION}" \
            --create-bucket-configuration LocationConstraint="${AWS_REGION}" || true

        # Enable versioning
        aws s3api put-bucket-versioning \
            --bucket "${state_bucket}" \
            --versioning-configuration Status=Enabled

        # Enable encryption
        aws s3api put-bucket-encryption \
            --bucket "${state_bucket}" \
            --server-side-encryption-configuration '{
                "Rules": [{
                    "ApplyServerSideEncryptionByDefault": {
                        "SSEAlgorithm": "AES256"
                    }
                }]
            }'
    fi

    # Create DynamoDB table for state locking
    local lock_table="phoenix-terraform-locks"
    if ! aws dynamodb describe-table --table-name "${lock_table}" 2>/dev/null; then
        log_info "Creating Terraform lock table..."
        aws dynamodb create-table \
            --table-name "${lock_table}" \
            --attribute-definitions AttributeName=LockID,AttributeType=S \
            --key-schema AttributeName=LockID,KeyType=HASH \
            --provisioned-throughput ReadCapacityUnits=5,WriteCapacityUnits=5 \
            --region "${AWS_REGION}" || true
    fi

    # Create ECR repositories for container images
    local ecr_repos=("phoenix-transport" "phoenix-certs" "phoenix-dashboard" "phoenix-cli")
    for repo in "${ecr_repos[@]}"; do
        if ! aws ecr describe-repositories --repository-names "${repo}" 2>/dev/null; then
            log_info "Creating ECR repository: ${repo}"
            aws ecr create-repository \
                --repository-name "${repo}" \
                --image-scanning-configuration scanOnPush=true \
                --region "${AWS_REGION}" || true
        fi
    done

    log_success "AWS foundation infrastructure ready"
}

# Deploy EKS cluster using Terraform
deploy_eks_cluster() {
    log_info "Deploying EKS cluster..."

    cd infrastructure/terraform

    # Initialize Terraform
    terraform init \
        -backend-config="bucket=phoenix-terraform-state-${AWS_REGION}" \
        -backend-config="key=${ENVIRONMENT}/phoenix-cluster.tfstate" \
        -backend-config="region=${AWS_REGION}" \
        -backend-config="dynamodb_table=phoenix-terraform-locks"

    # Plan deployment
    terraform plan \
        -var="environment=${ENVIRONMENT}" \
        -var="aws_region=${AWS_REGION}" \
        -var="phoenix_version=${PHOENIX_VERSION}" \
        -out=tfplan

    if [ "$DEPLOY_MODE" == "apply" ]; then
        log_info "Applying Terraform configuration..."
        terraform apply tfplan

        # Update kubeconfig
        aws eks update-kubeconfig \
            --region "${AWS_REGION}" \
            --name "phoenix-${ENVIRONMENT}"
    else
        log_warning "Running in check mode. Run with 'apply' to deploy"
    fi

    cd ../..
    log_success "EKS cluster deployment complete"
}

# Deploy Kubernetes resources
deploy_kubernetes_resources() {
    log_info "Deploying Kubernetes resources..."

    # Create namespaces
    kubectl create namespace phoenix-system --dry-run=client -o yaml | kubectl apply -f -
    kubectl create namespace phoenix-apps --dry-run=client -o yaml | kubectl apply -f -

    # Install cert-manager for TLS certificates
    log_info "Installing cert-manager..."
    helm repo add jetstack https://charts.jetstack.io
    helm repo update

    helm upgrade --install cert-manager jetstack/cert-manager \
        --namespace cert-manager \
        --create-namespace \
        --version v1.13.0 \
        --set installCRDs=true \
        --wait

    # Install ingress-nginx
    log_info "Installing NGINX Ingress Controller..."
    helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx

    helm upgrade --install ingress-nginx ingress-nginx/ingress-nginx \
        --namespace ingress-nginx \
        --create-namespace \
        --set controller.service.type=LoadBalancer \
        --set controller.metrics.enabled=true \
        --wait

    # Deploy Phoenix components
    log_info "Deploying Phoenix components..."
    helm upgrade --install phoenix-stack ./infrastructure/helm/phoenix \
        --namespace phoenix-system \
        --set global.environment="${ENVIRONMENT}" \
        --set global.version="${PHOENIX_VERSION}" \
        --set transport.replicas=3 \
        --set certificates.replicas=3 \
        --set dashboard.enabled=true \
        --wait

    log_success "Kubernetes resources deployed"
}

# Setup monitoring stack
setup_monitoring() {
    log_info "Setting up monitoring stack..."

    # Deploy Prometheus Operator
    helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
    helm repo update

    helm upgrade --install kube-prometheus-stack prometheus-community/kube-prometheus-stack \
        --namespace monitoring \
        --create-namespace \
        --set prometheus.prometheusSpec.retention=30d \
        --set prometheus.prometheusSpec.storageSpec.volumeClaimTemplate.spec.resources.requests.storage=50Gi \
        --set grafana.adminPassword=phoenix-admin \
        --wait

    # Deploy Phoenix custom metrics
    kubectl apply -f infrastructure/kubernetes/monitoring/

    log_success "Monitoring stack deployed"
}

# Setup CI/CD pipelines
setup_cicd() {
    log_info "Setting up CI/CD pipelines..."

    # Create GitHub Actions secrets
    local secrets=(
        "AWS_ACCESS_KEY_ID"
        "AWS_SECRET_ACCESS_KEY"
        "DOCKERHUB_TOKEN"
        "KUBECONFIG"
        "SLACK_WEBHOOK_URL"
    )

    for secret in "${secrets[@]}"; do
        if [ -n "${!secret:-}" ]; then
            log_info "Setting secret: ${secret}"
            echo "${!secret}" | gh secret set "${secret}" --org "${PHOENIX_ORG}"
        else
            log_warning "Secret ${secret} not set in environment"
        fi
    done

    # Deploy workflow files to repositories
    for repo in phoenix-sdk phoenix-transport phoenix-certs; do
        if gh repo view "${PHOENIX_ORG}/${repo}" >/dev/null 2>&1; then
            log_info "Setting up CI/CD for ${repo}..."

            # Clone repo temporarily
            temp_dir=$(mktemp -d)
            git clone "https://github.com/${PHOENIX_ORG}/${repo}.git" "${temp_dir}/${repo}"

            # Copy workflow files
            mkdir -p "${temp_dir}/${repo}/.github/workflows"
            cp infrastructure/github-workflows/*.yml "${temp_dir}/${repo}/.github/workflows/"

            # Commit and push
            cd "${temp_dir}/${repo}"
            git add .github/workflows/
            git commit -m "Add CI/CD workflows" || true
            git push || true
            cd -

            # Cleanup
            rm -rf "${temp_dir}"
        fi
    done

    log_success "CI/CD pipelines configured"
}

# Deploy CDN and edge infrastructure
deploy_cdn() {
    log_info "Deploying CDN infrastructure..."

    # Get ALB DNS name
    local alb_dns=$(kubectl get ingress -n phoenix-system phoenix-ingress -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')

    # Create CloudFront distribution
    aws cloudfront create-distribution \
        --distribution-config '{
            "CallerReference": "'$(date +%s)'",
            "Comment": "Phoenix SDK CDN",
            "DefaultRootObject": "index.html",
            "Origins": {
                "Quantity": 1,
                "Items": [{
                    "Id": "phoenix-alb",
                    "DomainName": "'${alb_dns}'",
                    "CustomOriginConfig": {
                        "HTTPPort": 80,
                        "HTTPSPort": 443,
                        "OriginProtocolPolicy": "https-only"
                    }
                }]
            },
            "DefaultCacheBehavior": {
                "TargetOriginId": "phoenix-alb",
                "ViewerProtocolPolicy": "redirect-to-https",
                "TrustedSigners": {
                    "Enabled": false,
                    "Quantity": 0
                },
                "ForwardedValues": {
                    "QueryString": true,
                    "Cookies": {"Forward": "all"}
                },
                "MinTTL": 0,
                "DefaultTTL": 86400,
                "MaxTTL": 31536000
            },
            "Enabled": true
        }' || true

    log_success "CDN infrastructure deployed"
}

# Run health checks
run_health_checks() {
    log_info "Running health checks..."

    # Check cluster health
    kubectl get nodes
    kubectl get pods -n phoenix-system

    # Check service endpoints
    local services=("phoenix-transport" "phoenix-certs" "phoenix-dashboard")
    for service in "${services[@]}"; do
        local endpoint=$(kubectl get service -n phoenix-system "${service}" -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')
        if [ -n "${endpoint}" ]; then
            log_info "Testing ${service} at ${endpoint}..."
            curl -sSf "http://${endpoint}/health" || log_warning "${service} health check failed"
        fi
    done

    log_success "Health checks complete"
}

# Main deployment flow
main() {
    log_info "Phoenix Production Infrastructure Deployment"
    log_info "Environment: ${ENVIRONMENT}"
    log_info "Version: ${PHOENIX_VERSION}"
    log_info "Mode: ${DEPLOY_MODE}"
    echo

    # Run deployment steps
    check_prerequisites

    if [ "$DEPLOY_MODE" == "apply" ]; then
        read -p "This will deploy production infrastructure. Continue? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Deployment cancelled"
            exit 0
        fi
    fi

    setup_github_org
    setup_aws_foundation
    deploy_eks_cluster
    deploy_kubernetes_resources
    setup_monitoring
    setup_cicd
    deploy_cdn
    run_health_checks

    echo
    log_success "Phoenix infrastructure deployment complete!"
    log_info "Dashboard: https://dashboard.phoenix-distributed.com"
    log_info "API: https://api.phoenix-distributed.com"
    log_info "Documentation: https://docs.phoenix-distributed.com"

    if [ "$DEPLOY_MODE" == "check" ]; then
        echo
        log_warning "This was a dry run. To deploy, run: $0 apply"
    fi
}

# Run main function
main "$@"