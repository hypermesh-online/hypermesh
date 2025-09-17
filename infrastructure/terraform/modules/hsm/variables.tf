# HSM Module Variables

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID"
  type        = string
}

variable "subnet_ids" {
  description = "Subnet IDs for HSM deployment"
  type        = list(string)
}

variable "enable_hsm" {
  description = "Enable AWS CloudHSM"
  type        = bool
  default     = true
}

variable "enable_ha" {
  description = "Enable high availability (multiple HSM instances)"
  type        = bool
  default     = true
}

variable "enable_backup" {
  description = "Enable automated backup configuration"
  type        = bool
  default     = true
}