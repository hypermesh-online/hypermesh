# Certificates Module Variables

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "domain_name" {
  description = "Primary domain name for SSL certificate"
  type        = string
}

variable "subject_alternative_names" {
  description = "Subject Alternative Names for the SSL certificate"
  type        = list(string)
  default = [
    "ca.trust.hypermesh.online",
    "ct.trust.hypermesh.online",
    "dns.trust.hypermesh.online",
    "stoq.trust.hypermesh.online",
    "api.trust.hypermesh.online"
  ]
}

variable "create_dns_records" {
  description = "Create DNS validation records in Route 53"
  type        = bool
  default     = true
}

variable "route53_zone_id" {
  description = "Route 53 hosted zone ID for DNS validation"
  type        = string
  default     = ""
}

variable "external_validation_fqdns" {
  description = "External validation FQDNs if not using Route 53"
  type        = list(string)
  default     = []
}

variable "sns_topic_arn" {
  description = "SNS topic ARN for certificate alerts"
  type        = string
  default     = ""
}

variable "enable_certificate_monitoring" {
  description = "Enable automated certificate monitoring"
  type        = bool
  default     = true
}