# Load Balancer Module Outputs

output "alb_arn" {
  description = "Application Load Balancer ARN"
  value       = aws_lb.trustchain.arn
}

output "alb_dns_name" {
  description = "Application Load Balancer DNS name"
  value       = aws_lb.trustchain.dns_name
}

output "alb_zone_id" {
  description = "Application Load Balancer hosted zone ID"
  value       = aws_lb.trustchain.zone_id
}

output "alb_ipv6_address" {
  description = "Application Load Balancer IPv6 address"
  value       = aws_lb.trustchain.dns_name # ALB doesn't expose direct IPv6, use DNS resolution
}

output "nlb_arn" {
  description = "Network Load Balancer ARN"
  value       = aws_lb.trustchain_nlb.arn
}

output "nlb_dns_name" {
  description = "Network Load Balancer DNS name"
  value       = aws_lb.trustchain_nlb.dns_name
}

output "nlb_zone_id" {
  description = "Network Load Balancer hosted zone ID"
  value       = aws_lb.trustchain_nlb.zone_id
}

output "target_group_arns" {
  description = "Target group ARNs"
  value = {
    ca          = aws_lb_target_group.ca.arn
    ct          = aws_lb_target_group.ct.arn
    hypermesh   = aws_lb_target_group.hypermesh.arn
    integration = aws_lb_target_group.integration.arn
    dns         = aws_lb_target_group.dns.arn
    stoq        = aws_lb_target_group.stoq.arn
  }
}

output "target_group_names" {
  description = "Target group names"
  value = {
    ca          = aws_lb_target_group.ca.name
    ct          = aws_lb_target_group.ct.name
    hypermesh   = aws_lb_target_group.hypermesh.name
    integration = aws_lb_target_group.integration.name
    dns         = aws_lb_target_group.dns.name
    stoq        = aws_lb_target_group.stoq.name
  }
}

output "listener_arns" {
  description = "Listener ARNs"
  value = {
    ca          = aws_lb_listener.ca.arn
    ct          = aws_lb_listener.ct.arn
    hypermesh   = aws_lb_listener.hypermesh.arn
    integration = aws_lb_listener.integration.arn
    dns         = aws_lb_listener.dns.arn
    stoq        = aws_lb_listener.stoq.arn
  }
}

output "service_endpoints" {
  description = "Service endpoint URLs"
  value = {
    ca_api           = "https://${aws_lb.trustchain.dns_name}:8443"
    certificate_transparency = "https://${aws_lb.trustchain.dns_name}:6962"
    dns_over_quic    = "quic://${aws_lb.trustchain_nlb.dns_name}:8853"
    stoq_protocol    = "quic://${aws_lb.trustchain_nlb.dns_name}:8444"
    hypermesh_integration = "https://${aws_lb.trustchain.dns_name}:8445"
    integration_api  = "https://${aws_lb.trustchain.dns_name}:8446"
  }
}

output "access_logs_bucket" {
  description = "ALB access logs S3 bucket"
  value = {
    name = aws_s3_bucket.alb_access_logs.bucket
    arn  = aws_s3_bucket.alb_access_logs.arn
  }
}

output "cloudwatch_alarms" {
  description = "CloudWatch alarm ARNs"
  value = {
    target_response_time = aws_cloudwatch_metric_alarm.alb_target_response_time.arn
    healthy_host_count   = aws_cloudwatch_metric_alarm.alb_healthy_host_count.arn
  }
}

output "load_balancer_configuration" {
  description = "Load balancer configuration summary"
  value = {
    alb_ipv6_only          = true
    nlb_ipv6_only          = true
    ssl_policy             = "ELBSecurityPolicy-TLS13-1-2-2021-06"
    cross_zone_enabled     = true
    access_logs_enabled    = true
    waf_enabled           = var.waf_web_acl_arn != ""
    deletion_protection   = var.environment == "prod" ? true : false
  }
}