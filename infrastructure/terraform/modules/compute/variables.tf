# Compute Module Variables

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID"
  type        = string
}

variable "subnet_ids" {
  description = "Subnet IDs for instances"
  type        = list(string)
}

variable "security_group_id" {
  description = "Security group ID for instances"
  type        = string
}

variable "key_pair_name" {
  description = "EC2 Key Pair name"
  type        = string
}

variable "instance_type" {
  description = "EC2 instance type"
  type        = string
  default     = "c6g.4xlarge"
}

variable "ami_id" {
  description = "AMI ID for instances"
  type        = string
}

variable "min_instances" {
  description = "Minimum number of instances"
  type        = number
  default     = 2
}

variable "max_instances" {
  description = "Maximum number of instances"
  type        = number
  default     = 6
}

variable "desired_instances" {
  description = "Desired number of instances"
  type        = number
  default     = 2
}

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
  description = "EBS IOPS"
  type        = number
  default     = 3000
}

variable "ebs_throughput_mbps" {
  description = "EBS throughput in MB/s"
  type        = number
  default     = 125
}

variable "max_certificate_operations_per_minute" {
  description = "Maximum certificate operations per minute before scaling"
  type        = number
  default     = 60000 # 1000 ops/sec * 60 seconds
}

variable "domain_name" {
  description = "Domain name for TrustChain"
  type        = string
  default     = "trust.hypermesh.online"
}

variable "consensus_proof_types" {
  description = "Consensus proof types for NKrypt integration"
  type        = list(string)
  default     = ["PoSpace", "PoStake", "PoWork", "PoTime"]
}

variable "certificate_rotation_hours" {
  description = "Certificate rotation interval in hours"
  type        = number
  default     = 24
}

variable "target_response_time_ms" {
  description = "Target response time in milliseconds"
  type        = number
  default     = 35
}