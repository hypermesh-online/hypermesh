# TrustChain AWS Infrastructure Deployment Guide

## Overview

This guide provides step-by-step instructions for deploying TrustChain's production infrastructure on AWS with IPv6-only networking, HSM security, and high-performance STOQ protocol support.

## Architecture Summary

- **IPv6-Only Networking**: Complete IPv6 infrastructure with no IPv4 support
- **AWS CloudHSM**: Hardware security modules for certificate authority keys
- **High Performance**: 40+ Gbps STOQ protocol support with enhanced networking
- **Multi-Service**: CA (8443), CT (6962), DNS (8853), STOQ (8444), HyperMesh (8445), Integration (8446)
- **Compliance**: 7-year retention, FIPS 140-2 Level 3, automated backup and monitoring

## Prerequisites

### 1. AWS Account Setup
```bash
# Verify AWS CLI is configured
aws sts get-caller-identity

# Required permissions for deployment
aws iam list-attached-user-policies --user-name YOUR_USERNAME
```

### 2. Domain Configuration
- Purchase or transfer `hypermesh.online` domain to Route 53
- Ensure you have administrative access to the domain
- Verify IPv6 support with your DNS provider

### 3. IPv6 Network Planning
```bash
# Verify IPv6 connectivity from your deployment environment
ping6 google.com

# Check AWS region IPv6 support
aws ec2 describe-regions --region us-west-2 --query 'Regions[0].OptInStatus'
```

### 4. Required Tools
```bash
# Install Terraform
curl -fsSL https://apt.releases.hashicorp.com/gpg | sudo apt-key add -
sudo apt-add-repository "deb [arch=amd64] https://apt.releases.hashicorp.com $(lsb_release -cs) main"
sudo apt-get update && sudo apt-get install terraform

# Verify installation
terraform --version  # Should be >= 1.0
aws --version        # Should be >= 2.0
```

## Quick Start Deployment

### 1. Clone and Configure
```bash
cd /home/persist/repos/projects/web3/infrastructure

# Copy example configuration
cp terraform/terraform.tfvars.example terraform/terraform.tfvars

# Edit configuration for your environment
nano terraform/terraform.tfvars
```

### 2. Essential Configuration
```hcl
# terraform/terraform.tfvars
aws_region  = "us-west-2"
environment = "prod"
domain_name = "trust.hypermesh.online"

# Critical: Replace with your actual IPv6 ranges
allowed_ipv6_cidrs = ["::/0"]  # Start permissive, lock down later
hypermesh_admin_ipv6_cidrs = ["2001:db8:admin::/48"]

# Security
enable_hsm = true  # REQUIRED for production
enable_waf = true
enable_guard_duty = true

# Performance
instance_type = "c6g.4xlarge"  # ARM-based for best price/performance
stoq_target_throughput_gbps = 40

# Alerts (create SNS topic first)
alert_sns_topic_arn = "arn:aws:sns:us-west-2:123456789012:trustchain-alerts"
```

### 3. Deploy Infrastructure
```bash
# Initialize and validate
./deploy.sh init
./deploy.sh validate

# Generate deployment plan
./deploy.sh plan

# Deploy (will prompt for confirmation)
./deploy.sh apply

# Or deploy with auto-approval
./deploy.sh apply --auto-approve
```

### 4. Verify Deployment
```bash
# Check outputs
./deploy.sh output

# Test endpoints (replace with your domain)
curl -6 https://trust.hypermesh.online:8443/health
dig AAAA trust.hypermesh.online
```

## Advanced Configuration

### Production Environment Variables
```bash
# Create .env file for production
cat > .env << EOF
export AWS_REGION="us-west-2"
export ENVIRONMENT="prod"
export DOMAIN_NAME="trust.hypermesh.online"
export TF_VAR_enable_hsm="true"
export TF_VAR_enable_detailed_monitoring="true"
EOF

source .env
```

### Multi-Environment Setup
```bash
# Staging environment
./deploy.sh apply --var-file terraform.tfvars.staging --environment staging

# Development environment  
./deploy.sh apply --var-file terraform.tfvars.dev --environment dev
```

### Targeted Deployments
```bash
# Deploy only compute resources
./deploy.sh apply --target module.compute

# Deploy only security updates
./deploy.sh apply --target module.security

# Update monitoring configuration
./deploy.sh apply --target module.monitoring
```

## Cost Management

### Estimated Monthly Costs (US West 2)

| Component | Production | Development | 
|-----------|------------|-------------|
| **Compute** | | |
| EC2 c6g.4xlarge (2x) | $300 | $150 |
| Load Balancers | $50 | $25 |
| **Storage** | | |
| S3 + EBS | $180 | $90 |
| **Security** | | |
| CloudHSM | $1,500 | $0* |
| KMS + WAF | $75 | $25 |
| **Networking** | | |
| Data Transfer | $75 | $25 |
| Route 53 | $10 | $5 |
| **Monitoring** | | |
| CloudWatch + X-Ray | $75 | $25 |
| **TOTAL** | **$2,265** | **$345** |

*Development uses software-based key management

### Cost Optimization
```bash
# Enable cost optimization features
export TF_VAR_enable_spot_instances="true"     # Dev only
export TF_VAR_enable_reserved_instances="true" # Production

# Deploy with cost optimizations
./deploy.sh apply
```

### Monthly Cost Monitoring
```bash
# Get current month estimate
aws ce get-cost-and-usage \
  --time-period Start=2024-01-01,End=2024-01-31 \
  --granularity MONTHLY \
  --metrics UnblendedCost \
  --group-by Type=DIMENSION,Key=SERVICE
```

