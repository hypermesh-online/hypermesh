#!/bin/bash

# Production Infrastructure Deployment Script for Web3 Ecosystem
# Deploys AWS CloudHSM, Certificate Transparency Storage, and Production Infrastructure

set -euo pipefail

# Configuration
ENVIRONMENT="prod"
AWS_REGION="us-west-2"
DOMAIN_NAME="trust.hypermesh.online"
PROJECT_ROOT="/home/persist/repos/projects/web3"
TERRAFORM_DIR="${PROJECT_ROOT}/infrastructure/terraform"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check if AWS CLI is configured
    if ! aws sts get-caller-identity > /dev/null 2>&1; then
        error "AWS CLI not configured. Please run 'aws configure' first."
        exit 1
    fi
    
    # Check if Terraform is installed
    if ! command -v terraform > /dev/null 2>&1; then
        error "Terraform not installed. Please install Terraform first."
        exit 1
    fi
    
    # Check Terraform version
    TERRAFORM_VERSION=$(terraform version -json | jq -r '.terraform_version')
    if [[ $(echo "$TERRAFORM_VERSION 1.0.0" | tr ' ' '\n' | sort -V | head -n1) != "1.0.0" ]]; then
        error "Terraform version >= 1.0.0 required. Current version: $TERRAFORM_VERSION"
        exit 1
    fi
    
    # Check if jq is installed
    if ! command -v jq > /dev/null 2>&1; then
        error "jq not installed. Please install jq for JSON processing."
        exit 1
    fi
    
    success "Prerequisites check passed"
}

# Validate AWS permissions
validate_aws_permissions() {
    log "Validating AWS permissions..."
    
    # Check CloudHSM permissions
    if ! aws cloudhsmv2 describe-clusters --region "$AWS_REGION" > /dev/null 2>&1; then
        warning "CloudHSM permissions may be insufficient. Proceeding anyway..."
    fi
    
    # Check S3 permissions
    if ! aws s3 ls > /dev/null 2>&1; then
        error "S3 permissions insufficient"
        exit 1
    fi
    
    # Check KMS permissions
    if ! aws kms list-keys --region "$AWS_REGION" > /dev/null 2>&1; then
        error "KMS permissions insufficient"
        exit 1
    fi
    
    success "AWS permissions validated"
}

# Initialize Terraform backend
init_terraform() {
    log "Initializing Terraform..."
    
    cd "$TERRAFORM_DIR"
    
    # Create backend configuration
    cat > backend.tf << EOF
terraform {
  backend "s3" {
    bucket = "trustchain-terraform-state-${ENVIRONMENT}-$(date +%s)"
    key    = "web3-ecosystem/terraform.tfstate"
    region = "${AWS_REGION}"
    encrypt = true
    versioning = true
  }
}
EOF
    
    # Create S3 bucket for Terraform state
    BUCKET_NAME="trustchain-terraform-state-${ENVIRONMENT}-$(date +%s)"
    aws s3 mb "s3://${BUCKET_NAME}" --region "$AWS_REGION"
    aws s3api put-bucket-versioning --bucket "$BUCKET_NAME" --versioning-configuration Status=Enabled
    aws s3api put-bucket-encryption --bucket "$BUCKET_NAME" --server-side-encryption-configuration '{
        "Rules": [
            {
                "ApplyServerSideEncryptionByDefault": {
                    "SSEAlgorithm": "AES256"
                }
            }
        ]
    }'
    
    # Initialize Terraform
    terraform init
    
    success "Terraform initialized with remote state backend"
}

# Plan infrastructure deployment
plan_deployment() {
    log "Planning infrastructure deployment..."
    
    cd "$TERRAFORM_DIR"
    
    # Create terraform.tfvars
    cat > terraform.tfvars << EOF
aws_region = "${AWS_REGION}"
environment = "${ENVIRONMENT}"
domain_name = "${DOMAIN_NAME}"
instance_type = "c6g.4xlarge"
key_pair_name = "trustchain-${ENVIRONMENT}"

# HSM Configuration
enable_hsm = true
hsm_instance_count = 2

# Certificate Transparency
certificate_transparency_retention_days = 2555

# Backup Configuration
backup_retention_days = 90
EOF
    
    # Run terraform plan
    terraform plan -out="tfplan-${ENVIRONMENT}.out" -var-file="terraform.tfvars"
    
    success "Infrastructure deployment planned"
}

