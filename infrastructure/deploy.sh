#!/bin/bash
# TrustChain Infrastructure Deployment Script
# Deploys IPv6-only AWS infrastructure for TrustChain Certificate Authority

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TERRAFORM_DIR="$SCRIPT_DIR/terraform"
ENVIRONMENT="${ENVIRONMENT:-prod}"
AWS_REGION="${AWS_REGION:-us-west-2}"
DOMAIN_NAME="${DOMAIN_NAME:-trust.hypermesh.online}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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
    echo -e "${RED}[ERROR]${NC} $1"
}

# Help function
show_help() {
    cat << EOF
TrustChain Infrastructure Deployment Script

Usage: $0 [COMMAND] [OPTIONS]

Commands:
    plan        Generate and show Terraform execution plan
    apply       Deploy infrastructure to AWS
    destroy     Destroy all infrastructure (USE WITH CAUTION)
    validate    Validate Terraform configuration
    init        Initialize Terraform
    output      Show Terraform outputs
    cost        Estimate monthly costs
    help        Show this help message

Options:
    --environment ENV    Set environment (default: prod)
    --region REGION      Set AWS region (default: us-west-2)
    --domain DOMAIN      Set domain name (default: trust.hypermesh.online)
    --auto-approve       Skip interactive approval for apply/destroy
    --var-file FILE      Use custom variables file
    --target RESOURCE    Target specific resource for operation

Examples:
    $0 plan --environment staging
    $0 apply --var-file terraform.tfvars.staging
    $0 destroy --target module.compute --auto-approve
    $0 output
    $0 cost

Prerequisites:
    - AWS CLI configured with appropriate credentials
    - Terraform >= 1.0 installed
    - Domain name configured in Route 53 (if using DNS module)
    - SNS topic created for alerts (optional)

EOF
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check AWS CLI
    if ! command -v aws &> /dev/null; then
        log_error "AWS CLI is not installed. Please install it first."
        exit 1
    fi
    
    # Check Terraform
    if ! command -v terraform &> /dev/null; then
        log_error "Terraform is not installed. Please install it first."
        exit 1
    fi
    
    # Check AWS credentials
    if ! aws sts get-caller-identity &> /dev/null; then
        log_error "AWS credentials not configured. Please run 'aws configure' first."
        exit 1
    fi
    
    # Check Terraform directory
    if [ ! -d "$TERRAFORM_DIR" ]; then
        log_error "Terraform directory not found: $TERRAFORM_DIR"
        exit 1
    fi
    
    # Check terraform.tfvars
    if [ ! -f "$TERRAFORM_DIR/terraform.tfvars" ] && [ ! -f "$TERRAFORM_DIR/terraform.tfvars.json" ]; then
        log_warning "terraform.tfvars not found. Using default values or environment variables."
        if [ -f "$TERRAFORM_DIR/terraform.tfvars.example" ]; then
            log_info "Example configuration available at: $TERRAFORM_DIR/terraform.tfvars.example"
        fi
    fi
    
    log_success "Prerequisites check completed"
}

# Initialize Terraform
terraform_init() {
    log_info "Initializing Terraform..."
    cd "$TERRAFORM_DIR"
    
    terraform init \
        -upgrade \
        -reconfigure
    
    log_success "Terraform initialized"
}

# Validate Terraform configuration
terraform_validate() {
    log_info "Validating Terraform configuration..."
    cd "$TERRAFORM_DIR"
    
    terraform validate
    terraform fmt -check -recursive
    
    log_success "Terraform configuration is valid"
}

# Generate Terraform plan
terraform_plan() {
    local var_file=""
    local target=""
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --var-file)
                var_file="$2"
                shift 2
                ;;
            --target)
                target="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done
    
    log_info "Generating Terraform plan for environment: $ENVIRONMENT"
    cd "$TERRAFORM_DIR"
    
    local cmd="terraform plan"
    
    if [ -n "$var_file" ]; then
        cmd="$cmd -var-file=$var_file"
    fi
    
    if [ -n "$target" ]; then
        cmd="$cmd -target=$target"
    fi
    
    cmd="$cmd -var environment=$ENVIRONMENT"
    cmd="$cmd -var aws_region=$AWS_REGION"
    cmd="$cmd -var domain_name=$DOMAIN_NAME"
    cmd="$cmd -out=tfplan"
    
    eval $cmd
    
    log_success "Terraform plan generated and saved to tfplan"
}

