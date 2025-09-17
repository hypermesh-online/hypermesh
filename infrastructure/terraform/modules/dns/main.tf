# DNS Module - Route 53 Configuration for TrustChain

# Route 53 Hosted Zone for TrustChain Domain
resource "aws_route53_zone" "trustchain" {
  count = var.create_hosted_zone ? 1 : 0
  name  = var.domain_name

  tags = {
    Name        = "trustchain-zone-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain DNS"
    Domain      = var.domain_name
  }
}

# Use existing hosted zone if not creating new one
data "aws_route53_zone" "existing" {
  count = var.create_hosted_zone ? 0 : 1
  name  = var.domain_name
}

locals {
  zone_id = var.create_hosted_zone ? aws_route53_zone.trustchain[0].zone_id : data.aws_route53_zone.existing[0].zone_id
}

# Primary AAAA record for trust.hypermesh.online (IPv6 only)
resource "aws_route53_record" "trustchain_primary" {
  zone_id = local.zone_id
  name    = var.domain_name
  type    = "AAAA"
  
  alias {
    name                   = var.load_balancer_dns_name
    zone_id                = var.load_balancer_zone_id
    evaluate_target_health = true
  }

  tags = {
    Name        = "trustchain-primary-aaaa-${var.environment}"
    Environment = var.environment
    RecordType  = "Primary"
  }
}

# Service-specific CNAME records
resource "aws_route53_record" "ca_service" {
  zone_id = local.zone_id
  name    = "ca.${var.domain_name}"
  type    = "CNAME"
  ttl     = 300
  records = [var.domain_name]

  tags = {
    Name        = "trustchain-ca-cname-${var.environment}"
    Environment = var.environment
    Service     = "Certificate Authority"
  }
}

resource "aws_route53_record" "ct_service" {
  zone_id = local.zone_id
  name    = "ct.${var.domain_name}"
  type    = "CNAME"
  ttl     = 300
  records = [var.domain_name]

  tags = {
    Name        = "trustchain-ct-cname-${var.environment}"
    Environment = var.environment
    Service     = "Certificate Transparency"
  }
}

resource "aws_route53_record" "dns_service" {
  zone_id = local.zone_id
  name    = "dns.${var.domain_name}"
  type    = "CNAME"
  ttl     = 300
  records = [var.domain_name]

  tags = {
    Name        = "trustchain-dns-cname-${var.environment}"
    Environment = var.environment
    Service     = "DNS Resolution"
  }
}

resource "aws_route53_record" "stoq_service" {
  zone_id = local.zone_id
  name    = "stoq.${var.domain_name}"
  type    = "CNAME"
  ttl     = 300
  records = [var.domain_name]

  tags = {
    Name        = "trustchain-stoq-cname-${var.environment}"
    Environment = var.environment
    Service     = "STOQ Protocol"
  }
}

resource "aws_route53_record" "api_service" {
  zone_id = local.zone_id
  name    = "api.${var.domain_name}"
  type    = "CNAME"
  ttl     = 300
  records = [var.domain_name]

  tags = {
    Name        = "trustchain-api-cname-${var.environment}"
    Environment = var.environment
    Service     = "API Endpoint"
  }
}

# Health Checks for DNS Monitoring
resource "aws_route53_health_check" "trustchain_https" {
  fqdn                            = var.domain_name
  port                            = 8443
  type                            = "HTTPS"
  resource_path                   = "/health"
  failure_threshold               = 3
  request_interval                = 30
  cloudwatch_alarm_region         = data.aws_region.current.name
  cloudwatch_alarm_name           = "trustchain-dns-health-${var.environment}"
  insufficient_data_health_status = "Failure"

  tags = {
    Name        = "trustchain-https-health-${var.environment}"
    Environment = var.environment
    Service     = "HTTPS Health Check"
  }
}

resource "aws_route53_health_check" "trustchain_dns_quic" {
  count = var.enable_dns_health_checks ? 1 : 0
  
  # Note: Route53 doesn't support QUIC health checks directly
  # This is a workaround using HTTPS to a health endpoint
  fqdn                            = var.domain_name
  port                            = 8080
  type                            = "HTTPS"
  resource_path                   = "/health/dns"
  failure_threshold               = 3
  request_interval                = 30
  cloudwatch_alarm_region         = data.aws_region.current.name
  cloudwatch_alarm_name           = "trustchain-dns-quic-health-${var.environment}"
  insufficient_data_health_status = "Failure"

  tags = {
    Name        = "trustchain-dns-quic-health-${var.environment}"
    Environment = var.environment
    Service     = "DNS-over-QUIC Health Check"
  }
}

# TXT Records for Domain Verification and Security
resource "aws_route53_record" "domain_verification" {
  zone_id = local.zone_id
  name    = var.domain_name
  type    = "TXT"
  ttl     = 300
  records = [
    "v=trustchain1; ca=trust.hypermesh.online; ct=enabled; dnssec=enabled",
    "hypermesh-verification=${var.environment}-${random_id.verification.hex}"
  ]

  tags = {
    Name        = "trustchain-verification-txt-${var.environment}"
    Environment = var.environment
    Purpose     = "Domain Verification"
  }
}

