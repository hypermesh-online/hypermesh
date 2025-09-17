# Security Module - Security Groups and WAF for TrustChain

# Security Group for TrustChain EC2 Instances
resource "aws_security_group" "trustchain" {
  name        = "trustchain-instances-${var.environment}"
  description = "Security group for TrustChain EC2 instances"
  vpc_id      = var.vpc_id

  # Certificate Authority API (8443)
  ingress {
    from_port        = 8443
    to_port          = 8443
    protocol         = "tcp"
    ipv6_cidr_blocks = ["::/0"]
    description      = "TrustChain CA API (HTTPS)"
  }

  # Certificate Transparency (6962)
  ingress {
    from_port        = 6962
    to_port          = 6962
    protocol         = "tcp"
    ipv6_cidr_blocks = ["::/0"]
    description      = "Certificate Transparency API"
  }

  # DNS-over-QUIC (8853)
  ingress {
    from_port        = 8853
    to_port          = 8853
    protocol         = "udp"
    ipv6_cidr_blocks = ["::/0"]
    description      = "DNS-over-QUIC"
  }

  # STOQ Protocol (8444)
  ingress {
    from_port        = 8444
    to_port          = 8444
    protocol         = "udp"
    ipv6_cidr_blocks = ["::/0"]
    description      = "STOQ Protocol Endpoint"
  }

  # HyperMesh Integration (8445)
  ingress {
    from_port        = 8445
    to_port          = 8445
    protocol         = "tcp"
    ipv6_cidr_blocks = ["::/0"]
    description      = "HyperMesh Integration API"
  }

  # Integration API (8446)
  ingress {
    from_port        = 8446
    to_port          = 8446
    protocol         = "tcp"
    ipv6_cidr_blocks = ["::/0"]
    description      = "Integration API Endpoint"
  }

  # SSH for administration (restricted)
  ingress {
    from_port        = 22
    to_port          = 22
    protocol         = "tcp"
    ipv6_cidr_blocks = ["2001:db8::/32"] # Restrict to organization IPv6 range
    description      = "SSH access (restricted)"
  }

  # Health check from load balancer
  ingress {
    from_port       = 8080
    to_port         = 8080
    protocol        = "tcp"
    security_groups = [aws_security_group.alb.id]
    description     = "Health check from ALB"
  }

  # All outbound traffic
  egress {
    from_port        = 0
    to_port          = 0
    protocol         = "-1"
    ipv6_cidr_blocks = ["::/0"]
    description      = "All outbound traffic"
  }

  tags = {
    Name        = "trustchain-instances-sg-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain EC2 Instance Security"
  }
}

# Security Group for Application Load Balancer
resource "aws_security_group" "alb" {
  name        = "trustchain-alb-${var.environment}"
  description = "Security group for TrustChain Application Load Balancer"
  vpc_id      = var.vpc_id

  # HTTPS traffic for all TrustChain services
  dynamic "ingress" {
    for_each = {
      for name, service in var.services : name => service
      if service.protocol == "HTTPS"
    }
    content {
      from_port        = ingress.value.port
      to_port          = ingress.value.port
      protocol         = "tcp"
      ipv6_cidr_blocks = ["::/0"]
      description      = "TrustChain ${ingress.key} (${ingress.value.port})"
    }
  }

  # UDP traffic for DNS and STOQ
  dynamic "ingress" {
    for_each = {
      for name, service in var.services : name => service
      if service.protocol == "UDP"
    }
    content {
      from_port        = ingress.value.port
      to_port          = ingress.value.port
      protocol         = "udp"
      ipv6_cidr_blocks = ["::/0"]
      description      = "TrustChain ${ingress.key} (${ingress.value.port})"
    }
  }

  # All outbound traffic
  egress {
    from_port        = 0
    to_port          = 0
    protocol         = "-1"
    ipv6_cidr_blocks = ["::/0"]
    description      = "All outbound traffic"
  }

  tags = {
    Name        = "trustchain-alb-sg-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain Load Balancer Security"
  }
}

