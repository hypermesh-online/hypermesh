# Backup Module Outputs

output "backup_vault_arn" {
  description = "Backup vault ARN"
  value       = aws_backup_vault.trustchain.arn
}

output "backup_vault_name" {
  description = "Backup vault name"
  value       = aws_backup_vault.trustchain.name
}

output "backup_plan_arn" {
  description = "Backup plan ARN"
  value       = aws_backup_plan.trustchain_critical.arn
}

output "backup_plan_id" {
  description = "Backup plan ID"
  value       = aws_backup_plan.trustchain_critical.id
}

output "backup_iam_role_arn" {
  description = "Backup service IAM role ARN"
  value       = aws_iam_role.backup.arn
}

output "kms_key_arn" {
  description = "Backup encryption KMS key ARN"
  value       = aws_kms_key.backup.arn
}

output "kms_key_id" {
  description = "Backup encryption KMS key ID"
  value       = aws_kms_key.backup.key_id
}

output "backup_selections" {
  description = "Backup selection details"
  value = {
    s3_buckets    = aws_backup_selection.s3_buckets.name
    ec2_instances = length(var.instance_ids) > 0 ? aws_backup_selection.ec2_instances[0].name : null
  }
}

output "compliance_framework_arn" {
  description = "Backup compliance framework ARN"
  value       = aws_backup_framework.trustchain_compliance.arn
}

output "backup_report_plan_arn" {
  description = "Backup report plan ARN"
  value       = aws_backup_report_plan.trustchain.arn
}

output "cloudwatch_alarms" {
  description = "CloudWatch alarm ARNs for backup monitoring"
  value = {
    backup_job_failed = aws_cloudwatch_metric_alarm.backup_job_failed.arn
    backup_vault_size = aws_cloudwatch_metric_alarm.backup_vault_size.arn
  }
}

output "event_rule_arn" {
  description = "EventBridge rule ARN for backup events"
  value       = aws_cloudwatch_event_rule.backup_events.arn
}

output "backup_configuration" {
  description = "Backup configuration summary"
  value = {
    vault_locked             = var.environment == "prod"
    cross_region_backup      = var.cross_region_vault_arn != ""
    critical_retention_days  = var.critical_retention_days
    standard_retention_days  = var.standard_retention_days
    long_term_retention_days = var.long_term_retention_days
    encryption_enabled       = true
    compliance_framework     = true
    automated_reporting      = true
    event_monitoring        = true
  }
}

output "backup_schedules" {
  description = "Backup schedule configuration"
  value = {
    critical_frequency = "Every 6 hours"
    standard_frequency = "Daily at 2 AM UTC"
    weekly_frequency   = "Weekly on Sunday at 3 AM UTC"
    cold_storage_after = "30 days"
    lifecycle_management = true
  }
}

output "compliance_features" {
  description = "Compliance and governance features"
  value = {
    vault_lock_enabled       = var.environment == "prod"
    encryption_required      = true
    minimum_retention_check  = true
    backup_frequency_check   = true
    compliance_reporting     = true
    audit_logging           = true
    immutable_backups       = var.environment == "prod"
  }
}

output "disaster_recovery" {
  description = "Disaster recovery configuration"
  value = {
    cross_region_replication = var.cross_region_vault_arn != ""
    automated_failover      = false # Manual failover for production safety
    rto_target_hours       = 4
    rpo_target_hours       = 6
    backup_testing_enabled = true
  }
}