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
    bucket         = "hypermesh-terraform-state"
    key            = "infrastructure/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "terraform-state-lock"
  }
}

# Provider configurations
provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Environment = var.environment
      Project     = "hypermesh"
      ManagedBy   = "terraform"
      Version     = var.version
    }
  }
}

provider "kubernetes" {
  host                   = module.eks.cluster_endpoint
  cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

  exec {
    api_version = "client.authentication.k8s.io/v1beta1"
    command     = "aws"
    args = [
      "eks",
      "get-token",
      "--cluster-name",
      module.eks.cluster_name
    ]
  }
}

provider "helm" {
  kubernetes {
    host                   = module.eks.cluster_endpoint
    cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

    exec {
      api_version = "client.authentication.k8s.io/v1beta1"
      command     = "aws"
      args = [
        "eks",
        "get-token",
        "--cluster-name",
        module.eks.cluster_name
      ]
    }
  }
}

# VPC Module
module "vpc" {
  source = "./modules/vpc"

  name                = "hypermesh-${var.environment}"
  cidr                = var.vpc_cidr
  azs                 = data.aws_availability_zones.available.names
  private_subnets     = var.private_subnet_cidrs
  public_subnets      = var.public_subnet_cidrs

  enable_nat_gateway  = true
  enable_vpn_gateway  = false
  enable_dns_hostnames = true
  enable_dns_support  = true

  enable_ipv6         = true
  assign_ipv6_address_on_creation = true

  tags = {
    "kubernetes.io/cluster/${local.cluster_name}" = "shared"
  }

  public_subnet_tags = {
    "kubernetes.io/cluster/${local.cluster_name}" = "shared"
    "kubernetes.io/role/elb"                       = "1"
  }

  private_subnet_tags = {
    "kubernetes.io/cluster/${local.cluster_name}" = "shared"
    "kubernetes.io/role/internal-elb"             = "1"
  }
}

# EKS Cluster
module "eks" {
  source = "./modules/eks"

  cluster_name    = local.cluster_name
  cluster_version = var.kubernetes_version

  vpc_id          = module.vpc.vpc_id
  subnet_ids      = module.vpc.private_subnets

  # Node groups configuration
  node_groups = {
    general = {
      desired_size = var.node_group_desired_size
      max_size     = var.node_group_max_size
      min_size     = var.node_group_min_size

      instance_types = var.node_instance_types

      k8s_labels = {
        Environment = var.environment
        NodeType    = "general"
      }

      tags = {
        Environment = var.environment
      }
    }

    high_performance = {
      desired_size = 2
      max_size     = 4
      min_size     = 1

      instance_types = ["c6i.2xlarge", "c6i.4xlarge"]

      k8s_labels = {
        Environment = var.environment
        NodeType    = "high-performance"
      }

      taints = [{
        key    = "high-performance"
        value  = "true"
        effect = "NO_SCHEDULE"
      }]

      tags = {
        Environment = var.environment
        NodeType    = "high-performance"
      }
    }
  }

  # Enable IRSA for pod identity
  enable_irsa = true

  # Add-ons
  cluster_addons = {
    coredns = {
      most_recent = true
    }
    kube-proxy = {
      most_recent = true
    }
    vpc-cni = {
      most_recent = true
    }
    ebs-csi-driver = {
      most_recent = true
    }
  }

  tags = {
    Environment = var.environment
  }
}

# RDS for persistent storage
module "rds" {
  source = "./modules/rds"

  identifier     = "hypermesh-${var.environment}"
  engine         = "postgres"
  engine_version = "16.1"
  instance_class = var.rds_instance_type

  allocated_storage     = var.rds_allocated_storage
  max_allocated_storage = var.rds_max_allocated_storage

  database_name = "hypermesh"
  username      = "hypermesh"

  vpc_id             = module.vpc.vpc_id
  subnet_ids         = module.vpc.private_subnets
  allowed_cidr_blocks = module.vpc.private_subnets_cidr_blocks

  backup_retention_period = var.rds_backup_retention_period
  backup_window          = "03:00-04:00"
  maintenance_window     = "sun:04:00-sun:05:00"

  enabled_cloudwatch_logs_exports = ["postgresql"]

  tags = {
    Environment = var.environment
  }
}

# ElastiCache for Redis
module "elasticache" {
  source = "./modules/elasticache"

  cluster_id           = "hypermesh-${var.environment}"
  engine              = "redis"
  node_type           = var.elasticache_node_type
  num_cache_nodes     = var.elasticache_num_nodes
  parameter_group_name = "default.redis7"

  vpc_id             = module.vpc.vpc_id
  subnet_ids         = module.vpc.private_subnets
  allowed_cidr_blocks = module.vpc.private_subnets_cidr_blocks

  snapshot_retention_limit = 5
  snapshot_window         = "03:00-05:00"

  tags = {
    Environment = var.environment
  }
}

# S3 buckets for storage
module "s3" {
  source = "./modules/s3"

