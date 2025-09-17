# Monitoring Module - CloudWatch, X-Ray, and Alerting

# CloudWatch Dashboard for TrustChain
resource "aws_cloudwatch_dashboard" "trustchain" {
  dashboard_name = "TrustChain-${var.environment}"

  dashboard_body = jsonencode({
    widgets = [
      {
        type   = "metric"
        x      = 0
        y      = 0
        width  = 12
        height = 6

        properties = {
          metrics = [
            ["AWS/ApplicationELB", "RequestCount", "LoadBalancer", var.load_balancer_arn],
            [".", "TargetResponseTime", ".", "."],
            [".", "HTTPCode_Target_2XX_Count", ".", "."],
            [".", "HTTPCode_Target_4XX_Count", ".", "."],
            [".", "HTTPCode_Target_5XX_Count", ".", "."]
          ]
          view    = "timeSeries"
          stacked = false
          region  = data.aws_region.current.name
          title   = "Load Balancer Metrics"
          period  = 300
        }
      },
      {
        type   = "metric"
        x      = 0
        y      = 6
        width  = 12
        height = 6

        properties = {
          metrics = [
            ["AWS/EC2", "CPUUtilization", "AutoScalingGroupName", var.autoscaling_group_name],
            [".", "NetworkIn", ".", "."],
            [".", "NetworkOut", ".", "."]
          ]
          view    = "timeSeries"
          stacked = false
          region  = data.aws_region.current.name
          title   = "EC2 Performance Metrics"
          period  = 300
        }
      },
      {
        type   = "metric"
        x      = 0
        y      = 12
        width  = 12
        height = 6

        properties = {
          metrics = [
            ["TrustChain/Performance", "CertificateOperations"],
            [".", "CertificateValidations"],
            [".", "DNSQueries"],
            [".", "CTLogEntries"]
          ]
          view    = "timeSeries"
          stacked = false
          region  = data.aws_region.current.name
          title   = "TrustChain Operations"
          period  = 60
        }
      },
      {
        type   = "metric"
        x      = 0
        y      = 18
        width  = 6
        height = 6

        properties = {
          metrics = [
            ["AWS/S3", "BucketSizeBytes", "BucketName", var.ct_logs_bucket_name, "StorageType", "StandardStorage"],
            [".", "NumberOfObjects", ".", ".", ".", "AllStorageTypes"]
          ]
          view    = "timeSeries"
          stacked = false
          region  = data.aws_region.current.name
          title   = "CT Logs Storage"
          period  = 86400
        }
      },
      {
        type   = "log"
        x      = 6
        y      = 18
        width  = 6
        height = 6

        properties = {
          query   = "SOURCE '/aws/ec2/trustchain/${var.environment}/application' | fields @timestamp, @message | filter @message like /ERROR/ | sort @timestamp desc | limit 20"
          region  = data.aws_region.current.name
          title   = "Recent Errors"
          view    = "table"
        }
      }
    ]
  })

  tags = {
    Name        = "trustchain-dashboard-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain Monitoring"
  }
}

# CloudWatch Log Groups
resource "aws_cloudwatch_log_group" "trustchain_application" {
  name              = "/aws/ec2/trustchain/${var.environment}/application"
  retention_in_days = var.log_retention_days

  tags = {
    Name        = "trustchain-app-logs-${var.environment}"
    Environment = var.environment
    LogType     = "Application"
  }
}

resource "aws_cloudwatch_log_group" "trustchain_security" {
  name              = "/aws/ec2/trustchain/${var.environment}/security"
  retention_in_days = var.log_retention_days * 2 # Keep security logs longer

  tags = {
    Name        = "trustchain-security-logs-${var.environment}"
    Environment = var.environment
    LogType     = "Security"
  }
}

