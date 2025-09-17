# TrustChain Infrastructure Variables

variable "aws_region" {
  description = "AWS region for deployment"
  type        = string
  default     = "us-west-2"
}

variable "environment" {
  description = "Environment name (prod, staging, dev)"
  type        = string
  default     = "prod"
}

variable "domain_name" {
  description = "Primary domain name for TrustChain services"
  type        = string
  default     = "trust.hypermesh.online"
}

variable "instance_type" {
  description = "EC2 instance type for TrustChain services"
  type        = string
  default     = "c6g.4xlarge" # ARM-based, high performance
}

variable "key_pair_name" {
  description = "EC2 Key Pair name for SSH access"
  type        = string
  default     = "trustchain-prod"
}

variable "alert_sns_topic_arn" {
  description = "SNS topic ARN for alerts"
  type        = string
  default     = ""
}

variable "certificate_transparency_retention_days" {
  description = "Number of days to retain CT logs"
  type        = number
  default     = 2555 # 7 years for legal compliance
}

variable "backup_retention_days" {
  description = "Number of days to retain backups"
  type        = number
  default     = 30
}

variable "enable_hsm" {
  description = "Enable AWS CloudHSM for production key management"
  type        = bool
  default     = true
}

variable "enable_multi_az" {
  description = "Enable Multi-AZ deployment for high availability"
  type        = bool
  default     = true
}

variable "enable_enhanced_networking" {
  description = "Enable enhanced networking for high performance"
  type        = bool
  default     = true
}

variable "max_certificate_operations_per_second" {
  description = "Maximum certificate operations per second target"
  type        = number
  default     = 1000
}

variable "target_response_time_ms" {
  description = "Target response time in milliseconds"
  type        = number
  default     = 35
}

variable "stoq_target_throughput_gbps" {
  description = "Target STOQ throughput in Gbps"
  type        = number
  default     = 40
}

variable "allowed_ipv6_cidrs" {
  description = "List of IPv6 CIDR blocks allowed to access services"
  type        = list(string)
  default     = ["::/0"] # Allow all IPv6 for public CA
}

variable "hypermesh_admin_ipv6_cidrs" {
  description = "IPv6 CIDR blocks for HyperMesh administrators"
  type        = list(string)
  default     = []
}

# Cost optimization variables
variable "enable_spot_instances" {
  description = "Use spot instances for cost optimization (non-critical workloads only)"
  type        = bool
  default     = false
}

variable "enable_reserved_instances" {
  description = "Enable reserved instance planning"
  type        = bool
  default     = true
}

# Performance tuning variables
variable "ebs_volume_size_gb" {
  description = "EBS volume size in GB"
  type        = number
  default     = 100
}

variable "ebs_volume_type" {
  description = "EBS volume type"
  type        = string
  default     = "gp3"
}

variable "ebs_iops" {
  description = "EBS IOPS for high-performance storage"
  type        = number
  default     = 3000
}

variable "ebs_throughput_mbps" {
  description = "EBS throughput in MB/s"
  type        = number
  default     = 125
}

# Security variables
variable "enable_encryption_at_rest" {
  description = "Enable encryption at rest for all storage"
  type        = bool
  default     = true
}

variable "enable_encryption_in_transit" {
  description = "Enforce encryption in transit"
  type        = bool
  default     = true
}

variable "enable_waf" {
  description = "Enable AWS WAF for additional protection"
  type        = bool
  default     = true
}

variable "enable_guard_duty" {
  description = "Enable AWS GuardDuty for threat detection"
  type        = bool
  default     = true
}

# Monitoring variables
variable "cloudwatch_log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 90
}

variable "enable_detailed_monitoring" {
  description = "Enable detailed CloudWatch monitoring"
  type        = bool
  default     = true
}

variable "enable_x_ray_tracing" {
  description = "Enable AWS X-Ray distributed tracing"
  type        = bool
  default     = true
}

# Network performance variables
variable "placement_group_strategy" {
  description = "EC2 placement group strategy for network performance"
  type        = string
  default     = "cluster"
}

variable "enable_sr_iov" {
  description = "Enable SR-IOV for enhanced networking"
  type        = bool
  default     = true
}

variable "enable_ena_support" {
  description = "Enable Elastic Network Adapter support"
  type        = bool
  default     = true
}

# TrustChain specific variables
variable "consensus_proof_types" {
  description = "Required consensus proof types for NKrypt integration"
  type        = list(string)
  default     = ["PoSpace", "PoStake", "PoWork", "PoTime"]
}

variable "certificate_rotation_hours" {
  description = "Automatic certificate rotation interval in hours"
  type        = number
  default     = 24
}

variable "ct_log_max_entries_per_shard" {
  description = "Maximum entries per CT log shard"
  type        = number
  default     = 1000000
}

variable "dns_over_quic_enabled" {
  description = "Enable DNS-over-QUIC functionality"
  type        = bool
  default     = true
}

variable "hypermesh_integration_enabled" {
  description = "Enable HyperMesh blockchain integration"
  type        = bool
  default     = true
}

# Disaster recovery variables
variable "enable_cross_region_backup" {
  description = "Enable cross-region backup for disaster recovery"
  type        = bool
  default     = true
}

variable "backup_region" {
  description = "Secondary region for disaster recovery"
  type        = string
  default     = "us-east-1"
}

variable "rto_minutes" {
  description = "Recovery Time Objective in minutes"
  type        = number
  default     = 30
}

variable "rpo_minutes" {
  description = "Recovery Point Objective in minutes"
  type        = number
  default     = 5
}