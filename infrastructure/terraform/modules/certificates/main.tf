# Certificates Module - SSL/TLS Certificates for TrustChain

# Request SSL Certificate for TrustChain Domain
resource "aws_acm_certificate" "trustchain" {
  domain_name               = var.domain_name
  subject_alternative_names = var.subject_alternative_names
  validation_method         = "DNS"

  options {
    certificate_transparency_logging_preference = "ENABLED"
  }

  lifecycle {
    create_before_destroy = true
  }

  tags = {
    Name        = "trustchain-ssl-cert-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain HTTPS Services"
    Domain      = var.domain_name
  }
}

# Certificate Validation via DNS
resource "aws_acm_certificate_validation" "trustchain" {
  certificate_arn         = aws_acm_certificate.trustchain.arn
  validation_record_fqdns = var.create_dns_records ? [for record in aws_route53_record.cert_validation : record.fqdn] : var.external_validation_fqdns

  timeouts {
    create = "10m"
  }

  depends_on = [aws_route53_record.cert_validation]
}

# Route 53 Records for Certificate Validation (if managing DNS)
resource "aws_route53_record" "cert_validation" {
  count = var.create_dns_records ? length(aws_acm_certificate.trustchain.domain_validation_options) : 0

  allow_overwrite = true
  name            = tolist(aws_acm_certificate.trustchain.domain_validation_options)[count.index].resource_record_name
  records         = [tolist(aws_acm_certificate.trustchain.domain_validation_options)[count.index].resource_record_value]
  ttl             = 60
  type            = tolist(aws_acm_certificate.trustchain.domain_validation_options)[count.index].resource_record_type
  zone_id         = var.route53_zone_id
}

# KMS Key for Certificate Private Key Protection
resource "aws_kms_key" "certificate_key" {
  description             = "TrustChain certificate private key protection"
  deletion_window_in_days = var.environment == "prod" ? 30 : 7
  enable_key_rotation     = true

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "Enable IAM User Permissions"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action   = "kms:*"
        Resource = "*"
      },
      {
        Sid    = "Allow ACM Service"
        Effect = "Allow"
        Principal = {
          Service = "acm.amazonaws.com"
        }
        Action = [
          "kms:Encrypt",
          "kms:Decrypt",
          "kms:ReEncrypt*",
          "kms:GenerateDataKey*",
          "kms:DescribeKey"
        ]
        Resource = "*"
      },
      {
        Sid    = "Allow TrustChain Services"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:role/trustchain-instance-role-${var.environment}"
        }
        Action = [
          "kms:Decrypt",
          "kms:DescribeKey"
        ]
        Resource = "*"
      }
    ]
  })

  tags = {
    Name        = "trustchain-cert-key-${var.environment}"
    Environment = var.environment
    Purpose     = "Certificate Private Key Protection"
  }
}

resource "aws_kms_alias" "certificate_key" {
  name          = "alias/trustchain-cert-${var.environment}"
  target_key_id = aws_kms_key.certificate_key.key_id
}

# CloudWatch Alarm for Certificate Expiration
resource "aws_cloudwatch_metric_alarm" "certificate_expiration" {
  alarm_name          = "trustchain-cert-expiration-${var.environment}"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = "1"
  metric_name        = "DaysToExpiry"
  namespace          = "AWS/CertificateManager"
  period             = "86400" # Daily check
  statistic          = "Average"
  threshold          = "30" # Alert 30 days before expiration
  alarm_description  = "TrustChain SSL certificate expiring soon"
  treat_missing_data = "breaching"

  dimensions = {
    CertificateArn = aws_acm_certificate.trustchain.arn
  }

  alarm_actions = compact([var.sns_topic_arn])

  tags = {
    Name        = "trustchain-cert-expiration-${var.environment}"
    Environment = var.environment
  }
}

# Certificate Monitoring and Renewal Automation
resource "aws_lambda_function" "certificate_monitor" {
  count = var.enable_certificate_monitoring ? 1 : 0

  filename         = data.archive_file.certificate_monitor[0].output_path
  function_name    = "trustchain-cert-monitor-${var.environment}"
  role            = aws_iam_role.certificate_monitor[0].arn
  handler         = "index.handler"
  source_code_hash = data.archive_file.certificate_monitor[0].output_base64sha256
  runtime         = "python3.9"
  timeout         = 60

  environment {
    variables = {
      CERTIFICATE_ARN = aws_acm_certificate.trustchain.arn
      ENVIRONMENT     = var.environment
      SNS_TOPIC_ARN   = var.sns_topic_arn
    }
  }

  tags = {
    Name        = "trustchain-cert-monitor-${var.environment}"
    Environment = var.environment
    Purpose     = "Certificate Monitoring"
  }
}

