# Storage Module Variables

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "certificate_transparency_retention_days" {
  description = "Number of days to retain CT logs"
  type        = number
  default     = 2555 # 7 years for legal compliance
}

variable "enable_cross_region_backup" {
  description = "Enable cross-region backup"
  type        = bool
  default     = true
}

variable "backup_region" {
  description = "Backup region for cross-region replication"
  type        = string
  default     = "us-east-1"
}