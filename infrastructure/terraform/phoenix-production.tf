terraform {
  required_version = ">= 1.5.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.23"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.11"
    }
  }

  backend "s3" {
    bucket         = "phoenix-terraform-state"
    key            = "production/infrastructure.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "phoenix-terraform-locks"
  }
}

# Variables
variable "environment" {
  description = "Deployment environment"
  type        = string
  default     = "production"
}

variable "phoenix_version" {
  description = "Phoenix SDK version"
  type        = string
  default     = "v1.0.0"
}

variable "aws_region" {
  description = "AWS region for deployment"
  type        = string
  default     = "us-east-1"
}

# Data sources
data "aws_availability_zones" "available" {
  state = "available"
}

data "aws_caller_identity" "current" {}

# Locals
locals {
  cluster_name = "phoenix-${var.environment}"
  common_tags = {
    Environment = var.environment
    Project     = "Phoenix"
    Version     = var.phoenix_version
    ManagedBy   = "Terraform"
  }
}

# VPC Configuration
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  name = "${local.cluster_name}-vpc"
  cidr = "10.0.0.0/16"

  azs             = slice(data.aws_availability_zones.available.names, 0, 3)
  private_subnets = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
  public_subnets  = ["10.0.101.0/24", "10.0.102.0/24", "10.0.103.0/24"]

  enable_nat_gateway   = true
  single_nat_gateway   = false
  enable_dns_hostnames = true
  enable_dns_support   = true

  # Enable IPv6
  enable_ipv6                     = true
  assign_ipv6_address_on_creation = true
  private_subnet_ipv6_prefixes    = [0, 1, 2]
  public_subnet_ipv6_prefixes     = [3, 4, 5]

  # Kubernetes tags
  public_subnet_tags = {
    "kubernetes.io/role/elb"                    = "1"
    "kubernetes.io/cluster/${local.cluster_name}" = "shared"
  }

  private_subnet_tags = {
    "kubernetes.io/role/internal-elb"           = "1"
    "kubernetes.io/cluster/${local.cluster_name}" = "shared"
  }

  tags = local.common_tags
}

# EKS Cluster
module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "~> 19.0"

  cluster_name    = local.cluster_name
  cluster_version = "1.28"

  cluster_endpoint_private_access = true
  cluster_endpoint_public_access  = true

  vpc_id     = module.vpc.vpc_id
  subnet_ids = module.vpc.private_subnets

  # Enable IRSA
  enable_irsa = true

  # Node Groups
  eks_managed_node_groups = {
    # General purpose nodes
    general = {
      desired_size = 3
      min_size     = 2
      max_size     = 10

      instance_types = ["t3.large", "t3a.large"]
      capacity_type  = "SPOT"

      labels = {
        Environment = var.environment
        NodeType    = "general"
      }

      tags = merge(local.common_tags, {
        NodeType = "general"
      })
    }

    # High-performance nodes for Phoenix Transport
    performance = {
      desired_size = 2
      min_size     = 1
      max_size     = 5

      instance_types = ["c6i.2xlarge"]
      capacity_type  = "ON_DEMAND"

      labels = {
        Environment = var.environment
        NodeType    = "performance"
        Workload    = "phoenix-transport"
      }

      taints = [{
        key    = "workload"
        value  = "phoenix-transport"
        effect = "NO_SCHEDULE"
      }]

      tags = merge(local.common_tags, {
        NodeType = "performance"
      })
    }

    # GPU nodes for computation
    gpu = {
      desired_size = 0
      min_size     = 0
      max_size     = 3

      instance_types = ["g4dn.xlarge"]
      capacity_type  = "SPOT"

      labels = {
        Environment = var.environment
        NodeType    = "gpu"
        Workload    = "computation"
      }

      taints = [{
        key    = "nvidia.com/gpu"
        value  = "true"
        effect = "NO_SCHEDULE"
      }]

      tags = merge(local.common_tags, {
        NodeType = "gpu"
      })
    }
  }

  # Cluster addons
  cluster_addons = {
    coredns = {
      most_recent = true
    }
    kube-proxy = {
      most_recent = true
    }
    vpc-cni = {
      most_recent = true
      configuration_values = jsonencode({
        env = {
          ENABLE_PREFIX_DELEGATION = "true"
          WARM_PREFIX_TARGET       = "1"
        }
      })
    }
    aws-ebs-csi-driver = {
      most_recent = true
    }
  }

  tags = local.common_tags
}

# RDS PostgreSQL for metadata storage
module "rds" {
  source  = "terraform-aws-modules/rds/aws"
  version = "~> 6.0"

