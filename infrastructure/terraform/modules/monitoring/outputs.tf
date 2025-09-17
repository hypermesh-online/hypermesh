# Monitoring Module Outputs

output "dashboard_url" {
  description = "CloudWatch dashboard URL"
  value       = "https://${data.aws_region.current.name}.console.aws.amazon.com/cloudwatch/home?region=${data.aws_region.current.name}#dashboards:name=${aws_cloudwatch_dashboard.trustchain.dashboard_name}"
}

output "dashboard_name" {
  description = "CloudWatch dashboard name"
  value       = aws_cloudwatch_dashboard.trustchain.dashboard_name
}

output "log_group_names" {
  description = "CloudWatch log group names"
  value = {
    application = aws_cloudwatch_log_group.trustchain_application.name
    security    = aws_cloudwatch_log_group.trustchain_security.name
    audit       = aws_cloudwatch_log_group.trustchain_audit.name
  }
}

output "log_group_arns" {
  description = "CloudWatch log group ARNs"
  value = {
    application = aws_cloudwatch_log_group.trustchain_application.arn
    security    = aws_cloudwatch_log_group.trustchain_security.arn
    audit       = aws_cloudwatch_log_group.trustchain_audit.arn
  }
}

output "alarm_arns" {
  description = "CloudWatch alarm ARNs"
  value = {
    certificate_operation_latency = aws_cloudwatch_metric_alarm.certificate_operation_latency.arn
    certificate_operation_errors  = aws_cloudwatch_metric_alarm.certificate_operation_errors.arn
    ct_log_processing_delay      = aws_cloudwatch_metric_alarm.ct_log_processing_delay.arn
    dns_query_latency           = aws_cloudwatch_metric_alarm.dns_query_latency.arn
    stoq_throughput             = aws_cloudwatch_metric_alarm.stoq_throughput.arn
    overall_health              = aws_cloudwatch_composite_alarm.trustchain_health.arn
  }
}

output "target_group_alarms" {
  description = "Target group health alarm ARNs"
  value       = { for k, v in aws_cloudwatch_metric_alarm.target_group_healthy_hosts : k => v.arn }
}

output "metric_filters" {
  description = "CloudWatch log metric filters"
  value = {
    error_count           = aws_cloudwatch_log_metric_filter.error_count.name
    certificate_operations = aws_cloudwatch_log_metric_filter.certificate_operations.name
    security_events       = aws_cloudwatch_log_metric_filter.security_events.name
  }
}

output "xray_sampling_rule_arn" {
  description = "X-Ray sampling rule ARN"
  value       = var.enable_xray_tracing ? aws_xray_sampling_rule.trustchain[0].arn : null
}

output "event_rules" {
  description = "EventBridge rule ARNs"
  value = {
    certificate_issued = aws_cloudwatch_event_rule.certificate_issued.arn
  }
}

output "insights_queries" {
  description = "CloudWatch Insights query names"
  value = {
    performance_analysis = aws_cloudwatch_query_definition.performance_analysis.name
    error_analysis      = aws_cloudwatch_query_definition.error_analysis.name
  }
}

output "monitoring_configuration" {
  description = "Monitoring configuration summary"
  value = {
    dashboard_enabled       = true
    detailed_monitoring     = var.enable_detailed_monitoring
    xray_tracing           = var.enable_xray_tracing
    log_retention_days     = var.log_retention_days
    audit_log_retention    = 365
    security_log_retention = var.log_retention_days * 2
    composite_alarms       = true
    custom_metrics         = true
    automated_insights     = true
  }
}

output "performance_thresholds" {
  description = "Performance monitoring thresholds"
  value = {
    certificate_operation_latency_ms = var.target_response_time_ms
    dns_query_latency_ms            = 100
    ct_log_processing_delay_ms      = 30000
    stoq_throughput_gbps_min        = var.stoq_target_throughput_gbps * 0.8
    error_threshold_per_5min        = 10
  }
}