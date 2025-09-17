# DNS Module Outputs

output "zone_id" {
  description = "Route 53 hosted zone ID"
  value       = local.zone_id
}

output "name_servers" {
  description = "Route 53 name servers"
  value       = var.create_hosted_zone ? aws_route53_zone.trustchain[0].name_servers : data.aws_route53_zone.existing[0].name_servers
}

output "domain_name" {
  description = "Primary domain name"
  value       = var.domain_name
}

output "primary_record_name" {
  description = "Primary AAAA record name"
  value       = aws_route53_record.trustchain_primary.name
}

output "service_records" {
  description = "Service-specific DNS records"
  value = {
    ca   = aws_route53_record.ca_service.name
    ct   = aws_route53_record.ct_service.name
    dns  = aws_route53_record.dns_service.name
    stoq = aws_route53_record.stoq_service.name
    api  = aws_route53_record.api_service.name
  }
}

output "health_check_ids" {
  description = "Route 53 health check IDs"
  value = {
    https    = aws_route53_health_check.trustchain_https.id
    dns_quic = var.enable_dns_health_checks ? aws_route53_health_check.trustchain_dns_quic[0].id : null
  }
}

output "caa_records" {
  description = "Certificate Authority Authorization records"
  value       = aws_route53_record.caa_records.records
}

output "txt_verification_record" {
  description = "Domain verification TXT record"
  value       = aws_route53_record.domain_verification.records
  sensitive   = true
}

output "dnssec_configuration" {
  description = "DNSSEC configuration"
  value = var.enable_dnssec ? {
    enabled    = true
    ksk_id     = aws_route53_key_signing_key.trustchain[0].key_tag
    ds_records = aws_route53_hosted_zone_dnssec.trustchain[0].status_message
  } : {
    enabled = false
  }
  sensitive = var.enable_dnssec
}

output "query_logging_configuration" {
  description = "DNS query logging configuration"
  value = var.enable_query_logging ? {
    enabled           = true
    log_group_name    = aws_cloudwatch_log_group.dns_query_logs[0].name
    log_group_arn     = aws_cloudwatch_log_group.dns_query_logs[0].arn
    retention_days    = 30
  } : {
    enabled = false
  }
}

output "dns_configuration_summary" {
  description = "DNS configuration summary"
  value = {
    domain_name             = var.domain_name
    ipv6_only              = true
    hosted_zone_managed    = var.create_hosted_zone
    dnssec_enabled         = var.enable_dnssec
    health_checks_enabled  = var.enable_dns_health_checks
    query_logging_enabled  = var.enable_query_logging
    caa_records_configured = true
    txt_verification       = true
    service_records_count  = 5
  }
}

output "service_endpoints" {
  description = "Service endpoint DNS names"
  value = {
    primary_domain    = var.domain_name
    ca_endpoint      = "ca.${var.domain_name}"
    ct_endpoint      = "ct.${var.domain_name}"
    dns_endpoint     = "dns.${var.domain_name}"
    stoq_endpoint    = "stoq.${var.domain_name}"
    api_endpoint     = "api.${var.domain_name}"
  }
}

output "cloudwatch_alarms" {
  description = "CloudWatch alarm ARNs for DNS monitoring"
  value = {
    dns_health_check = aws_cloudwatch_metric_alarm.dns_health_check.arn
  }
}

output "dns_security_features" {
  description = "DNS security features configured"
  value = {
    caa_protection          = true
    domain_verification     = true
    dnssec_enabled         = var.enable_dnssec
    health_monitoring      = var.enable_dns_health_checks
    query_logging          = var.enable_query_logging
    ipv6_only_resolution   = true
    certificate_pinning    = true
  }
}