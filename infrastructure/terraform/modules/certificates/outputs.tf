# Certificates Module Outputs

output "ssl_certificate_arn" {
  description = "SSL certificate ARN"
  value       = aws_acm_certificate_validation.trustchain.certificate_arn
}

output "ssl_certificate_domain_name" {
  description = "SSL certificate domain name"
  value       = aws_acm_certificate.trustchain.domain_name
}

output "ssl_certificate_status" {
  description = "SSL certificate status"
  value       = aws_acm_certificate.trustchain.status
}

output "domain_validation_options" {
  description = "Domain validation options for the certificate"
  value       = aws_acm_certificate.trustchain.domain_validation_options
  sensitive   = true
}

output "certificate_transparency_enabled" {
  description = "Certificate transparency logging status"
  value       = true
}

output "kms_key_arn" {
  description = "KMS key ARN for certificate protection"
  value       = aws_kms_key.certificate_key.arn
}

output "kms_key_id" {
  description = "KMS key ID for certificate protection"
  value       = aws_kms_key.certificate_key.key_id
}

output "certificate_monitor_function_arn" {
  description = "Certificate monitor Lambda function ARN"
  value       = var.enable_certificate_monitoring ? aws_lambda_function.certificate_monitor[0].arn : null
}

output "certificate_expiration_alarm_arn" {
  description = "Certificate expiration CloudWatch alarm ARN"
  value       = aws_cloudwatch_metric_alarm.certificate_expiration.arn
}

output "ssm_parameters" {
  description = "Systems Manager parameter names for certificate configuration"
  value = {
    certificate_arn    = aws_ssm_parameter.certificate_arn.name
    certificate_domain = aws_ssm_parameter.certificate_domain.name
  }
}

output "certificate_configuration" {
  description = "Certificate configuration summary"
  value = {
    domain_name                    = var.domain_name
    subject_alternative_names      = var.subject_alternative_names
    validation_method             = "DNS"
    certificate_transparency      = true
    automated_monitoring          = var.enable_certificate_monitoring
    expiration_alerting          = true
    kms_protection              = true
    dns_validation_managed       = var.create_dns_records
  }
}

output "security_features" {
  description = "Certificate security features"
  value = {
    tls_version_minimum          = "1.3"
    certificate_transparency     = true
    automated_renewal           = true
    expiration_monitoring       = true
    private_key_protection      = "KMS"
    validation_method          = "DNS"
    multi_domain_support       = length(var.subject_alternative_names) > 0
  }
}