# DNS Module Variables

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "domain_name" {
  description = "Primary domain name"
  type        = string
}

variable "create_hosted_zone" {
  description = "Create new Route 53 hosted zone"
  type        = bool
  default     = true
}

variable "load_balancer_dns_name" {
  description = "Load balancer DNS name for alias records"
  type        = string
}

variable "load_balancer_zone_id" {
  description = "Load balancer hosted zone ID"
  type        = string
}

variable "enable_dnssec" {
  description = "Enable DNSSEC for the hosted zone"
  type        = bool
  default     = false # Disabled by default due to complexity
}

variable "enable_dns_health_checks" {
  description = "Enable DNS health checks"
  type        = bool
  default     = true
}

variable "enable_query_logging" {
  description = "Enable DNS query logging"
  type        = bool
  default     = true
}

variable "sns_topic_arn" {
  description = "SNS topic ARN for DNS alerts"
  type        = string
  default     = ""
}