# Lambda Function Code for Certificate Monitoring
data "archive_file" "certificate_monitor" {
  count = var.enable_certificate_monitoring ? 1 : 0

  type        = "zip"
  output_path = "/tmp/certificate-monitor-${var.environment}.zip"
  source {
    content = templatefile("${path.module}/certificate_monitor.py", {
      environment = var.environment
    })
    filename = "index.py"
  }
}

# IAM Role for Certificate Monitor Lambda
resource "aws_iam_role" "certificate_monitor" {
  count = var.enable_certificate_monitoring ? 1 : 0
  name  = "trustchain-cert-monitor-role-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name        = "trustchain-cert-monitor-role-${var.environment}"
    Environment = var.environment
  }
}

# IAM Policy for Certificate Monitor
resource "aws_iam_role_policy" "certificate_monitor" {
  count = var.enable_certificate_monitoring ? 1 : 0
  name  = "trustchain-cert-monitor-policy-${var.environment}"
  role  = aws_iam_role.certificate_monitor[0].id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents"
        ]
        Resource = "arn:aws:logs:*:*:*"
      },
      {
        Effect = "Allow"
        Action = [
          "acm:DescribeCertificate",
          "acm:ListCertificates"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "sns:Publish"
        ]
        Resource = var.sns_topic_arn
      },
      {
        Effect = "Allow"
        Action = [
          "cloudwatch:PutMetricData"
        ]
        Resource = "*"
      }
    ]
  })
}

# EventBridge Rule for Certificate Monitor
resource "aws_cloudwatch_event_rule" "certificate_monitor" {
  count = var.enable_certificate_monitoring ? 1 : 0

  name                = "trustchain-cert-monitor-${var.environment}"
  description         = "Trigger certificate monitoring"
  schedule_expression = "rate(1 day)" # Run daily

  tags = {
    Name        = "trustchain-cert-monitor-rule-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_cloudwatch_event_target" "certificate_monitor" {
  count = var.enable_certificate_monitoring ? 1 : 0

  rule      = aws_cloudwatch_event_rule.certificate_monitor[0].name
  target_id = "TrustChainCertificateMonitor"
  arn       = aws_lambda_function.certificate_monitor[0].arn
}

resource "aws_lambda_permission" "certificate_monitor" {
  count = var.enable_certificate_monitoring ? 1 : 0

  statement_id  = "AllowExecutionFromCloudWatch"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.certificate_monitor[0].function_name
  principal     = "events.amazonaws.com"
  source_arn    = aws_cloudwatch_event_rule.certificate_monitor[0].arn
}

# Systems Manager Parameters for Certificate Information
resource "aws_ssm_parameter" "certificate_arn" {
  name  = "/trustchain/${var.environment}/certificate/arn"
  type  = "String"
  value = aws_acm_certificate.trustchain.arn

  tags = {
    Name        = "trustchain-cert-arn-${var.environment}"
    Environment = var.environment
    Purpose     = "Certificate Configuration"
  }
}

resource "aws_ssm_parameter" "certificate_domain" {
  name  = "/trustchain/${var.environment}/certificate/domain"
  type  = "String"
  value = var.domain_name

  tags = {
    Name        = "trustchain-cert-domain-${var.environment}"
    Environment = var.environment
    Purpose     = "Certificate Configuration"
  }
}

# Certificate Transparency Log Entry (for monitoring)
resource "aws_cloudwatch_log_metric_filter" "certificate_issued" {
  name           = "trustchain-certificates-issued-${var.environment}"
  log_group_name = "/aws/lambda/trustchain-cert-monitor-${var.environment}"
  pattern        = "[timestamp, request_id, level=\"INFO\", message=\"CERTIFICATE_ISSUED\", ...]"

  metric_transformation {
    name      = "CertificatesIssued"
    namespace = "TrustChain/Certificates"
    value     = "1"
  }
}

# Data source
data "aws_caller_identity" "current" {}