## Security Configuration

### HSM Setup (Production Only)
```bash
# Verify HSM cluster is active
aws cloudhsmv2 describe-clusters --region us-west-2

# Initialize HSM (run once)
# This requires manual setup - see AWS CloudHSM documentation
```

### IPv6 Security Hardening
```hcl
# Restrict to your organization's IPv6 ranges
allowed_ipv6_cidrs = [
  "2001:db8:corp::/48",    # Corporate network
  "2001:db8:remote::/48"   # Remote access range
]

# Admin access (SSH, management)
hypermesh_admin_ipv6_cidrs = [
  "2001:db8:admin::/48"    # Administrative network only
]
```

### Certificate Management
```bash
# Verify certificate auto-renewal
aws acm describe-certificate --certificate-arn $(terraform output -raw ssl_certificate_arn)

# Check certificate transparency logs
aws logs describe-log-groups --log-group-name-prefix "/aws/lambda/trustchain-cert"
```

## Monitoring and Alerting

### CloudWatch Dashboard
Access the monitoring dashboard:
```bash
# Get dashboard URL
./deploy.sh output | grep dashboard_url
```

### Custom Alerts Setup
```bash
# Create SNS topic for alerts
aws sns create-topic --name trustchain-alerts-prod --region us-west-2

# Subscribe email to alerts
aws sns subscribe \
  --topic-arn arn:aws:sns:us-west-2:123456789012:trustchain-alerts-prod \
  --protocol email \
  --notification-endpoint admin@hypermesh.online
```

### Performance Monitoring
```bash
# Check certificate operation latency
aws cloudwatch get-metric-statistics \
  --namespace TrustChain/Performance \
  --metric-name CertificateOperationLatency \
  --start-time 2024-01-01T00:00:00Z \
  --end-time 2024-01-02T00:00:00Z \
  --period 3600 \
  --statistics Average
```

## Troubleshooting

### Common Issues

1. **IPv6 Connectivity Problems**
```bash
# Test IPv6 connectivity
ping6 -c 4 2001:4860:4860::8888

# Check security group rules
aws ec2 describe-security-groups --filters Name=group-name,Values=trustchain-*
```

2. **Certificate Validation Failures**
```bash
# Check DNS validation records
dig TXT _acme-challenge.trust.hypermesh.online

# Verify certificate status
aws acm describe-certificate --certificate-arn YOUR_CERT_ARN
```

3. **HSM Cluster Issues**
```bash
# Check HSM cluster state
aws cloudhsmv2 describe-clusters

# Review HSM logs
aws logs describe-log-groups --log-group-name-prefix "/aws/cloudhsm"
```

4. **High Costs**
```bash
# Identify cost drivers
aws ce get-dimension-values \
  --dimension SERVICE \
  --time-period Start=2024-01-01,End=2024-01-31

# Review instance utilization
aws cloudwatch get-metric-statistics \
  --namespace AWS/EC2 \
  --metric-name CPUUtilization \
  --dimensions Name=AutoScalingGroupName,Value=trustchain-asg-prod
```

### Debug Mode
```bash
# Enable Terraform debug logging
export TF_LOG=DEBUG
export TF_LOG_PATH="./terraform-debug.log"

# Run with debug output
./deploy.sh plan
```

### Recovery Procedures

1. **Disaster Recovery**
```bash
# Restore from backup (if configured)
aws backup start-restore-job \
  --recovery-point-arn YOUR_RECOVERY_POINT_ARN \
  --iam-role-arn YOUR_BACKUP_ROLE_ARN
```

2. **Rollback Deployment**
```bash
# Revert to previous configuration
git checkout HEAD~1 terraform/
./deploy.sh apply
```

## Maintenance

### Regular Tasks

1. **Monthly Cost Review**
```bash
# Generate cost report
./deploy.sh cost

# Check for unused resources
aws support describe-trusted-advisor-checks \
  --language en \
  --check-ids pt9TAmKfVEJVUVAiKqtQjjY3Gvj
```

2. **Security Updates**
```bash
# Update AMI IDs for latest security patches
aws ec2 describe-images \
  --owners 099720109477 \
  --filters Name=name,Values="ubuntu/images/hvm-ssd/ubuntu-22.04-arm64-server-*" \
  --query 'Images | sort_by(@, &CreationDate) | [-1].ImageId'

# Apply updates
./deploy.sh apply
```

3. **Certificate Renewal**
```bash
# Check certificate expiration
aws acm list-certificates --certificate-statuses ISSUED \
  --query 'CertificateSummaryList[?DomainName==`trust.hypermesh.online`]'

# Force renewal (if needed)
aws acm request-certificate \
  --domain-name trust.hypermesh.online \
  --validation-method DNS
```

## Support and Documentation

### Additional Resources
- [AWS CloudHSM User Guide](https://docs.aws.amazon.com/cloudhsm/)
- [IPv6 on AWS](https://docs.aws.amazon.com/vpc/latest/userguide/vpc-ipv6.html)
- [Terraform AWS Provider](https://registry.terraform.io/providers/hashicorp/aws/latest/docs)

### Getting Help
- Review CloudWatch logs for application-specific issues
- Check AWS Service Health Dashboard for regional issues
- Consult Terraform state for resource dependencies

### Emergency Contacts
- AWS Support: Use AWS Support Center
- Infrastructure Team: admin@hypermesh.online
- Security Team: security@hypermesh.online