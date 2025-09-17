# Storage Module Outputs

output "bucket_ids" {
  description = "S3 bucket IDs"
  value = {
    ct_logs      = aws_s3_bucket.ct_logs.id
    certificates = aws_s3_bucket.certificates.id
    config       = aws_s3_bucket.config.id
    backup       = var.enable_cross_region_backup ? aws_s3_bucket.backup[0].id : null
  }
}

output "bucket_names" {
  description = "S3 bucket names"
  value = {
    ct_logs      = aws_s3_bucket.ct_logs.bucket
    certificates = aws_s3_bucket.certificates.bucket
    config       = aws_s3_bucket.config.bucket
    backup       = var.enable_cross_region_backup ? aws_s3_bucket.backup[0].bucket : null
  }
}

output "bucket_arns" {
  description = "S3 bucket ARNs"
  value = {
    ct_logs      = aws_s3_bucket.ct_logs.arn
    certificates = aws_s3_bucket.certificates.arn
    config       = aws_s3_bucket.config.arn
    backup       = var.enable_cross_region_backup ? aws_s3_bucket.backup[0].arn : null
  }
}

output "kms_key_id" {
  description = "KMS key ID for S3 encryption"
  value       = aws_kms_key.trustchain.key_id
}

output "kms_key_arn" {
  description = "KMS key ARN for S3 encryption"
  value       = aws_kms_key.trustchain.arn
}

output "kms_alias" {
  description = "KMS key alias"
  value       = aws_kms_alias.trustchain.name
}

output "storage_configuration" {
  description = "Storage configuration summary"
  value = {
    encryption_enabled        = true
    kms_key_rotation         = true
    versioning_enabled       = true
    lifecycle_policies       = true
    cross_region_backup      = var.enable_cross_region_backup
    ct_logs_retention_days   = var.certificate_transparency_retention_days
    public_access_blocked    = true
    secure_transport_only    = true
  }
}

output "compliance_configuration" {
  description = "Compliance and legal configuration"
  value = {
    ct_logs_retention_years    = var.certificate_transparency_retention_days / 365
    certificate_retention_years = (var.certificate_transparency_retention_days + 365) / 365
    legal_compliance_met      = var.certificate_transparency_retention_days >= 2555 # 7 years minimum
    data_residency_controls   = true
    encryption_at_rest        = true
    audit_logging_enabled     = true
  }
}