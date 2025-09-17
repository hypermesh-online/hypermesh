# Load Balancer Module Variables

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID"
  type        = string
}

variable "subnet_ids" {
  description = "Subnet IDs for load balancer"
  type        = list(string)
}

variable "security_group_id" {
  description = "Security group ID for load balancer"
  type        = string
}

variable "instance_ids" {
  description = "Instance IDs to attach to target groups"
  type        = list(string)
  default     = []
}

variable "autoscaling_group_name" {
  description = "Auto Scaling Group name for target group attachment"
  type        = string
  default     = ""
}

variable "ssl_certificate_arn" {
  description = "SSL certificate ARN for HTTPS listeners"
  type        = string
}

variable "waf_web_acl_arn" {
  description = "WAF Web ACL ARN"
  type        = string
  default     = ""
}

variable "services" {
  description = "Service configurations"
  type = map(object({
    port        = number
    protocol    = string
    health_path = optional(string)
    description = optional(string)
  }))
}