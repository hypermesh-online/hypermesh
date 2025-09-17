# Backup Module Variables

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "s3_bucket_arns" {
  description = "S3 bucket ARNs to include in backup"
  type        = list(string)
  default     = []
}

variable "instance_ids" {
  description = "EC2 instance IDs to include in backup"
  type        = list(string)
  default     = []
}

variable "critical_retention_days" {
  description = "Retention period for critical backups in days"
  type        = number
  default     = 2555 # 7 years for compliance
}

variable "standard_retention_days" {
  description = "Retention period for standard backups in days"
  type        = number
  default     = 90
}

variable "long_term_retention_days" {
  description = "Retention period for long-term backups in days"
  type        = number
  default     = 2555 # 7 years
}

variable "min_retention_days" {
  description = "Minimum retention period for compliance"
  type        = number
  default     = 7
}

variable "max_retention_days" {
  description = "Maximum retention period for vault lock"
  type        = number
  default     = 3650 # 10 years
}

variable "cross_region_vault_arn" {
  description = "Cross-region backup vault ARN for disaster recovery"
  type        = string
  default     = ""
}

variable "backup_reports_bucket" {
  description = "S3 bucket for backup reports"
  type        = string
  default     = ""
}

variable "sns_topic_arn" {
  description = "SNS topic ARN for backup alerts"
  type        = string
  default     = ""
}