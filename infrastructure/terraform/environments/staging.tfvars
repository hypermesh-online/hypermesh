# Web3 Ecosystem Staging Environment Configuration
# Production-like configuration for staging validation

# Environment configuration
environment = "staging"
aws_region  = "us-west-2"
project_name = "hypermesh-web3"

# Domain configuration
domain_name = "staging.hypermesh.online"

# Infrastructure sizing (production-like)
instance_type = "c6g.xlarge"         # 4 vCPU, 8GB RAM
min_capacity = 2
max_capacity = 6
desired_capacity = 2

# Feature flags
enable_hsm = false                   # Software-based keys for staging
enable_monitoring = true
enable_backup = true
enable_multi_az = true              # Production-like redundancy
enable_cross_region_backup = true   # Test disaster recovery

# Security configuration (production-like)
enable_waf = true
enable_guardduty = true
enable_config = true
ssl_policy = "ELBSecurityPolicy-TLS-1-3-2021-06"

# Performance tuning (production targets)
catalog_target_ms = 2000
trustchain_target_ms = 5000
stoq_target_gbps = 10               # Phase 1 target for staging validation
hypermesh_target_ms = 1000

# Storage configuration
storage_type = "gp3"
storage_size = 100                   # GB
storage_iops = 3000
backup_retention_days = 30           # Extended retention for validation

# Cost optimization (balanced)
enable_spot_instances = false        # Reliability over cost for staging
enable_reserved_instances = false
storage_lifecycle_enabled = true
log_retention_days = 30

# Auto-scaling configuration
scale_up_threshold = 70              # More aggressive scaling
scale_down_threshold = 30
scale_up_adjustment = 2
scale_down_adjustment = -1

# Database configuration
db_instance_class = "db.t4g.large"   # Production-like database
db_allocated_storage = 100           # GB
db_backup_retention_period = 7       # Days
db_multi_az = true                   # High availability

# Monitoring configuration
detailed_monitoring = true           # Full monitoring for validation
enable_enhanced_monitoring = true
cloudwatch_log_retention = 30       # Days

# Network configuration
vpc_cidr = "10.1.0.0/16"
public_subnet_cidrs = ["10.1.1.0/24", "10.1.2.0/24"]
private_subnet_cidrs = ["10.1.3.0/24", "10.1.4.0/24"]

# Component-specific configuration
components = {
  trustchain = {
    enabled = true
    instance_count = 2
    ports = [8443, 6962, 8853]
    health_check_path = "/health"
    performance_target_ms = 35       # Current performance level
  }
  
  stoq = {
    enabled = true
    instance_count = 2
    ports = [8444, 8454]
    health_check_path = "/health"
    performance_target_gbps = 2.95   # Current performance level
  }
  
  hypermesh = {
    enabled = true
    instance_count = 2
    ports = [8445, 8455]
    health_check_path = "/health"
    performance_target_ms = 2        # Current performance level
  }
  
  catalog = {
    enabled = true
    instance_count = 2
    ports = [8446, 8456]
    health_check_path = "/health"
    performance_target_ms = 1.69     # Current performance level
  }
  
  caesar = {
    enabled = true
    instance_count = 2
    ports = [8447, 8457]
    health_check_path = "/health"
  }
  
  ngauge = {
    enabled = true                   # Enable for staging validation
    instance_count = 1
    ports = [8448, 8458]
    health_check_path = "/health"
  }
}

# Load balancer configuration
load_balancer_type = "application"
enable_deletion_protection = false   # Allow deletion for staging environment
enable_cross_zone_load_balancing = true
idle_timeout = 60

# Performance testing configuration
load_testing = {
  enabled = true
  max_concurrent_connections = 1000
  test_duration_minutes = 30
  ramp_up_time_minutes = 5
}

# Security testing configuration
security_testing = {
  enabled = true
  vulnerability_scanning = true
  penetration_testing = false       # Manual execution
  compliance_checking = true
}

# Backup and disaster recovery testing
disaster_recovery = {
  enabled = true
  backup_testing_schedule = "weekly"
  failover_testing_schedule = "monthly"
  rto_target_minutes = 30
  rpo_target_minutes = 5
}

# Alerts and notifications
alerting = {
  enabled = true
  email_recipients = ["staging-alerts@hypermesh.online"]
  slack_webhook_enabled = true
  pagerduty_enabled = false
  
  thresholds = {
    cpu_utilization = 80
    memory_utilization = 85
    disk_utilization = 90
    response_time_ms = 1000
    error_rate_percent = 5
  }
}

# Tags for resource management
additional_tags = {
  CostCenter = "Staging"
  Team = "Web3-Core"
  Purpose = "Production-Validation"
  Environment = "staging"
  Compliance = "SOC2-ISO27001"
}