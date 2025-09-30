# TrustChain Production Infrastructure - Terraform Configuration
# Multi-region federated certificate authority with enterprise-grade reliability

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
    bucket         = "trustchain-terraform-state"
    key            = "production/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "trustchain-terraform-locks"
  }
}

# Variables
variable "environment" {
  description = "Deployment environment"
  type        = string
  default     = "production"
}

variable "regions" {
  description = "AWS regions for multi-region deployment"
  type        = list(string)
  default     = ["us-east-1", "eu-west-1", "ap-southeast-1"]
}

variable "domain" {
  description = "Primary domain for TrustChain"
  type        = string
  default     = "trust.hypermesh.online"
}

variable "ca_replicas" {
  description = "Number of CA replicas per region"
  type        = number
  default     = 3
}

variable "ct_log_servers" {
  description = "Number of CT log servers"
  type        = number
  default     = 3
}

variable "dns_servers" {
  description = "Number of DNS servers"
  type        = number
  default     = 3
}

# Providers for multi-region
provider "aws" {
  alias  = "us-east-1"
  region = "us-east-1"
}

provider "aws" {
  alias  = "eu-west-1"
  region = "eu-west-1"
}

provider "aws" {
  alias  = "ap-southeast-1"
  region = "ap-southeast-1"
}

# Data sources
data "aws_caller_identity" "current" {}

data "aws_availability_zones" "available" {
  for_each = toset(var.regions)
  provider = aws
  state    = "available"
}

# VPC Module for each region
module "vpc" {
  for_each = toset(var.regions)
  source   = "./modules/vpc"

  providers = {
    aws = aws
  }

  environment         = var.environment
  region             = each.value
  availability_zones = data.aws_availability_zones.available[each.key].names
  cidr_block        = "10.${index(var.regions, each.value)}.0.0/16"
  enable_ipv6       = true
  enable_flow_logs  = true

  tags = {
    Environment = var.environment
    Component   = "TrustChain"
    Region      = each.value
  }
}

# EKS Cluster for each region
module "eks" {
  for_each = toset(var.regions)
  source   = "./modules/eks"

  providers = {
    aws = aws
  }

  environment            = var.environment
  region                = each.value
  vpc_id                = module.vpc[each.key].vpc_id
  subnet_ids            = module.vpc[each.key].private_subnet_ids
  cluster_name          = "trustchain-${var.environment}-${each.value}"
  kubernetes_version    = "1.28"

  node_groups = {
    ca = {
      desired_size = var.ca_replicas
      min_size     = var.ca_replicas
      max_size     = 10
      instance_types = ["c6i.2xlarge"]

      labels = {
        role = "ca"
        component = "trustchain"
      }

      taints = [{
        key    = "dedicated"
        value  = "ca"
        effect = "NoSchedule"
      }]
    }

    ct = {
      desired_size = var.ct_log_servers
      min_size     = var.ct_log_servers
      max_size     = 15
      instance_types = ["r6i.2xlarge"]

      labels = {
        role = "ct"
        component = "trustchain"
      }

      taints = [{
        key    = "dedicated"
        value  = "ct"
        effect = "NoSchedule"
      }]
    }

    dns = {
      desired_size = var.dns_servers
      min_size     = var.dns_servers
      max_size     = 10
      instance_types = ["c6in.xlarge"]

      labels = {
        role = "dns"
        component = "trustchain"
      }

      taints = [{
        key    = "dedicated"
        value  = "dns"
        effect = "NoSchedule"
      }]
    }

    monitoring = {
      desired_size = 2
      min_size     = 2
      max_size     = 5
      instance_types = ["m6i.xlarge"]

      labels = {
        role = "monitoring"
        component = "trustchain"
      }
    }
  }

  tags = {
    Environment = var.environment
    Component   = "TrustChain"
    Region      = each.value
  }
}

# S3 Buckets for CT logs and backups
resource "aws_s3_bucket" "ct_logs" {
  for_each = toset(var.regions)
  provider = aws

  bucket = "trustchain-ct-logs-${var.environment}-${each.value}"

  tags = {
    Environment = var.environment
    Component   = "TrustChain-CT"
    Region      = each.value
  }
}

resource "aws_s3_bucket_versioning" "ct_logs" {
  for_each = aws_s3_bucket.ct_logs
  provider = aws

  bucket = each.value.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_encryption" "ct_logs" {
  for_each = aws_s3_bucket.ct_logs
  provider = aws

  bucket = each.value.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm     = "aws:kms"
      kms_master_key_id = aws_kms_key.trustchain[each.key].arn
    }
  }
}

