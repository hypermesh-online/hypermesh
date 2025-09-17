# Load Balancer Module - IPv6-Only Application Load Balancer

# Application Load Balancer
resource "aws_lb" "trustchain" {
  name               = "trustchain-alb-${var.environment}"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [var.security_group_id]
  subnets           = var.subnet_ids
  ip_address_type   = "ipv6"

  enable_deletion_protection       = var.environment == "prod" ? true : false
  enable_cross_zone_load_balancing = true
  enable_http2                    = true

  access_logs {
    bucket  = aws_s3_bucket.alb_access_logs.bucket
    prefix  = "trustchain-alb"
    enabled = true
  }

  tags = {
    Name        = "trustchain-alb-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain Load Balancer"
  }

  depends_on = [aws_s3_bucket_policy.alb_access_logs]
}

# Target Groups for different services
resource "aws_lb_target_group" "ca" {
  name     = "trustchain-ca-${var.environment}"
  port     = var.services.ca.port
  protocol = "HTTPS"
  vpc_id   = var.vpc_id
  ip_address_type = "ipv6"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 2
    timeout             = 5
    interval            = 30
    path                = var.services.ca.health_path
    matcher             = "200"
    port                = "traffic-port"
    protocol            = "HTTPS"
  }

  stickiness {
    type            = "lb_cookie"
    cookie_duration = 86400 # 24 hours
    enabled         = true
  }

  tags = {
    Name        = "trustchain-ca-tg-${var.environment}"
    Environment = var.environment
    Service     = "Certificate Authority"
  }
}

resource "aws_lb_target_group" "ct" {
  name     = "trustchain-ct-${var.environment}"
  port     = var.services.ct.port
  protocol = "HTTPS"
  vpc_id   = var.vpc_id
  ip_address_type = "ipv6"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 2
    timeout             = 5
    interval            = 30
    path                = var.services.ct.health_path
    matcher             = "200"
    port                = "traffic-port"
    protocol            = "HTTPS"
  }

  tags = {
    Name        = "trustchain-ct-tg-${var.environment}"
    Environment = var.environment
    Service     = "Certificate Transparency"
  }
}

resource "aws_lb_target_group" "hypermesh" {
  name     = "trustchain-hypermesh-${var.environment}"
  port     = var.services.hypermesh.port
  protocol = "HTTPS"
  vpc_id   = var.vpc_id
  ip_address_type = "ipv6"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 2
    timeout             = 5
    interval            = 30
    path                = "/health"
    matcher             = "200"
    port                = "traffic-port"
    protocol            = "HTTPS"
  }

  tags = {
    Name        = "trustchain-hypermesh-tg-${var.environment}"
    Environment = var.environment
    Service     = "HyperMesh Integration"
  }
}

resource "aws_lb_target_group" "integration" {
  name     = "trustchain-integration-${var.environment}"
  port     = var.services.integration.port
  protocol = "HTTPS"
  vpc_id   = var.vpc_id
  ip_address_type = "ipv6"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 2
    timeout             = 5
    interval            = 30
    path                = "/health"
    matcher             = "200"
    port                = "traffic-port"
    protocol            = "HTTPS"
  }

  tags = {
    Name        = "trustchain-integration-tg-${var.environment}"
    Environment = var.environment
    Service     = "Integration API"
  }
}

# Network Load Balancer for UDP services (DNS and STOQ)
resource "aws_lb" "trustchain_nlb" {
  name               = "trustchain-nlb-${var.environment}"
  internal           = false
  load_balancer_type = "network"
  subnets           = var.subnet_ids
  ip_address_type   = "ipv6"

  enable_deletion_protection       = var.environment == "prod" ? true : false
  enable_cross_zone_load_balancing = true

  tags = {
    Name        = "trustchain-nlb-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain Network Load Balancer (UDP)"
  }
}

# Target Groups for UDP services
resource "aws_lb_target_group" "dns" {
  name     = "trustchain-dns-${var.environment}"
  port     = var.services.dns.port
  protocol = "UDP"
  vpc_id   = var.vpc_id
  ip_address_type = "ipv6"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 2
    timeout             = 6
    interval            = 10
    port                = 8080
    protocol            = "HTTP"
    path                = "/health"
  }

  tags = {
    Name        = "trustchain-dns-tg-${var.environment}"
    Environment = var.environment
    Service     = "DNS-over-QUIC"
  }
}

resource "aws_lb_target_group" "stoq" {
  name     = "trustchain-stoq-${var.environment}"
  port     = var.services.stoq.port
  protocol = "UDP"
  vpc_id   = var.vpc_id
  ip_address_type = "ipv6"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 2
    timeout             = 6
    interval            = 10
    port                = 8080
    protocol            = "HTTP"
    path                = "/health"
  }

  tags = {
    Name        = "trustchain-stoq-tg-${var.environment}"
    Environment = var.environment
    Service     = "STOQ Protocol"
  }
}

