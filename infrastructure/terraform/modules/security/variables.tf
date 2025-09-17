# Security Module Variables

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID"
  type        = string
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

variable "enable_guardduty" {
  description = "Enable AWS GuardDuty"
  type        = bool
  default     = true
}