  identifier = "${local.cluster_name}-postgres"

  engine               = "postgres"
  engine_version       = "16.1"
  family              = "postgres16"
  major_engine_version = "16"
  instance_class       = "db.r6i.xlarge"

  allocated_storage     = 100
  max_allocated_storage = 1000
  storage_encrypted     = true

  db_name  = "phoenix"
  username = "phoenix_admin"
  port     = 5432

  vpc_security_group_ids = [aws_security_group.rds.id]
  db_subnet_group_name   = aws_db_subnet_group.rds.name

  backup_retention_period = 30
  backup_window          = "03:00-04:00"
  maintenance_window     = "sun:04:00-sun:05:00"

  enabled_cloudwatch_logs_exports = ["postgresql"]

  performance_insights_enabled = true
  performance_insights_retention_period = 7

  tags = local.common_tags
}

resource "aws_db_subnet_group" "rds" {
  name       = "${local.cluster_name}-rds"
  subnet_ids = module.vpc.private_subnets

  tags = local.common_tags
}

resource "aws_security_group" "rds" {
  name_prefix = "${local.cluster_name}-rds"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = [module.vpc.vpc_cidr_block]
  }

  tags = local.common_tags
}

# ElastiCache Redis for caching and queuing
module "elasticache" {
  source  = "terraform-aws-modules/elasticache/aws"
  version = "~> 1.0"

  cluster_id = "${local.cluster_name}-redis"

  engine          = "redis"
  engine_version  = "7.1"
  node_type       = "cache.r7g.xlarge"
  num_cache_nodes = 3

  parameter_group_family = "redis7"
  port                  = 6379

  subnet_ids = module.vpc.private_subnets
  vpc_id     = module.vpc.vpc_id

  snapshot_retention_limit = 7
  snapshot_window         = "03:00-05:00"

  tags = local.common_tags
}

# S3 Buckets
resource "aws_s3_bucket" "phoenix_storage" {
  for_each = toset([
    "artifacts",
    "backups",
    "logs",
    "data"
  ])

  bucket = "${local.cluster_name}-${each.key}-${data.aws_caller_identity.current.account_id}"

  tags = merge(local.common_tags, {
    Purpose = each.key
  })
}

resource "aws_s3_bucket_versioning" "phoenix_storage" {
  for_each = aws_s3_bucket.phoenix_storage

  bucket = each.value.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_encryption" "phoenix_storage" {
  for_each = aws_s3_bucket.phoenix_storage

  bucket = each.value.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

# ECR Repositories
resource "aws_ecr_repository" "phoenix" {
  for_each = toset([
    "phoenix-transport",
    "phoenix-certs",
    "phoenix-dashboard",
    "phoenix-cli"
  ])

  name                 = each.key
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }

  encryption_configuration {
    encryption_type = "AES256"
  }

  tags = local.common_tags
}

resource "aws_ecr_lifecycle_policy" "phoenix" {
  for_each = aws_ecr_repository.phoenix

  repository = each.value.name

  policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last 10 images"
        selection = {
          tagStatus     = "any"
          countType     = "imageCountMoreThan"
          countNumber   = 10
        }
        action = {
          type = "expire"
        }
      }
    ]
  })
}

# Application Load Balancer
resource "aws_lb" "phoenix" {
  name               = "${local.cluster_name}-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb.id]
  subnets           = module.vpc.public_subnets

  enable_deletion_protection = true
  enable_http2              = true
  enable_cross_zone_load_balancing = true

  tags = local.common_tags
}

resource "aws_security_group" "alb" {
  name_prefix = "${local.cluster_name}-alb"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }

  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }

  tags = local.common_tags
}

# Network Load Balancer for Phoenix Transport (QUIC/UDP)
resource "aws_lb" "phoenix_transport" {
  name               = "${local.cluster_name}-nlb"
  internal           = false
  load_balancer_type = "network"
  subnets           = module.vpc.public_subnets

  enable_deletion_protection = true
  enable_cross_zone_load_balancing = true

  tags = local.common_tags
}

resource "aws_lb_target_group" "phoenix_transport" {
  name     = "${local.cluster_name}-transport"
  port     = 9292
  protocol = "UDP"
  vpc_id   = module.vpc.vpc_id

  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 2
    interval            = 30
    port                = 9293
    protocol            = "TCP"
  }

  tags = local.common_tags
}

resource "aws_lb_listener" "phoenix_transport" {
  load_balancer_arn = aws_lb.phoenix_transport.arn
  port              = "9292"
  protocol          = "UDP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.phoenix_transport.arn
  }
}