  buckets = {
    artifacts = {
      name = "hypermesh-artifacts-${var.environment}-${data.aws_caller_identity.current.account_id}"
      versioning = true
      lifecycle_rules = [{
        id      = "expire-old-versions"
        enabled = true
        expiration = {
          days = 90
        }
      }]
    }

    backups = {
      name = "hypermesh-backups-${var.environment}-${data.aws_caller_identity.current.account_id}"
      versioning = true
      lifecycle_rules = [{
        id      = "transition-to-glacier"
        enabled = true
        transitions = [{
          days          = 30
          storage_class = "GLACIER"
        }]
      }]
    }

    logs = {
      name = "hypermesh-logs-${var.environment}-${data.aws_caller_identity.current.account_id}"
      versioning = false
      lifecycle_rules = [{
        id      = "expire-old-logs"
        enabled = true
        expiration = {
          days = 30
        }
      }]
    }
  }

  tags = {
    Environment = var.environment
  }
}

# Application Load Balancer
module "alb" {
  source = "./modules/alb"

  name               = "hypermesh-${var.environment}"
  load_balancer_type = "application"

  vpc_id  = module.vpc.vpc_id
  subnets = module.vpc.public_subnets

  security_group_rules = {
    ingress_all_http = {
      from_port   = 80
      to_port     = 80
      protocol    = "tcp"
      cidr_blocks = ["0.0.0.0/0"]
    }
    ingress_all_https = {
      from_port   = 443
      to_port     = 443
      protocol    = "tcp"
      cidr_blocks = ["0.0.0.0/0"]
    }
  }

  target_groups = [
    {
      name             = "hypermesh-api"
      backend_protocol = "HTTP"
      backend_port     = 8080
      target_type      = "ip"
      health_check = {
        enabled             = true
        interval            = 30
        path                = "/health"
        port                = "traffic-port"
        healthy_threshold   = 2
        unhealthy_threshold = 2
        timeout             = 5
        protocol            = "HTTP"
        matcher             = "200"
      }
    }
  ]

  tags = {
    Environment = var.environment
  }
}

# CloudFront CDN
module "cloudfront" {
  source = "./modules/cloudfront"

  aliases             = var.cloudfront_aliases
  origin_domain_name  = module.alb.dns_name

  default_cache_behavior = {
    allowed_methods  = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
    cached_methods   = ["GET", "HEAD", "OPTIONS"]
    target_origin_id = "hypermesh-alb"

    forwarded_values = {
      query_string = true
      headers      = ["Host", "Origin", "Accept", "Authorization"]
      cookies = {
        forward = "all"
      }
    }

    viewer_protocol_policy = "redirect-to-https"
    min_ttl                = 0
    default_ttl            = 86400
    max_ttl                = 31536000
  }

  tags = {
    Environment = var.environment
  }
}

# Route53 DNS
module "route53" {
  source = "./modules/route53"

  zone_name = var.domain_name

  records = [
    {
      name    = ""
      type    = "A"
      alias   = {
        name                   = module.cloudfront.domain_name
        zone_id                = module.cloudfront.hosted_zone_id
        evaluate_target_health = false
      }
    },
    {
      name    = "api"
      type    = "CNAME"
      ttl     = 300
      records = [module.alb.dns_name]
    },
    {
      name    = "trust"
      type    = "A"
      ttl     = 300
      records = [module.eks.cluster_endpoint_ip]
    }
  ]

  tags = {
    Environment = var.environment
  }
}

# Monitoring and Alerting
module "monitoring" {
  source = "./modules/monitoring"

  cluster_name = module.eks.cluster_name

  alarms = {
    high_cpu = {
      comparison_operator = "GreaterThanThreshold"
      evaluation_periods  = "2"
      metric_name        = "CPUUtilization"
      namespace          = "AWS/EKS"
      period             = "300"
      statistic          = "Average"
      threshold          = "80"
      alarm_description  = "This metric monitors EKS CPU utilization"
    }

    high_memory = {
      comparison_operator = "GreaterThanThreshold"
      evaluation_periods  = "2"
      metric_name        = "MemoryUtilization"
      namespace          = "AWS/EKS"
      period             = "300"
      statistic          = "Average"
      threshold          = "80"
      alarm_description  = "This metric monitors EKS memory utilization"
    }
  }

  sns_topic_arn = aws_sns_topic.alerts.arn

  tags = {
    Environment = var.environment
  }
}

# SNS Topic for alerts
resource "aws_sns_topic" "alerts" {
  name = "hypermesh-alerts-${var.environment}"

  tags = {
    Environment = var.environment
  }
}

resource "aws_sns_topic_subscription" "alerts_email" {
  topic_arn = aws_sns_topic.alerts.arn
  protocol  = "email"
  endpoint  = var.alert_email
}

# Data sources
data "aws_availability_zones" "available" {
  state = "available"
}

data "aws_caller_identity" "current" {}

# Locals
locals {
  cluster_name = "hypermesh-${var.environment}"
}

# Outputs
output "cluster_endpoint" {
  value = module.eks.cluster_endpoint
}

output "cluster_name" {
  value = module.eks.cluster_name
}

output "alb_dns" {
  value = module.alb.dns_name
}

output "cloudfront_domain" {
  value = module.cloudfront.domain_name
}

output "rds_endpoint" {
  value     = module.rds.endpoint
  sensitive = true
}

output "redis_endpoint" {
  value     = module.elasticache.endpoint
  sensitive = true
}