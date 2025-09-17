# TrustChain Infrastructure Outputs

# Network Information
output "vpc_id" {
  description = "VPC ID for TrustChain infrastructure"
  value       = module.networking.vpc_id
}

output "vpc_ipv6_cidr_block" {
  description = "IPv6 CIDR block for VPC"
  value       = module.networking.vpc_ipv6_cidr_block
}

output "public_subnet_ids" {
  description = "Public subnet IDs"
  value       = module.networking.public_subnet_ids
}

output "private_subnet_ids" {
  description = "Private subnet IDs"
  value       = module.networking.private_subnet_ids
}

# Load Balancer Information
output "load_balancer_dns_name" {
  description = "Application Load Balancer DNS name"
  value       = module.load_balancer.alb_dns_name
}

output "load_balancer_ipv6_address" {
  description = "Application Load Balancer IPv6 address"
  value       = module.load_balancer.alb_ipv6_address
}

output "load_balancer_zone_id" {
  description = "Application Load Balancer hosted zone ID"
  value       = module.load_balancer.alb_zone_id
}

# Service Endpoints
output "trustchain_endpoints" {
  description = "TrustChain service endpoints"
  value = {
    ca_api           = "https://${var.domain_name}:8443"
    certificate_transparency = "https://${var.domain_name}:6962"
    dns_over_quic    = "quic://${var.domain_name}:8853"
    stoq_protocol    = "quic://${var.domain_name}:8444"
    hypermesh_integration = "https://${var.domain_name}:8445"
    integration_api  = "https://${var.domain_name}:8446"
  }
}

# Instance Information
output "instance_ids" {
  description = "EC2 instance IDs"
  value       = module.compute.instance_ids
}

output "instance_ipv6_addresses" {
  description = "EC2 instance IPv6 addresses"
  value       = module.compute.instance_ipv6_addresses
}

output "instance_dns_names" {
  description = "EC2 instance DNS names"
  value       = module.compute.instance_dns_names
}

# Security Information
output "security_group_ids" {
  description = "Security group IDs"
  value = {
    trustchain = module.security.trustchain_security_group_id
    alb        = module.security.alb_security_group_id
    hsm        = module.hsm.hsm_security_group_id
  }
}

# Storage Information
output "s3_bucket_names" {
  description = "S3 bucket names"
  value       = module.storage.bucket_names
}

output "s3_bucket_arns" {
  description = "S3 bucket ARNs"
  value       = module.storage.bucket_arns
}

# HSM Information
output "cloudhsm_cluster_id" {
  description = "CloudHSM cluster ID"
  value       = var.enable_hsm ? module.hsm.cluster_id : null
  sensitive   = true
}

output "cloudhsm_cluster_state" {
  description = "CloudHSM cluster state"
  value       = var.enable_hsm ? module.hsm.cluster_state : null
}

# Certificate Information
output "ssl_certificate_arn" {
  description = "SSL certificate ARN for HTTPS endpoints"
  value       = module.certificates.ssl_certificate_arn
}

output "ssl_certificate_domain_validation_options" {
  description = "SSL certificate domain validation options"
  value       = module.certificates.domain_validation_options
  sensitive   = true
}

# DNS Information
output "route53_zone_id" {
  description = "Route 53 hosted zone ID"
  value       = module.dns.zone_id
}

output "route53_name_servers" {
  description = "Route 53 name servers"
  value       = module.dns.name_servers
}

# Monitoring Information
output "cloudwatch_dashboard_url" {
  description = "CloudWatch dashboard URL"
  value       = module.monitoring.dashboard_url
}

output "cloudwatch_log_groups" {
  description = "CloudWatch log group names"
  value       = module.monitoring.log_group_names
}

# Cost Information
output "estimated_monthly_cost_usd" {
  description = "Estimated monthly cost in USD"
  value = {
    compute         = 300  # c6g.4xlarge x2 instances
    load_balancer   = 25   # Application Load Balancer
    storage         = 150  # S3 + EBS storage
    hsm            = var.enable_hsm ? 1500 : 0  # CloudHSM cluster
    networking     = 75    # Data transfer and Route 53
    monitoring     = 50    # CloudWatch and X-Ray
    total          = var.enable_hsm ? 2100 : 600
  }
}

# Performance Metrics
output "performance_targets" {
  description = "Performance targets and current configuration"
  value = {
    target_response_time_ms    = var.target_response_time_ms
    target_throughput_ops_sec  = var.max_certificate_operations_per_second
    target_stoq_throughput_gbps = var.stoq_target_throughput_gbps
    instance_network_performance = "Up to 25 Gbps"
    enhanced_networking_enabled = var.enable_enhanced_networking
  }
}

# Deployment Information
output "deployment_info" {
  description = "Deployment information and next steps"
  value = {
    environment             = var.environment
    region                 = var.aws_region
    availability_zones     = local.availability_zones
    ipv6_only_networking   = true
    hsm_enabled           = var.enable_hsm
    multi_az_enabled      = var.enable_multi_az
    encryption_at_rest    = var.enable_encryption_at_rest
    encryption_in_transit = var.enable_encryption_in_transit
  }
}

# Security Configuration
output "security_configuration" {
  description = "Security configuration summary"
  value = {
    waf_enabled          = var.enable_waf
    guard_duty_enabled   = var.enable_guard_duty
    hsm_enabled         = var.enable_hsm
    ipv6_only          = true
    tls_version        = "1.3"
    certificate_rotation_hours = var.certificate_rotation_hours
  }
}

# Integration Endpoints (for other services)
output "integration_config" {
  description = "Configuration for integrating with other HyperMesh services"
  value = {
    trustchain_ca_endpoint = "https://${var.domain_name}:8443"
    certificate_transparency_endpoint = "https://${var.domain_name}:6962"
    dns_resolution_endpoint = "${var.domain_name}:8853"
    stoq_transport_endpoint = "${var.domain_name}:8444"
    consensus_proof_validation = var.consensus_proof_types
    ipv6_primary_address = module.load_balancer.alb_ipv6_address
  }
}

# Backup and Recovery
output "backup_configuration" {
  description = "Backup and disaster recovery configuration"
  value = {
    cross_region_backup_enabled = var.enable_cross_region_backup
    backup_retention_days      = var.backup_retention_days
    ct_log_retention_days     = var.certificate_transparency_retention_days
    rto_minutes              = var.rto_minutes
    rpo_minutes              = var.rpo_minutes
    backup_region           = var.backup_region
  }
}

# Health Check Endpoints
output "health_check_endpoints" {
  description = "Health check endpoints for monitoring"
  value = {
    ca_health         = "https://${var.domain_name}:8443/health"
    ct_health         = "https://${var.domain_name}:6962/ct/v1/get-sth"
    dns_health        = "udp://${var.domain_name}:8853"
    stoq_health       = "udp://${var.domain_name}:8444/health"
    hypermesh_health  = "https://${var.domain_name}:8445/health"
    integration_health = "https://${var.domain_name}:8446/health"
  }
}