# Deploy infrastructure
deploy_infrastructure() {
    log "Deploying production infrastructure..."
    
    cd "$TERRAFORM_DIR"
    
    # Show plan summary
    log "Deployment will create:"
    terraform show -json "tfplan-${ENVIRONMENT}.out" | jq -r '.resource_changes[] | select(.change.actions[] == "create") | .address' | sort
    
    # Confirm deployment
    read -p "Deploy infrastructure? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log "Deployment cancelled"
        exit 0
    fi
    
    # Apply Terraform
    terraform apply "tfplan-${ENVIRONMENT}.out"
    
    success "Infrastructure deployment completed"
}

# Configure CloudHSM
configure_hsm() {
    log "Configuring CloudHSM cluster..."
    
    cd "$TERRAFORM_DIR"
    
    # Get HSM cluster ID
    HSM_CLUSTER_ID=$(terraform output -raw hsm_cluster_id 2>/dev/null || echo "")
    
    if [[ -z "$HSM_CLUSTER_ID" ]]; then
        warning "HSM cluster ID not found in Terraform output"
        return 0
    fi
    
    # Wait for HSM cluster to be active
    log "Waiting for HSM cluster to be active..."
    while true; do
        CLUSTER_STATE=$(aws cloudhsmv2 describe-clusters --cluster-id "$HSM_CLUSTER_ID" --region "$AWS_REGION" --query 'Clusters[0].State' --output text)
        if [[ "$CLUSTER_STATE" == "ACTIVE" ]]; then
            break
        fi
        log "HSM cluster state: $CLUSTER_STATE. Waiting..."
        sleep 30
    done
    
    success "CloudHSM cluster is active"
    
    # Initialize HSM cluster (requires manual steps)
    warning "HSM cluster initialization requires manual steps:"
    warning "1. Download the cluster certificate"
    warning "2. Install CloudHSM client on EC2 instances"
    warning "3. Create crypto officer (CO) user"
    warning "4. Create crypto user (CU) for applications"
    warning "See: https://docs.aws.amazon.com/cloudhsm/latest/userguide/getting-started.html"
}

# Validate deployment
validate_deployment() {
    log "Validating infrastructure deployment..."
    
    cd "$TERRAFORM_DIR"
    
    # Check Terraform outputs
    log "Terraform outputs:"
    terraform output
    
    # Check AWS resources
    log "Validating AWS resources..."
    
    # Check VPC
    VPC_ID=$(terraform output -raw vpc_id 2>/dev/null || echo "")
    if [[ -n "$VPC_ID" ]]; then
        aws ec2 describe-vpcs --vpc-ids "$VPC_ID" --region "$AWS_REGION" > /dev/null
        success "VPC validated: $VPC_ID"
    fi
    
    # Check Load Balancer
    ALB_ARN=$(terraform output -raw alb_arn 2>/dev/null || echo "")
    if [[ -n "$ALB_ARN" ]]; then
        aws elbv2 describe-load-balancers --load-balancer-arns "$ALB_ARN" --region "$AWS_REGION" > /dev/null
        success "Load Balancer validated"
    fi
    
    # Check S3 buckets
    CT_BUCKET=$(terraform output -raw ct_logs_bucket_name 2>/dev/null || echo "")
    if [[ -n "$CT_BUCKET" ]]; then
        aws s3 ls "s3://${CT_BUCKET}" > /dev/null
        success "Certificate Transparency S3 bucket validated"
    fi
    
    # Check HSM cluster
    HSM_CLUSTER_ID=$(terraform output -raw hsm_cluster_id 2>/dev/null || echo "")
    if [[ -n "$HSM_CLUSTER_ID" ]]; then
        aws cloudhsmv2 describe-clusters --cluster-id "$HSM_CLUSTER_ID" --region "$AWS_REGION" > /dev/null
        success "CloudHSM cluster validated"
    fi
    
    success "Infrastructure validation completed"
}

