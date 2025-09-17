# Web3 Ecosystem Development Environment Configuration
# Cost-optimized configuration for development and testing

# Environment configuration
environment = "development"
aws_region  = "us-west-2"
project_name = "hypermesh-web3"

# Domain configuration
domain_name = "dev.hypermesh.online"

# Infrastructure sizing (cost-optimized)
instance_type = "t4g.large"  # 2 vCPU, 8GB RAM
min_capacity = 1
max_capacity = 3
desired_capacity = 1

# Feature flags
enable_hsm = false                    # Use software-based keys for development
enable_monitoring = true
enable_backup = true
enable_multi_az = false              # Single AZ for cost savings
enable_cross_region_backup = false   # Development data not critical

# Security configuration (relaxed for development)
enable_waf = true
enable_guardduty = false             # Cost optimization
enable_config = false               # Cost optimization
ssl_policy = "ELBSecurityPolicy-TLS-1-2-2017-01"

# Performance tuning (relaxed targets)
catalog_target_ms = 2000
trustchain_target_ms = 5000
stoq_target_gbps = 2.95
hypermesh_target_ms = 1000

# Storage configuration
storage_type = "gp3"
storage_size = 50                    # GB
storage_iops = 3000
backup_retention_days = 7            # Short retention for development

# Cost optimization
enable_spot_instances = true         # Use spot instances for cost savings
enable_reserved_instances = false
storage_lifecycle_enabled = true
log_retention_days = 7

# Auto-scaling configuration
scale_up_threshold = 80              # CPU percentage
scale_down_threshold = 20
scale_up_adjustment = 1
scale_down_adjustment = -1

# Database configuration
db_instance_class = "db.t4g.micro"   # Minimal database for development
db_allocated_storage = 20            # GB
db_backup_retention_period = 1       # Days
db_multi_az = false

# Monitoring configuration
detailed_monitoring = false          # Basic monitoring for cost savings
enable_enhanced_monitoring = false
cloudwatch_log_retention = 7        # Days

# Network configuration
vpc_cidr = "10.0.0.0/16"
public_subnet_cidrs = ["10.0.1.0/24", "10.0.2.0/24"]
private_subnet_cidrs = ["10.0.3.0/24", "10.0.4.0/24"]

# Component-specific configuration
components = {
  trustchain = {
    enabled = true
    instance_count = 1
    ports = [8443, 6962, 8853]
    health_check_path = "/health"
  }
  
  stoq = {
    enabled = true
    instance_count = 1
    ports = [8444, 8454]
    health_check_path = "/health"
  }
  
  hypermesh = {
    enabled = true
    instance_count = 1
    ports = [8445, 8455]
    health_check_path = "/health"
  }
  
  catalog = {
    enabled = true
    instance_count = 1
    ports = [8446, 8456]
    health_check_path = "/health"
  }
  
  caesar = {
    enabled = true
    instance_count = 1
    ports = [8447, 8457]
    health_check_path = "/health"
  }
  
  ngauge = {
    enabled = false              # Application layer not ready
    instance_count = 0
    ports = [8448, 8458]
    health_check_path = "/health"
  }
}

# Load balancer configuration
load_balancer_type = "application"
enable_deletion_protection = false   # Allow deletion in development
enable_cross_zone_load_balancing = false
idle_timeout = 60

# Tags for cost allocation
additional_tags = {
  CostCenter = "Development"
  Team = "Web3-Core"
  AutoShutdown = "enabled"           # For cost management
  Environment = "development"
}