# Security Module Outputs

output "trustchain_security_group_id" {
  description = "Security group ID for TrustChain instances"
  value       = aws_security_group.trustchain.id
}

output "alb_security_group_id" {
  description = "Security group ID for Application Load Balancer"
  value       = aws_security_group.alb.id
}

output "waf_web_acl_arn" {
  description = "WAF Web ACL ARN"
  value       = aws_wafv2_web_acl.trustchain.arn
}

output "waf_web_acl_id" {
  description = "WAF Web ACL ID"
  value       = aws_wafv2_web_acl.trustchain.id
}

output "guardduty_detector_id" {
  description = "GuardDuty detector ID"
  value       = var.enable_guardduty ? aws_guardduty_detector.trustchain[0].id : null
}

output "security_alerts_topic_arn" {
  description = "SNS topic ARN for security alerts"
  value       = aws_sns_topic.security_alerts.arn
}

output "security_configuration" {
  description = "Security configuration summary"
  value = {
    waf_enabled              = true
    guardduty_enabled        = var.enable_guardduty
    rate_limiting_enabled    = true
    malware_protection       = var.enable_guardduty
    security_monitoring      = true
    ipv6_only_access        = true
  }
}