# ALB Listeners for HTTPS services
resource "aws_lb_listener" "ca" {
  load_balancer_arn = aws_lb.trustchain.arn
  port              = var.services.ca.port
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.ssl_certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.ca.arn
  }

  tags = {
    Name        = "trustchain-ca-listener-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_lb_listener" "ct" {
  load_balancer_arn = aws_lb.trustchain.arn
  port              = var.services.ct.port
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.ssl_certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.ct.arn
  }

  tags = {
    Name        = "trustchain-ct-listener-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_lb_listener" "hypermesh" {
  load_balancer_arn = aws_lb.trustchain.arn
  port              = var.services.hypermesh.port
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.ssl_certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.hypermesh.arn
  }

  tags = {
    Name        = "trustchain-hypermesh-listener-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_lb_listener" "integration" {
  load_balancer_arn = aws_lb.trustchain.arn
  port              = var.services.integration.port
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.ssl_certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.integration.arn
  }

  tags = {
    Name        = "trustchain-integration-listener-${var.environment}"
    Environment = var.environment
  }
}

# NLB Listeners for UDP services
resource "aws_lb_listener" "dns" {
  load_balancer_arn = aws_lb.trustchain_nlb.arn
  port              = var.services.dns.port
  protocol          = "UDP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.dns.arn
  }

  tags = {
    Name        = "trustchain-dns-listener-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_lb_listener" "stoq" {
  load_balancer_arn = aws_lb.trustchain_nlb.arn
  port              = var.services.stoq.port
  protocol          = "UDP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.stoq.arn
  }

  tags = {
    Name        = "trustchain-stoq-listener-${var.environment}"
    Environment = var.environment
  }
}

# Target Group Attachments (instances will be attached by Auto Scaling Group)
resource "aws_autoscaling_attachment" "ca" {
  autoscaling_group_name = var.autoscaling_group_name
  lb_target_group_arn   = aws_lb_target_group.ca.arn
}

resource "aws_autoscaling_attachment" "ct" {
  autoscaling_group_name = var.autoscaling_group_name
  lb_target_group_arn   = aws_lb_target_group.ct.arn
}

resource "aws_autoscaling_attachment" "hypermesh" {
  autoscaling_group_name = var.autoscaling_group_name
  lb_target_group_arn   = aws_lb_target_group.hypermesh.arn
}

resource "aws_autoscaling_attachment" "integration" {
  autoscaling_group_name = var.autoscaling_group_name
  lb_target_group_arn   = aws_lb_target_group.integration.arn
}

resource "aws_autoscaling_attachment" "dns" {
  autoscaling_group_name = var.autoscaling_group_name
  lb_target_group_arn   = aws_lb_target_group.dns.arn
}

resource "aws_autoscaling_attachment" "stoq" {
  autoscaling_group_name = var.autoscaling_group_name
  lb_target_group_arn   = aws_lb_target_group.stoq.arn
}

# CloudWatch Alarms for Load Balancer Health
resource "aws_cloudwatch_metric_alarm" "alb_target_response_time" {
  alarm_name          = "trustchain-alb-response-time-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "TargetResponseTime"
  namespace          = "AWS/ApplicationELB"
  period             = "300"
  statistic          = "Average"
  threshold          = "1.0"
  alarm_description  = "This metric monitors ALB target response time"

  dimensions = {
    LoadBalancer = aws_lb.trustchain.arn_suffix
  }

  tags = {
    Name        = "trustchain-alb-response-time-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_cloudwatch_metric_alarm" "alb_healthy_host_count" {
  alarm_name          = "trustchain-alb-healthy-hosts-${var.environment}"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "HealthyHostCount"
  namespace          = "AWS/ApplicationELB"
  period             = "60"
  statistic          = "Average"
  threshold          = "1"
  alarm_description  = "This metric monitors healthy target count"

  dimensions = {
    TargetGroup  = aws_lb_target_group.ca.arn_suffix
    LoadBalancer = aws_lb.trustchain.arn_suffix
  }

  tags = {
    Name        = "trustchain-alb-healthy-hosts-${var.environment}"
    Environment = var.environment
  }
}

# WAF Association with ALB
resource "aws_wafv2_web_acl_association" "trustchain" {
  resource_arn = aws_lb.trustchain.arn
  web_acl_arn  = var.waf_web_acl_arn
}

# Access Logs S3 Bucket
resource "aws_s3_bucket" "alb_access_logs" {
  bucket = "trustchain-alb-access-logs-${var.environment}-${random_id.bucket_suffix.hex}"

  tags = {
    Name        = "trustchain-alb-access-logs-${var.environment}"
    Environment = var.environment
    Purpose     = "ALB Access Logs"
  }
}

resource "aws_s3_bucket_versioning" "alb_access_logs" {
  bucket = aws_s3_bucket.alb_access_logs.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_encryption" "alb_access_logs" {
  bucket = aws_s3_bucket.alb_access_logs.id

  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        sse_algorithm = "AES256"
      }
    }
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "alb_access_logs" {
  bucket = aws_s3_bucket.alb_access_logs.id

  rule {
    id     = "delete_old_logs"
    status = "Enabled"

    expiration {
      days = 90
    }

    noncurrent_version_expiration {
      noncurrent_days = 30
    }
  }
}

resource "aws_s3_bucket_policy" "alb_access_logs" {
  bucket = aws_s3_bucket.alb_access_logs.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Principal = {
          AWS = data.aws_elb_service_account.main.arn
        }
        Action   = "s3:PutObject"
        Resource = "${aws_s3_bucket.alb_access_logs.arn}/AWSLogs/${data.aws_caller_identity.current.account_id}/*"
      },
      {
        Effect = "Allow"
        Principal = {
          Service = "delivery.logs.amazonaws.com"
        }
        Action   = "s3:PutObject"
        Resource = "${aws_s3_bucket.alb_access_logs.arn}/AWSLogs/${data.aws_caller_identity.current.account_id}/*"
        Condition = {
          StringEquals = {
            "s3:x-amz-acl" = "bucket-owner-full-control"
          }
        }
      },
      {
        Effect = "Allow"
        Principal = {
          Service = "delivery.logs.amazonaws.com"
        }
        Action   = "s3:GetBucketAcl"
        Resource = aws_s3_bucket.alb_access_logs.arn
      }
    ]
  })
}


# Data sources
data "aws_caller_identity" "current" {}
data "aws_elb_service_account" "main" {}

# Random ID for bucket naming
resource "random_id" "bucket_suffix" {
  byte_length = 4
}