resource "aws_s3_bucket_replication_configuration" "ct_logs" {
  for_each = aws_s3_bucket.ct_logs
  provider = aws

  role   = aws_iam_role.replication[each.key].arn
  bucket = each.value.id

  rule {
    id     = "replicate-ct-logs"
    status = "Enabled"

    destination {
      bucket        = aws_s3_bucket.ct_logs_replica[each.key].arn
      storage_class = "INTELLIGENT_TIERING"

      encryption_configuration {
        replica_kms_key_id = aws_kms_key.trustchain_replica[each.key].arn
      }
    }
  }
}

# KMS Keys for encryption
resource "aws_kms_key" "trustchain" {
  for_each = toset(var.regions)
  provider = aws

  description             = "TrustChain encryption key for ${var.environment}-${each.value}"
  deletion_window_in_days = 30
  enable_key_rotation     = true

  tags = {
    Environment = var.environment
    Component   = "TrustChain"
    Region      = each.value
  }
}

resource "aws_kms_alias" "trustchain" {
  for_each = aws_kms_key.trustchain
  provider = aws

  name          = "alias/trustchain-${var.environment}-${each.key}"
  target_key_id = each.value.key_id
}

# RDS for certificate metadata (multi-region)
module "rds" {
  for_each = toset(var.regions)
  source   = "./modules/rds"

  providers = {
    aws = aws
  }

  environment               = var.environment
  region                   = each.value
  vpc_id                   = module.vpc[each.key].vpc_id
  subnet_ids               = module.vpc[each.key].database_subnet_ids
  engine                   = "aurora-postgresql"
  engine_version          = "15.4"
  instance_class          = "db.r6g.2xlarge"
  allocated_storage       = 100
  storage_encrypted       = true
  kms_key_id             = aws_kms_key.trustchain[each.key].arn
  backup_retention_period = 30
  backup_window          = "03:00-04:00"
  maintenance_window     = "sun:04:00-sun:05:00"

  enable_global_database = true
  global_cluster_identifier = "trustchain-${var.environment}"

  tags = {
    Environment = var.environment
    Component   = "TrustChain-DB"
    Region      = each.value
  }
}

# DynamoDB for distributed state
resource "aws_dynamodb_table" "trustchain_state" {
  for_each = toset(var.regions)
  provider = aws

  name           = "trustchain-state-${var.environment}"
  billing_mode   = "PAY_PER_REQUEST"
  hash_key       = "id"
  range_key      = "timestamp"

  attribute {
    name = "id"
    type = "S"
  }

  attribute {
    name = "timestamp"
    type = "N"
  }

  global_secondary_index {
    name            = "type-index"
    hash_key        = "type"
    range_key       = "timestamp"
    projection_type = "ALL"
  }

  ttl {
    attribute_name = "ttl"
    enabled        = true
  }

  stream_enabled   = true
  stream_view_type = "NEW_AND_OLD_IMAGES"

  server_side_encryption {
    enabled     = true
    kms_key_arn = aws_kms_key.trustchain[each.key].arn
  }

  replica {
    for_each = setsubtract(var.regions, [each.key])
    region_name = each.value
  }

  tags = {
    Environment = var.environment
    Component   = "TrustChain-State"
    Region      = each.value
  }
}

# Route53 for DNS
resource "aws_route53_zone" "trustchain" {
  name = var.domain

  tags = {
    Environment = var.environment
    Component   = "TrustChain-DNS"
  }
}

resource "aws_route53_health_check" "ca" {
  for_each = toset(var.regions)

  fqdn              = "ca-${each.value}.${var.domain}"
  port              = 8443
  type              = "HTTPS"
  resource_path     = "/health"
  failure_threshold = 2
  request_interval  = 30

  tags = {
    Name        = "trustchain-ca-${each.value}"
    Environment = var.environment
    Component   = "TrustChain-CA"
  }
}

resource "aws_route53_record" "ca" {
  for_each = toset(var.regions)

  zone_id = aws_route53_zone.trustchain.zone_id
  name    = "ca-${each.value}"
  type    = "A"

  alias {
    name                   = module.alb[each.key].dns_name
    zone_id                = module.alb[each.key].zone_id
    evaluate_target_health = true
  }

  set_identifier = each.value

  geolocation_routing_policy {
    continent = local.region_to_continent[each.value]
  }
}

# Application Load Balancers
module "alb" {
  for_each = toset(var.regions)
  source   = "./modules/alb"

  providers = {
    aws = aws
  }

  environment         = var.environment
  vpc_id             = module.vpc[each.key].vpc_id
  subnet_ids         = module.vpc[each.key].public_subnet_ids
  certificate_arn    = aws_acm_certificate.trustchain[each.key].arn

