# Web3 Ecosystem Production Infrastructure - Main Configuration
# Complete multi-service IPv6-only AWS infrastructure for HyperMesh ecosystem

terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    cloudinit = {
      source  = "hashicorp/cloudinit"
      version = "~> 2.3"
    }
  }
}

# Configure AWS Provider
provider "aws" {
  region = var.aws_region
  
  default_tags {
    tags = {
      Project     = "Web3-Ecosystem"
      Environment = var.environment
      Component   = "Multi-Service"
      ManagedBy   = "Terraform"
      Owner       = "HyperMesh"
      Repository  = "hypermesh-online"
    }
  }
}

# Data sources
data "aws_availability_zones" "available" {
  state = "available"
}

data "aws_ami" "ubuntu_arm64" {
  most_recent = true
  owners      = ["099720109477"] # Canonical

  filter {
    name   = "name"
    values = ["ubuntu/images/hvm-ssd/ubuntu-22.04-arm64-server-*"]
  }

  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }
}

# Local values
locals {
  availability_zones = slice(data.aws_availability_zones.available.names, 0, 2)
  
  # IPv6 CIDR blocks for subnets
  ipv6_subnets = {
    public = [
      cidrsubnet(aws_vpc.trustchain.ipv6_cidr_block, 8, 1),
      cidrsubnet(aws_vpc.trustchain.ipv6_cidr_block, 8, 2)
    ]
    private = [
      cidrsubnet(aws_vpc.trustchain.ipv6_cidr_block, 8, 3),
      cidrsubnet(aws_vpc.trustchain.ipv6_cidr_block, 8, 4)
    ]
  }

  # Web3 Ecosystem Service endpoints
  services = {
    # TrustChain services
    trustchain_ca = {
      port        = 8443
      protocol    = "HTTPS"
      health_path = "/health"
      component   = "trustchain"
    }
    trustchain_ct = {
      port        = 6962
      protocol    = "HTTPS"
      health_path = "/ct/v1/get-sth"
      component   = "trustchain"
    }
    trustchain_dns = {
      port     = 8853
      protocol = "UDP"
      component = "trustchain"
    }
    
    # STOQ Protocol services
    stoq_transport = {
      port        = 8444
      protocol    = "UDP"
      description = "STOQ Transport Protocol"
      component   = "stoq"
    }
    stoq_api = {
      port        = 8454
      protocol    = "HTTPS"
      health_path = "/health"
      component   = "stoq"
    }
    
    # HyperMesh services
    hypermesh_api = {
      port        = 8445
      protocol    = "HTTPS"
      health_path = "/health"
      component   = "hypermesh"
    }
    hypermesh_assets = {
      port        = 8455
      protocol    = "HTTPS"
      health_path = "/assets/health"
      component   = "hypermesh"
    }
    
    # Catalog services
    catalog_api = {
      port        = 8446
      protocol    = "HTTPS"
      health_path = "/health"
      component   = "catalog"
    }
    catalog_vm = {
      port        = 8456
      protocol    = "HTTPS"
      health_path = "/vm/health"
      component   = "catalog"
    }
    
    # Caesar services
    caesar_wallet = {
      port        = 8447
      protocol    = "HTTPS"
      health_path = "/health"
      component   = "caesar"
    }
    caesar_exchange = {
      port        = 8457
      protocol    = "HTTPS"
      health_path = "/exchange/health"
      component   = "caesar"
    }
    
    # NGauge services
    ngauge_engagement = {
      port        = 8448
      protocol    = "HTTPS"
      health_path = "/health"
      component   = "ngauge"
    }
    ngauge_analytics = {
      port        = 8458
      protocol    = "HTTPS"
      health_path = "/analytics/health"
      component   = "ngauge"
    }
  }
}

# Call modules
module "networking" {
  source = "./modules/networking"
  
  environment        = var.environment
  availability_zones = local.availability_zones
  ipv6_subnets      = local.ipv6_subnets
}

module "security" {
  source = "./modules/security"
  
  environment = var.environment
  vpc_id      = module.networking.vpc_id
  services    = local.services
}

module "compute" {
  source = "./modules/compute"
  
  environment        = var.environment
  vpc_id            = module.networking.vpc_id
  subnet_ids        = module.networking.public_subnet_ids
  security_group_id = module.security.trustchain_security_group_id
  key_pair_name     = var.key_pair_name
  instance_type     = var.instance_type
  ami_id           = data.aws_ami.ubuntu_arm64.id
  
  depends_on = [module.storage, module.hsm]
}

module "load_balancer" {
  source = "./modules/load_balancer"
  
  environment               = var.environment
  vpc_id                   = module.networking.vpc_id
  subnet_ids               = module.networking.public_subnet_ids
  security_group_id        = module.security.alb_security_group_id
  autoscaling_group_name   = module.compute.autoscaling_group_name
  ssl_certificate_arn      = module.certificates.ssl_certificate_arn
  waf_web_acl_arn         = module.security.waf_web_acl_arn
  services                 = local.services
}

module "storage" {
  source = "./modules/storage"
  
  environment = var.environment
}

module "hsm" {
  source = "./modules/hsm"
  
  environment = var.environment
  vpc_id      = module.networking.vpc_id
  subnet_ids  = module.networking.private_subnet_ids
}

module "monitoring" {
  source = "./modules/monitoring"
  
  environment             = var.environment
  load_balancer_arn       = module.load_balancer.alb_arn
  target_group_arns       = module.load_balancer.target_group_arns
  instance_ids            = module.compute.instance_ids
  autoscaling_group_name  = module.compute.autoscaling_group_name
  ct_logs_bucket_name     = module.storage.bucket_names.ct_logs
  sns_topic_arn           = var.alert_sns_topic_arn
}

module "certificates" {
  source = "./modules/certificates"
  
  environment = var.environment
  domain_name = var.domain_name
}

module "dns" {
  source = "./modules/dns"
  
  environment            = var.environment
  domain_name           = var.domain_name
  load_balancer_dns_name = module.load_balancer.alb_dns_name
  load_balancer_zone_id  = module.load_balancer.alb_zone_id
  sns_topic_arn         = var.alert_sns_topic_arn
}

module "backup" {
  source = "./modules/backup"
  
  environment     = var.environment
  s3_bucket_arns  = [for arn in module.storage.bucket_arns : arn if arn != null]
  instance_ids    = module.compute.instance_ids
  sns_topic_arn   = var.alert_sns_topic_arn
}