# CAA Records for Certificate Authority Authorization
resource "aws_route53_record" "caa_records" {
  zone_id = local.zone_id
  name    = var.domain_name
  type    = "CAA"
  ttl     = 300
  records = [
    "0 issue \"amazon.com\"",
    "0 issue \"trustchain.hypermesh.online\"",
    "0 iodef \"mailto:security@hypermesh.online\""
  ]

  tags = {
    Name        = "trustchain-caa-${var.environment}"
    Environment = var.environment
    Purpose     = "Certificate Authority Authorization"
  }
}

# DNSSEC Configuration (if enabled)
resource "aws_route53_key_signing_key" "trustchain" {
  count = var.enable_dnssec ? 1 : 0
  
  hosted_zone_id             = local.zone_id
  key_management_service_arn = aws_kms_key.dnssec[0].arn
  name                       = "trustchain-ksk-${var.environment}"

  tags = {
    Name        = "trustchain-ksk-${var.environment}"
    Environment = var.environment
    Purpose     = "DNSSEC Key Signing"
  }
}

resource "aws_route53_hosted_zone_dnssec" "trustchain" {
  count = var.enable_dnssec ? 1 : 0
  
  hosted_zone_id = local.zone_id
  depends_on     = [aws_route53_key_signing_key.trustchain]

  tags = {
    Name        = "trustchain-dnssec-${var.environment}"
    Environment = var.environment
  }
}

# KMS Key for DNSSEC
resource "aws_kms_key" "dnssec" {
  count = var.enable_dnssec ? 1 : 0
  
  description                = "TrustChain DNSSEC signing key"
  customer_master_key_spec   = "ECC_NIST_P256"
  key_usage                  = "SIGN_VERIFY"
  deletion_window_in_days    = var.environment == "prod" ? 30 : 7

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
        Sid    = "Allow Route 53 DNSSEC Service"
        Effect = "Allow"
        Principal = {
          Service = "dnssec.route53.amazonaws.com"
        }
        Action = [
          "kms:DescribeKey",
          "kms:GetPublicKey",
          "kms:Sign"
        ]
        Resource = "*"
        Condition = {
          StringEquals = {
            "aws:SourceAccount" = data.aws_caller_identity.current.account_id
          }
        }
      }
    ]
  })

  tags = {
    Name        = "trustchain-dnssec-key-${var.environment}"
    Environment = var.environment
    Purpose     = "DNSSEC Signing"
  }
}

# CloudWatch Alarms for DNS Health
resource "aws_cloudwatch_metric_alarm" "dns_health_check" {
  alarm_name          = "trustchain-dns-health-${var.environment}"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "HealthCheckStatus"
  namespace          = "AWS/Route53"
  period             = "60"
  statistic          = "Minimum"
  threshold          = "1"
  alarm_description  = "TrustChain DNS health check failed"

  dimensions = {
    HealthCheckId = aws_route53_health_check.trustchain_https.id
  }

  alarm_actions = compact([var.sns_topic_arn])

  tags = {
    Name        = "trustchain-dns-health-alarm-${var.environment}"
    Environment = var.environment
  }
}

# Query Logging for DNS Analytics
resource "aws_cloudwatch_log_group" "dns_query_logs" {
  count = var.enable_query_logging ? 1 : 0
  
  name              = "/aws/route53/trustchain-${var.environment}"
  retention_in_days = 30

  tags = {
    Name        = "trustchain-dns-query-logs-${var.environment}"
    Environment = var.environment
    Purpose     = "DNS Query Logging"
  }
}

resource "aws_route53_query_log" "trustchain" {
  count = var.enable_query_logging ? 1 : 0
  
  cloudwatch_log_group_arn = aws_cloudwatch_log_group.dns_query_logs[0].arn
  hosted_zone_id          = local.zone_id

  depends_on = [aws_cloudwatch_log_group.dns_query_logs]
}

# CloudWatch Log Metric Filter for DNS Query Analysis
resource "aws_cloudwatch_log_metric_filter" "dns_query_count" {
  count = var.enable_query_logging ? 1 : 0
  
  name           = "trustchain-dns-query-count-${var.environment}"
  log_group_name = aws_cloudwatch_log_group.dns_query_logs[0].name
  pattern        = "[version, account_id, region, vpc_id, query_timestamp, query_name, query_type, query_class, rcode, answer_type, answer_class, answer_rdata, answer_ttl]"

  metric_transformation {
    name      = "DNSQueryCount"
    namespace = "TrustChain/DNS"
    value     = "1"
  }
}

# Data sources
data "aws_region" "current" {}
data "aws_caller_identity" "current" {}

# Random ID for verification
resource "random_id" "verification" {
  byte_length = 8
}