  target_groups = {
    ca = {
      port     = 8443
      protocol = "HTTPS"
      health_check = {
        path                = "/health"
        healthy_threshold   = 2
        unhealthy_threshold = 2
        interval            = 30
      }
    }
    ct = {
      port     = 8444
      protocol = "HTTPS"
      health_check = {
        path                = "/health"
        healthy_threshold   = 2
        unhealthy_threshold = 2
        interval            = 30
      }
    }
    api = {
      port     = 8446
      protocol = "HTTPS"
      health_check = {
        path                = "/health"
        healthy_threshold   = 2
        unhealthy_threshold = 2
        interval            = 30
      }
    }
  }

  tags = {
    Environment = var.environment
    Component   = "TrustChain-ALB"
    Region      = each.value
  }
}

# Network Load Balancers for DNS
module "nlb" {
  for_each = toset(var.regions)
  source   = "./modules/nlb"

  providers = {
    aws = aws
  }

  environment = var.environment
  vpc_id      = module.vpc[each.key].vpc_id
  subnet_ids  = module.vpc[each.key].public_subnet_ids

  listeners = {
    dns_quic = {
      port     = 853
      protocol = "UDP"
    }
  }

  tags = {
    Environment = var.environment
    Component   = "TrustChain-DNS"
    Region      = each.value
  }
}

# ACM Certificates
resource "aws_acm_certificate" "trustchain" {
  for_each = toset(var.regions)
  provider = aws

  domain_name               = var.domain
  subject_alternative_names = [
    "*.${var.domain}",
    "ca-${each.value}.${var.domain}",
    "ct-${each.value}.${var.domain}",
    "dns-${each.value}.${var.domain}"
  ]

  validation_method = "DNS"

  lifecycle {
    create_before_destroy = true
  }

  tags = {
    Environment = var.environment
    Component   = "TrustChain"
    Region      = each.value
  }
}

# WAF for API protection
module "waf" {
  for_each = toset(var.regions)
  source   = "./modules/waf"

  providers = {
    aws = aws
  }

  environment = var.environment
  alb_arn     = module.alb[each.key].arn

  rules = {
    rate_limit = {
      priority = 1
      action   = "block"
      rate_limit = 10000
    }
    geo_block = {
      priority = 2
      action   = "block"
      countries = ["CN", "RU", "KP"]
    }
    ip_reputation = {
      priority = 3
      action   = "block"
    }
  }

  tags = {
    Environment = var.environment
    Component   = "TrustChain-WAF"
    Region      = each.value
  }
}

# CloudWatch Dashboards
resource "aws_cloudwatch_dashboard" "trustchain" {
  for_each = toset(var.regions)
  provider = aws

  dashboard_name = "trustchain-${var.environment}-${each.value}"

  dashboard_body = jsonencode({
    widgets = [
      {
        type = "metric"
        properties = {
          title   = "Certificate Issuance Rate"
          region  = each.value
          metrics = [
            ["TrustChain", "CertificatesIssued", { stat = "Sum", period = 300 }],
            [".", "CertificateIssuanceTime", { stat = "Average", period = 300, yAxis = "right" }]
          ]
          period = 300
          stat   = "Average"
          region = each.value
          yAxis = {
            left = { min = 0 }
            right = { min = 0, max = 100 }
          }
        }
      },
      {
        type = "metric"
        properties = {
          title   = "CT Log Performance"
          region  = each.value
          metrics = [
            ["TrustChain", "CTLogEntries", { stat = "Sum", period = 300 }],
            [".", "CTLogAppendTime", { stat = "Average", period = 300, yAxis = "right" }]
          ]
          period = 300
          stat   = "Average"
          region = each.value
        }
      },
      {
        type = "metric"
        properties = {
          title   = "DNS Query Performance"
          region  = each.value
          metrics = [
            ["TrustChain", "DNSQueries", { stat = "Sum", period = 300 }],
            [".", "DNSResponseTime", { stat = "Average", period = 300, yAxis = "right" }]
          ]
          period = 300
          stat   = "Average"
          region = each.value
        }
      }
    ]
  })
}

# CloudWatch Alarms
module "alarms" {
  for_each = toset(var.regions)
  source   = "./modules/cloudwatch_alarms"

  providers = {
    aws = aws
  }

  environment = var.environment
  region      = each.value