# WAF Web ACL for Certificate Authority Protection
resource "aws_wafv2_web_acl" "trustchain" {
  name        = "trustchain-waf-${var.environment}"
  description = "WAF for TrustChain Certificate Authority"
  scope       = "REGIONAL"

  default_action {
    allow {}
  }

  # Rate limiting rule
  rule {
    name     = "RateLimitRule"
    priority = 1

    override_action {
      none {}
    }

    statement {
      rate_based_statement {
        limit              = 1000
        aggregate_key_type = "IP"
      }
    }

    visibility_config {
      cloudwatch_metrics_enabled = true
      metric_name                = "TrustChainRateLimit"
      sampled_requests_enabled   = true
    }

    action {
      block {}
    }
  }

  # Block known malicious IPs
  rule {
    name     = "AWSManagedRulesKnownBadInputsRuleSet"
    priority = 2

    override_action {
      none {}
    }

    statement {
      managed_rule_group_statement {
        name        = "AWSManagedRulesKnownBadInputsRuleSet"
        vendor_name = "AWS"
      }
    }

    visibility_config {
      cloudwatch_metrics_enabled = true
      metric_name                = "TrustChainKnownBadInputs"
      sampled_requests_enabled   = true
    }
  }

  # SQL injection protection
  rule {
    name     = "AWSManagedRulesSQLiRuleSet"
    priority = 3

    override_action {
      none {}
    }

    statement {
      managed_rule_group_statement {
        name        = "AWSManagedRulesSQLiRuleSet"
        vendor_name = "AWS"
      }
    }

    visibility_config {
      cloudwatch_metrics_enabled = true
      metric_name                = "TrustChainSQLiProtection"
      sampled_requests_enabled   = true
    }
  }

  # Core rule set
  rule {
    name     = "AWSManagedRulesCommonRuleSet"
    priority = 4

    override_action {
      none {}
    }

    statement {
      managed_rule_group_statement {
        name        = "AWSManagedRulesCommonRuleSet"
        vendor_name = "AWS"

        # Exclude rules that might block legitimate certificate requests
        excluded_rule {
          name = "SizeRestrictions_BODY"
        }
        excluded_rule {
          name = "GenericRFI_BODY"
        }
      }
    }

    visibility_config {
      cloudwatch_metrics_enabled = true
      metric_name                = "TrustChainCommonRules"
      sampled_requests_enabled   = true
    }
  }

  # Custom rule for certificate-specific protection
  rule {
    name     = "CertificateOperationProtection"
    priority = 5

    action {
      allow {}
    }

    statement {
      and_statement {
        statement {
          byte_match_statement {
            search_string = "certificate"
            field_to_match {
              uri_path {}
            }
            text_transformation {
              priority = 0
              type     = "LOWERCASE"
            }
            positional_constraint = "CONTAINS"
          }
        }
        statement {
          size_constraint_statement {
            field_to_match {
              body {}
            }
            comparison_operator = "LE"
            size                = 65536 # 64KB max for certificate requests
            text_transformation {
              priority = 0
              type     = "NONE"
            }
          }
        }
      }
    }

    visibility_config {
      cloudwatch_metrics_enabled = true
      metric_name                = "TrustChainCertificateOps"
      sampled_requests_enabled   = true
    }
  }

  tags = {
    Name        = "trustchain-waf-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain Certificate Authority Protection"
  }

  visibility_config {
    cloudwatch_metrics_enabled = true
    metric_name                = "TrustChainWAF"
    sampled_requests_enabled   = true
  }
}

# CloudWatch Log Group for WAF
resource "aws_cloudwatch_log_group" "waf" {
  name              = "/aws/wafv2/trustchain-${var.environment}"
  retention_in_days = 30

  tags = {
    Name        = "trustchain-waf-logs-${var.environment}"
    Environment = var.environment
  }
}

# WAF Logging Configuration
resource "aws_wafv2_web_acl_logging_configuration" "trustchain" {
  resource_arn            = aws_wafv2_web_acl.trustchain.arn
  log_destination_configs = [aws_cloudwatch_log_group.waf.arn]

  redacted_field {
    single_header {
      name = "authorization"
    }
  }

  redacted_field {
    single_header {
      name = "x-api-key"
    }
  }
}

# IAM Role for GuardDuty (if enabled)
resource "aws_iam_role" "guardduty" {
  count = var.enable_guardduty ? 1 : 0
  name  = "trustchain-guardduty-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "guardduty.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name        = "trustchain-guardduty-role-${var.environment}"
    Environment = var.environment
  }
}

# GuardDuty Detector
resource "aws_guardduty_detector" "trustchain" {
  count  = var.enable_guardduty ? 1 : 0
  enable = true

  datasources {
    s3_logs {
      enable = true
    }
    kubernetes {
      audit_logs {
        enable = false # Not using Kubernetes
      }
    }
    malware_protection {
      scan_ec2_instance_with_findings {
        ebs_volumes {
          enable = true
        }
      }
    }
  }

  tags = {
    Name        = "trustchain-guardduty-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain Threat Detection"
  }
}

# SNS Topic for Security Alerts
resource "aws_sns_topic" "security_alerts" {
  name = "trustchain-security-alerts-${var.environment}"

  tags = {
    Name        = "trustchain-security-alerts-${var.environment}"
    Environment = var.environment
    Purpose     = "Security Alert Notifications"
  }
}

# CloudWatch Event Rule for GuardDuty Findings
resource "aws_cloudwatch_event_rule" "guardduty_findings" {
  count       = var.enable_guardduty ? 1 : 0
  name        = "trustchain-guardduty-findings-${var.environment}"
  description = "Capture GuardDuty findings"

  event_pattern = jsonencode({
    source      = ["aws.guardduty"]
    detail-type = ["GuardDuty Finding"]
    detail = {
      severity = [4.0, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 5.0, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 6.0, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 6.9, 7.0, 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8, 7.9, 8.0, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8, 8.9, 9.0, 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.9, 10.0]
    }
  })

  tags = {
    Name        = "trustchain-guardduty-rule-${var.environment}"
    Environment = var.environment
  }
}

# CloudWatch Event Target for SNS
resource "aws_cloudwatch_event_target" "sns" {
  count     = var.enable_guardduty ? 1 : 0
  rule      = aws_cloudwatch_event_rule.guardduty_findings[0].name
  target_id = "SendToSNS"
  arn       = aws_sns_topic.security_alerts.arn
}

# SNS Topic Policy
resource "aws_sns_topic_policy" "security_alerts" {
  arn = aws_sns_topic.security_alerts.arn

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Principal = {
          Service = "events.amazonaws.com"
        }
        Action   = "SNS:Publish"
        Resource = aws_sns_topic.security_alerts.arn
      }
    ]
  })
}