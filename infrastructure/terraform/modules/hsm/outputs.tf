# HSM Module Outputs

output "cluster_id" {
  description = "CloudHSM cluster ID"
  value       = var.enable_hsm ? aws_cloudhsm_v2_cluster.trustchain[0].cluster_id : null
  sensitive   = true
}

output "cluster_state" {
  description = "CloudHSM cluster state"
  value       = var.enable_hsm ? aws_cloudhsm_v2_cluster.trustchain[0].cluster_state : null
}

output "cluster_certificates" {
  description = "CloudHSM cluster certificates"
  value       = var.enable_hsm ? aws_cloudhsm_v2_cluster.trustchain[0].cluster_certificates : null
  sensitive   = true
}

output "hsm_security_group_id" {
  description = "Security group ID for CloudHSM"
  value       = var.enable_hsm ? aws_security_group.cloudhsm[0].id : null
}

output "hsm_instances" {
  description = "HSM instance information"
  value = var.enable_hsm ? {
    primary   = aws_cloudhsm_v2_hsm.trustchain_primary[0].hsm_id
    secondary = var.enable_ha ? aws_cloudhsm_v2_hsm.trustchain_secondary[0].hsm_id : null
  } : null
  sensitive = true
}

output "iam_role_arn" {
  description = "IAM role ARN for HSM client access"
  value       = var.enable_hsm ? aws_iam_role.hsm_client[0].arn : null
}

output "backup_key_arn" {
  description = "KMS key ARN for HSM backup encryption"
  value       = var.enable_hsm && var.enable_backup ? aws_kms_key.hsm_backup[0].arn : null
}

output "cloudwatch_alarms" {
  description = "CloudWatch alarm ARNs for HSM monitoring"
  value = var.enable_hsm ? {
    cluster_state     = aws_cloudwatch_metric_alarm.hsm_cluster_state[0].arn
    instance_health   = aws_cloudwatch_metric_alarm.hsm_instance_health[0].arn
  } : null
}

output "hsm_configuration" {
  description = "HSM configuration summary"
  value = {
    enabled               = var.enable_hsm
    high_availability     = var.enable_ha
    backup_enabled        = var.enable_backup
    compliance_level      = "FIPS-140-2-Level-3"
    audit_logging         = var.enable_hsm
    cluster_encryption    = var.enable_hsm
  }
}