  alarms = {
    high_cert_issuance_time = {
      metric_name = "CertificateIssuanceTime"
      namespace   = "TrustChain"
      statistic   = "Average"
      period      = 300
      threshold   = 100
      comparison  = "GreaterThanThreshold"
    }
    high_error_rate = {
      metric_name = "Errors"
      namespace   = "TrustChain"
      statistic   = "Sum"
      period      = 300
      threshold   = 100
      comparison  = "GreaterThanThreshold"
    }
    low_availability = {
      metric_name = "Availability"
      namespace   = "TrustChain"
      statistic   = "Average"
      period      = 300
      threshold   = 99
      comparison  = "LessThanThreshold"
    }
  }

  sns_topic_arn = aws_sns_topic.alerts[each.key].arn
}

# SNS Topics for alerts
resource "aws_sns_topic" "alerts" {
  for_each = toset(var.regions)
  provider = aws

  name = "trustchain-alerts-${var.environment}-${each.value}"

  kms_master_key_id = aws_kms_key.trustchain[each.key].id

  tags = {
    Environment = var.environment
    Component   = "TrustChain-Alerts"
    Region      = each.value
  }
}

# IAM Roles
module "iam" {
  source = "./modules/iam"

  environment = var.environment

  roles = {
    ca_role = {
      service = "ec2.amazonaws.com"
      policies = [
        "arn:aws:iam::aws:policy/AmazonS3FullAccess",
        "arn:aws:iam::aws:policy/SecretsManagerReadWrite",
        "arn:aws:iam::aws:policy/CloudWatchFullAccess"
      ]
    }
    ct_role = {
      service = "ec2.amazonaws.com"
      policies = [
        "arn:aws:iam::aws:policy/AmazonS3FullAccess",
        "arn:aws:iam::aws:policy/AmazonDynamoDBFullAccess",
        "arn:aws:iam::aws:policy/CloudWatchFullAccess"
      ]
    }
    backup_role = {
      service = "backup.amazonaws.com"
      policies = [
        "arn:aws:iam::aws:policy/service-role/AWSBackupServiceRolePolicyForBackup",
        "arn:aws:iam::aws:policy/service-role/AWSBackupServiceRolePolicyForRestores"
      ]
    }
  }
}

# AWS Backup
module "backup" {
  for_each = toset(var.regions)
  source   = "./modules/backup"

  providers = {
    aws = aws
  }

  environment = var.environment
  region      = each.value

  backup_plan_name = "trustchain-backup-${var.environment}"

  rules = {
    daily = {
      schedule          = "cron(0 5 * * ? *)"
      retention_days    = 30
      copy_to_region    = var.regions[0] != each.value ? var.regions[0] : var.regions[1]
    }
    weekly = {
      schedule          = "cron(0 6 ? * SUN *)"
      retention_days    = 90
      copy_to_region    = var.regions[0] != each.value ? var.regions[0] : var.regions[1]
    }
  }

  resources = [
    module.rds[each.key].cluster_arn,
    aws_dynamodb_table.trustchain_state[each.key].arn,
    aws_s3_bucket.ct_logs[each.key].arn
  ]

  tags = {
    Environment = var.environment
    Component   = "TrustChain-Backup"
    Region      = each.value
  }
}

# Outputs
output "ca_endpoints" {
  description = "Certificate Authority endpoints"
  value = {
    for region in var.regions : region => {
      url         = "https://ca-${region}.${var.domain}"
      health      = "https://ca-${region}.${var.domain}/health"
      api         = "https://ca-${region}.${var.domain}/api/v1"
    }
  }
}

output "ct_endpoints" {
  description = "Certificate Transparency endpoints"
  value = {
    for region in var.regions : region => {
      url         = "https://ct-${region}.${var.domain}"
      health      = "https://ct-${region}.${var.domain}/health"
      api         = "https://ct-${region}.${var.domain}/api/v1"
    }
  }
}

output "dns_endpoints" {
  description = "DNS-over-QUIC endpoints"
  value = {
    for region in var.regions : region => {
      url    = "quic://dns-${region}.${var.domain}:853"
      health = "https://dns-${region}.${var.domain}/health"
    }
  }
}

output "monitoring_dashboard" {
  description = "CloudWatch dashboard URLs"
  value = {
    for region in var.regions : region =>
      "https://console.aws.amazon.com/cloudwatch/home?region=${region}#dashboards:name=trustchain-${var.environment}-${region}"
  }
}

output "kms_keys" {
  description = "KMS key ARNs for encryption"
  value = {
    for region in var.regions : region => aws_kms_key.trustchain[region].arn
  }
}

# Local values
locals {
  region_to_continent = {
    "us-east-1"      = "NA"
    "eu-west-1"      = "EU"
    "ap-southeast-1" = "AS"
  }
}