# CloudFront CDN
resource "aws_cloudfront_distribution" "phoenix" {
  enabled             = true
  is_ipv6_enabled    = true
  comment            = "Phoenix SDK CDN"
  default_root_object = "index.html"

  origin {
    domain_name = aws_lb.phoenix.dns_name
    origin_id   = "phoenix-alb"

    custom_origin_config {
      http_port              = 80
      https_port             = 443
      origin_protocol_policy = "https-only"
      origin_ssl_protocols   = ["TLSv1.2"]
    }
  }

  default_cache_behavior {
    allowed_methods  = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
    cached_methods   = ["GET", "HEAD", "OPTIONS"]
    target_origin_id = "phoenix-alb"

    forwarded_values {
      query_string = true
      headers      = ["Host", "Origin", "Access-Control-Request-Method", "Access-Control-Request-Headers"]

      cookies {
        forward = "all"
      }
    }

    viewer_protocol_policy = "redirect-to-https"
    min_ttl                = 0
    default_ttl            = 86400
    max_ttl                = 31536000
    compress               = true
  }

  price_class = "PriceClass_100"

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    cloudfront_default_certificate = true
  }

  tags = local.common_tags
}

# CloudWatch Log Groups
resource "aws_cloudwatch_log_group" "phoenix" {
  for_each = toset([
    "phoenix-transport",
    "phoenix-certs",
    "phoenix-dashboard",
    "phoenix-cli"
  ])

  name              = "/aws/phoenix/${each.key}"
  retention_in_days = 30

  tags = local.common_tags
}

# CloudWatch Alarms
resource "aws_cloudwatch_metric_alarm" "cluster_cpu" {
  alarm_name          = "${local.cluster_name}-high-cpu"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "CPUUtilization"
  namespace          = "AWS/EKS"
  period             = "300"
  statistic          = "Average"
  threshold          = "80"
  alarm_description  = "This metric monitors EKS cluster CPU utilization"

  dimensions = {
    ClusterName = module.eks.cluster_name
  }

  alarm_actions = [aws_sns_topic.alerts.arn]

  tags = local.common_tags
}

resource "aws_cloudwatch_metric_alarm" "cluster_memory" {
  alarm_name          = "${local.cluster_name}-high-memory"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "node_memory_utilization"
  namespace          = "ContainerInsights"
  period             = "300"
  statistic          = "Average"
  threshold          = "80"
  alarm_description  = "This metric monitors EKS cluster memory utilization"

  dimensions = {
    ClusterName = module.eks.cluster_name
  }

  alarm_actions = [aws_sns_topic.alerts.arn]

  tags = local.common_tags
}

# SNS Topic for Alerts
resource "aws_sns_topic" "alerts" {
  name = "${local.cluster_name}-alerts"

  tags = local.common_tags
}

resource "aws_sns_topic_subscription" "alerts_email" {
  topic_arn = aws_sns_topic.alerts.arn
  protocol  = "email"
  endpoint  = "ops@phoenix-distributed.com"
}

# IAM Roles for Service Accounts (IRSA)
module "phoenix_transport_irsa" {
  source  = "terraform-aws-modules/iam/aws//modules/iam-role-for-service-accounts-eks"
  version = "~> 5.0"

  role_name = "${local.cluster_name}-phoenix-transport"

  attach_external_secrets_policy = true

  oidc_providers = {
    main = {
      provider_arn               = module.eks.oidc_provider_arn
      namespace_service_accounts = ["phoenix-system:phoenix-transport"]
    }
  }

  tags = local.common_tags
}

# Outputs
output "cluster_endpoint" {
  description = "EKS cluster endpoint"
  value       = module.eks.cluster_endpoint
}

output "cluster_name" {
  description = "EKS cluster name"
  value       = module.eks.cluster_name
}

output "alb_dns_name" {
  description = "ALB DNS name"
  value       = aws_lb.phoenix.dns_name
}

output "nlb_dns_name" {
  description = "NLB DNS name for Phoenix Transport"
  value       = aws_lb.phoenix_transport.dns_name
}

output "cloudfront_domain" {
  description = "CloudFront distribution domain"
  value       = aws_cloudfront_distribution.phoenix.domain_name
}

output "rds_endpoint" {
  description = "RDS PostgreSQL endpoint"
  value       = module.rds.db_instance_endpoint
  sensitive   = true
}

output "redis_endpoint" {
  description = "ElastiCache Redis endpoint"
  value       = module.elasticache.primary_endpoint_address
  sensitive   = true
}

output "ecr_repositories" {
  description = "ECR repository URLs"
  value = {
    for k, v in aws_ecr_repository.phoenix : k => v.repository_url
  }
}