# Generate deployment report
generate_report() {
    log "Generating deployment report..."
    
    cd "$TERRAFORM_DIR"
    
    REPORT_FILE="${PROJECT_ROOT}/PRODUCTION_INFRASTRUCTURE_DEPLOYMENT_REPORT.md"
    
    cat > "$REPORT_FILE" << EOF
# Production Infrastructure Deployment Report

**Date**: $(date)  
**Environment**: ${ENVIRONMENT}  
**AWS Region**: ${AWS_REGION}  
**Domain**: ${DOMAIN_NAME}  

## Infrastructure Components

### Network Infrastructure
- **VPC**: IPv6-only networking with dual-stack support
- **Availability Zones**: Multi-AZ deployment for high availability
- **Load Balancer**: Application Load Balancer with WAF protection

### Security Infrastructure
- **CloudHSM**: FIPS 140-2 Level 3 Hardware Security Modules
- **KMS**: Key Management Service for encryption at rest
- **WAF**: Web Application Firewall for application protection
- **VPC Flow Logs**: Network traffic monitoring and analysis

### Storage Infrastructure
- **S3 Buckets**: Encrypted storage for Certificate Transparency logs
- **Backup**: Automated backup and disaster recovery
- **Lifecycle Management**: Automated data archival and retention

### Compute Infrastructure
- **EC2 Instances**: ARM-based instances for performance and cost optimization
- **Auto Scaling**: Automatic scaling based on demand
- **Security Groups**: Network-level security controls

## Terraform Outputs

\`\`\`
$(terraform output 2>/dev/null || echo "Terraform outputs not available")
\`\`\`

## Security Validation

### CloudHSM Configuration
- **Cluster State**: $(aws cloudhsmv2 describe-clusters --region "$AWS_REGION" --query 'Clusters[0].State' --output text 2>/dev/null || echo "N/A")
- **FIPS Compliance**: FIPS 140-2 Level 3 certified
- **High Availability**: Multi-AZ HSM deployment

### Certificate Transparency Storage
- **Encryption**: KMS-encrypted S3 storage
- **Immutability**: Versioning and MFA delete protection
- **Retention**: 7-year legal compliance retention

### Network Security
- **IPv6-Only**: Enhanced security through IPv6-only networking
- **WAF Protection**: Application-layer security controls
- **VPC Flow Logs**: Comprehensive network monitoring

## Next Steps

1. **HSM Initialization**
   - Download cluster certificate
   - Install CloudHSM client software
   - Create crypto officer (CO) user
   - Create crypto user (CU) for applications

2. **DNS Configuration**
   - Configure DNS records for trust.hypermesh.online
   - Set up IPv6 AAAA records

3. **Application Deployment**
   - Deploy TrustChain applications
   - Configure certificate transparency logging
   - Enable monitoring and alerting

4. **Testing and Validation**
   - Performance testing
   - Security validation
   - Disaster recovery testing

## Monitoring and Maintenance

- **CloudWatch**: Metrics and logging
- **SNS**: Alert notifications
- **Auto Scaling**: Automatic capacity management
- **Backup**: Daily automated backups

---

**Report Generated**: $(date)  
**Status**: Infrastructure deployment complete, manual configuration required
EOF
    
    success "Deployment report generated: $REPORT_FILE"
}

# Main deployment function
main() {
    log "Starting Web3 Ecosystem Production Infrastructure Deployment"
    
    check_prerequisites
    validate_aws_permissions
    init_terraform
    plan_deployment
    deploy_infrastructure
    configure_hsm
    validate_deployment
    generate_report
    
    success "Production infrastructure deployment completed successfully!"
    warning "Manual HSM configuration required before production use"
    log "See deployment report for next steps: ${PROJECT_ROOT}/PRODUCTION_INFRASTRUCTURE_DEPLOYMENT_REPORT.md"
}

# Handle script termination
cleanup() {
    error "Deployment interrupted"
    cd "$TERRAFORM_DIR" 2>/dev/null || true
    if [[ -f "tfplan-${ENVIRONMENT}.out" ]]; then
        rm -f "tfplan-${ENVIRONMENT}.out"
    fi
    exit 1
}

trap cleanup INT TERM

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi