# Monitoring Module Variables

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "load_balancer_arn" {
  description = "Load balancer ARN for monitoring"
  type        = string
}

variable "target_group_arns" {
  description = "Target group ARNs for health monitoring"
  type        = map(string)
}

variable "instance_ids" {
  description = "EC2 instance IDs for monitoring"
  type        = list(string)
}

variable "autoscaling_group_name" {
  description = "Auto Scaling Group name for monitoring"
  type        = string
  default     = ""
}

variable "ct_logs_bucket_name" {
  description = "CT logs S3 bucket name for monitoring"
  type        = string
  default     = ""
}

variable "sns_topic_arn" {
  description = "SNS topic ARN for alerts"
  type        = string
  default     = ""
}

variable "log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 90
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

variable "enable_xray_tracing" {
  description = "Enable AWS X-Ray distributed tracing"
  type        = bool
  default     = true
}

variable "enable_detailed_monitoring" {
  description = "Enable detailed CloudWatch monitoring"
  type        = bool
  default     = true
}