# Apply Terraform configuration
terraform_apply() {
    local auto_approve=""
    local var_file=""
    local target=""
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --auto-approve)
                auto_approve="-auto-approve"
                shift
                ;;
            --var-file)
                var_file="$2"
                shift 2
                ;;
            --target)
                target="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done
    
    log_info "Deploying TrustChain infrastructure to AWS..."
    log_info "Environment: $ENVIRONMENT"
    log_info "Region: $AWS_REGION"
    log_info "Domain: $DOMAIN_NAME"
    
    cd "$TERRAFORM_DIR"
    
    # Generate plan first
    terraform_plan --var-file="$var_file" --target="$target"
    
    # Show plan summary
    log_info "Terraform plan summary:"
    terraform show -no-color tfplan | grep -E "^(Plan:|# |~)"
    
    # Confirm deployment
    if [ -z "$auto_approve" ]; then
        echo
        log_warning "This will deploy TrustChain infrastructure to AWS."
        log_warning "Estimated monthly cost: \$2,100 (with HSM) or \$600 (without HSM)"
        echo
        read -p "Do you want to continue? (yes/no): " confirm
        if [ "$confirm" != "yes" ]; then
            log_info "Deployment cancelled"
            exit 0
        fi
    fi
    
    # Apply configuration
    log_info "Applying Terraform configuration..."
    terraform apply $auto_approve tfplan
    
    log_success "TrustChain infrastructure deployed successfully!"
    
    # Show important outputs
    log_info "Retrieving deployment information..."
    terraform output -json > deployment_outputs.json
    
    echo
    log_success "=== DEPLOYMENT COMPLETE ==="
    echo
    log_info "Domain: $(terraform output -raw trustchain_endpoints | jq -r '.ca_api')"
    log_info "Load Balancer: $(terraform output -raw load_balancer_dns_name)"
    log_info "VPC ID: $(terraform output -raw vpc_id)"
    log_info "Dashboard: $(terraform output -raw cloudwatch_dashboard_url)"
    echo
    log_info "Full deployment details saved to: deployment_outputs.json"
    echo
}

# Destroy infrastructure
terraform_destroy() {
    local auto_approve=""
    local target=""
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --auto-approve)
                auto_approve="-auto-approve"
                shift
                ;;
            --target)
                target="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done
    
    log_warning "DANGER: This will destroy TrustChain infrastructure!"
    log_warning "This action cannot be undone!"
    
    if [ -z "$auto_approve" ]; then
        echo
        read -p "Type 'destroy' to confirm: " confirm
        if [ "$confirm" != "destroy" ]; then
            log_info "Destruction cancelled"
            exit 0
        fi
    fi
    
    cd "$TERRAFORM_DIR"
    
    local cmd="terraform destroy"
    
    if [ -n "$target" ]; then
        cmd="$cmd -target=$target"
    fi
    
    cmd="$cmd -var environment=$ENVIRONMENT"
    cmd="$cmd -var aws_region=$AWS_REGION"
    cmd="$cmd -var domain_name=$DOMAIN_NAME"
    
    if [ -n "$auto_approve" ]; then
        cmd="$cmd -auto-approve"
    fi
    
    eval $cmd
    
    log_success "Infrastructure destroyed"
}

# Show Terraform outputs
terraform_output() {
    log_info "Terraform outputs:"
    cd "$TERRAFORM_DIR"
    
    if terraform state list &> /dev/null; then
        terraform output
    else
        log_warning "No Terraform state found. Run 'deploy' first."
    fi
}

# Estimate costs
estimate_costs() {
    log_info "Estimating monthly costs for TrustChain infrastructure..."
    cd "$TERRAFORM_DIR"
    
    cat << EOF

=== ESTIMATED MONTHLY COSTS (USD) ===

Compute Infrastructure:
  • EC2 c6g.4xlarge (2 instances)    \$300
  • Application Load Balancer         \$25
  • Network Load Balancer             \$25

Storage:
  • S3 (CT logs, certificates)        \$100
  • EBS volumes (high-performance)     \$80

Security:
  • AWS CloudHSM cluster              \$1,500
  • KMS operations                    \$50
  • WAF                               \$25

Networking:
  • Data transfer                     \$75
  • Route 53                          \$10

Monitoring:
  • CloudWatch                        \$50
  • X-Ray tracing                     \$25

Total with HSM:     \$2,145/month
Total without HSM:  \$645/month

Note: Costs may vary based on actual usage patterns.
CloudHSM provides FIPS 140-2 Level 3 compliance for production.

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --environment)
                ENVIRONMENT="$2"
                shift 2
                ;;
            --region)
                AWS_REGION="$2"
                shift 2
                ;;
            --domain)
                DOMAIN_NAME="$2"
                shift 2
                ;;
            *)
                break
                ;;
        esac
    done
}

# Main function
main() {
    local command="$1"
    shift
    
    parse_args "$@"
    
    case $command in
        init)
            check_prerequisites
            terraform_init
            ;;
        validate)
            check_prerequisites
            terraform_validate
            ;;
        plan)
            check_prerequisites
            terraform_plan "$@"
            ;;
        apply)
            check_prerequisites
            terraform_init
            terraform_validate
            terraform_apply "$@"
            ;;
        destroy)
            check_prerequisites
            terraform_destroy "$@"
            ;;
        output)
            terraform_output
            ;;
        cost)
            estimate_costs
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            log_error "Unknown command: $command"
            echo
            show_help
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"