resource "aws_cloudwatch_log_group" "trustchain_audit" {
  name              = "/aws/ec2/trustchain/${var.environment}/audit"
  retention_in_days = 365 # Keep audit logs for 1 year minimum

  tags = {
    Name        = "trustchain-audit-logs-${var.environment}"
    Environment = var.environment
    LogType     = "Audit"
    Compliance  = "Required"
  }
}

# Custom CloudWatch Metrics for TrustChain Performance
resource "aws_cloudwatch_metric_alarm" "certificate_operation_latency" {
  alarm_name          = "trustchain-cert-op-latency-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "CertificateOperationLatency"
  namespace          = "TrustChain/Performance"
  period             = "300"
  statistic          = "Average"
  threshold          = var.target_response_time_ms
  alarm_description  = "Certificate operation latency too high"

  alarm_actions = compact([var.sns_topic_arn])

  tags = {
    Name        = "trustchain-cert-op-latency-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_cloudwatch_metric_alarm" "certificate_operation_errors" {
  alarm_name          = "trustchain-cert-op-errors-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "1"
  metric_name        = "CertificateOperationErrors"
  namespace          = "TrustChain/Performance"
  period             = "300"
  statistic          = "Sum"
  threshold          = "10"
  alarm_description  = "High number of certificate operation errors"

  alarm_actions = compact([var.sns_topic_arn])

  tags = {
    Name        = "trustchain-cert-op-errors-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_cloudwatch_metric_alarm" "ct_log_processing_delay" {
  alarm_name          = "trustchain-ct-log-delay-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "CTLogProcessingDelay"
  namespace          = "TrustChain/Performance"
  period             = "300"
  statistic          = "Average"
  threshold          = "30000" # 30 seconds in milliseconds
  alarm_description  = "CT log processing delay too high"

  alarm_actions = compact([var.sns_topic_arn])

  tags = {
    Name        = "trustchain-ct-log-delay-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_cloudwatch_metric_alarm" "dns_query_latency" {
  alarm_name          = "trustchain-dns-latency-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "DNSQueryLatency"
  namespace          = "TrustChain/Performance"
  period             = "300"
  statistic          = "Average"
  threshold          = "100" # 100ms for DNS queries
  alarm_description  = "DNS query latency too high"

  alarm_actions = compact([var.sns_topic_arn])

  tags = {
    Name        = "trustchain-dns-latency-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_cloudwatch_metric_alarm" "stoq_throughput" {
  alarm_name          = "trustchain-stoq-throughput-${var.environment}"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = "3"
  metric_name        = "STOQThroughputGbps"
  namespace          = "TrustChain/Performance"
  period             = "300"
  statistic          = "Average"
  threshold          = var.stoq_target_throughput_gbps * 0.8 # Alert at 80% of target
  alarm_description  = "STOQ throughput below target"

  alarm_actions = compact([var.sns_topic_arn])

  tags = {
    Name        = "trustchain-stoq-throughput-${var.environment}"
    Environment = var.environment
  }
}

# Target Group Health Monitoring
resource "aws_cloudwatch_metric_alarm" "target_group_healthy_hosts" {
  for_each = var.target_group_arns

  alarm_name          = "trustchain-${each.key}-healthy-hosts-${var.environment}"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "HealthyHostCount"
  namespace          = "AWS/ApplicationELB"
  period             = "60"
  statistic          = "Average"
  threshold          = "1"
  alarm_description  = "No healthy hosts in ${each.key} target group"

  dimensions = {
    TargetGroup = each.value
  }

  alarm_actions = compact([var.sns_topic_arn])

  tags = {
    Name        = "trustchain-${each.key}-healthy-hosts-${var.environment}"
    Environment = var.environment
    Service     = each.key
  }
}

# X-Ray Tracing (if enabled)
resource "aws_xray_sampling_rule" "trustchain" {
  count = var.enable_xray_tracing ? 1 : 0

  rule_name      = "TrustChainSampling-${var.environment}"
  priority       = 9000
  version        = 1
  reservoir_size = 1
  fixed_rate     = 0.1
  url_path       = "*"
  host           = "*"
  http_method    = "*"
  service_type   = "*"
  service_name   = "*"
  resource_arn   = "*"

  tags = {
    Name        = "trustchain-xray-sampling-${var.environment}"
    Environment = var.environment
  }
}

# EventBridge Rules for TrustChain Events
resource "aws_cloudwatch_event_rule" "certificate_issued" {
  name        = "trustchain-cert-issued-${var.environment}"
  description = "Capture certificate issuance events"

  event_pattern = jsonencode({
    source      = ["trustchain.certificate"]
    detail-type = ["Certificate Issued"]
    detail = {
      environment = [var.environment]
    }
  })

  tags = {
    Name        = "trustchain-cert-issued-rule-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_cloudwatch_event_target" "certificate_issued_log" {
  rule      = aws_cloudwatch_event_rule.certificate_issued.name
  target_id = "LogCertificateIssuance"
  arn       = aws_cloudwatch_log_group.trustchain_audit.arn
}

# CloudWatch Logs Metric Filters
resource "aws_cloudwatch_log_metric_filter" "error_count" {
  name           = "trustchain-error-count-${var.environment}"
  log_group_name = aws_cloudwatch_log_group.trustchain_application.name
  pattern        = "ERROR"

  metric_transformation {
    name      = "ErrorCount"
    namespace = "TrustChain/Errors"
    value     = "1"
  }
}

resource "aws_cloudwatch_log_metric_filter" "certificate_operations" {
  name           = "trustchain-cert-ops-${var.environment}"
  log_group_name = aws_cloudwatch_log_group.trustchain_application.name
  pattern        = "[timestamp, request_id, operation=\"CERTIFICATE_*\", ...]"

  metric_transformation {
    name      = "CertificateOperations"
    namespace = "TrustChain/Performance"
    value     = "1"
  }
}

resource "aws_cloudwatch_log_metric_filter" "security_events" {
  name           = "trustchain-security-events-${var.environment}"
  log_group_name = aws_cloudwatch_log_group.trustchain_security.name
  pattern        = "[timestamp, level=\"SECURITY\", ...]"

  metric_transformation {
    name      = "SecurityEvents"
    namespace = "TrustChain/Security"
    value     = "1"
  }
}

# CloudWatch Composite Alarms
resource "aws_cloudwatch_composite_alarm" "trustchain_health" {
  alarm_name        = "trustchain-overall-health-${var.environment}"
  alarm_description = "Overall health of TrustChain infrastructure"

  alarm_rule = format(
    "ALARM(%s) OR ALARM(%s) OR ALARM(%s)",
    aws_cloudwatch_metric_alarm.certificate_operation_latency.alarm_name,
    aws_cloudwatch_metric_alarm.certificate_operation_errors.alarm_name,
    aws_cloudwatch_metric_alarm.ct_log_processing_delay.alarm_name
  )

  alarm_actions = compact([var.sns_topic_arn])

  tags = {
    Name        = "trustchain-overall-health-${var.environment}"
    Environment = var.environment
    Type        = "Composite"
  }
}

# CloudWatch Insights Queries
resource "aws_cloudwatch_query_definition" "performance_analysis" {
  name = "TrustChain Performance Analysis"

  log_group_names = [
    aws_cloudwatch_log_group.trustchain_application.name
  ]

  query_string = <<EOF
fields @timestamp, @message, operation, duration, status
| filter operation like /CERTIFICATE/
| stats avg(duration), max(duration), count() by operation
| sort avg(duration) desc
EOF
}

resource "aws_cloudwatch_query_definition" "error_analysis" {
  name = "TrustChain Error Analysis"

  log_group_names = [
    aws_cloudwatch_log_group.trustchain_application.name
  ]

  query_string = <<EOF
fields @timestamp, @message, error_code, operation
| filter @message like /ERROR/
| stats count() by error_code, operation
| sort count desc
EOF
}

# Data source
data "aws_region" "current" {}