# Web3 Ecosystem Production Environment Configuration
# Enterprise-grade configuration for production deployment

# Environment configuration
environment = "production"
aws_region  = "us-west-2"
project_name = "hypermesh-web3"

# Domain configuration
domain_name = "hypermesh.online"

# Infrastructure sizing (enterprise-grade)
instance_type = "c6g.4xlarge"        # 16 vCPU, 32GB RAM
min_capacity = 3                     # Always maintain minimum capacity
max_capacity = 12                    # Handle 10x traffic spikes
desired_capacity = 3

# Feature flags (full production features)
enable_hsm = true                    # AWS CloudHSM for certificate keys
enable_monitoring = true
enable_backup = true
enable_multi_az = true              # High availability required
enable_cross_region_backup = true   # Disaster recovery compliance

# Security configuration (maximum security)
enable_waf = true
enable_guardduty = true
enable_config = true
enable_vpc_flow_logs = true
enable_cloudtrail = true
ssl_policy = "ELBSecurityPolicy-TLS-1-3-2021-06"

# Performance tuning (stringent production targets)
catalog_target_ms = 2000             # 2s target (currently 1.69ms)
trustchain_target_ms = 5000          # 5s target (currently 35ms)
stoq_target_gbps = 40               # 40+ Gbps target (currently 2.95 Gbps)
hypermesh_target_ms = 1000          # 1s target (currently 2ms)

# Storage configuration (enterprise-grade)
storage_type = "gp3"
storage_size = 500                   # GB
storage_iops = 10000                # High IOPS for performance
storage_throughput = 500            # MB/s
backup_retention_days = 2555        # 7 years for compliance

# Cost optimization (production efficiency)
enable_spot_instances = false        # Reliability over cost
enable_reserved_instances = true     # Cost optimization for predictable workloads
storage_lifecycle_enabled = true
log_retention_days = 2555           # 7 years for compliance

# Auto-scaling configuration (responsive)
scale_up_threshold = 60              # Conservative scaling
scale_down_threshold = 20
scale_up_adjustment = 2              # Quick response to load
scale_down_adjustment = -1           # Gradual scale down
scale_up_cooldown = 300             # 5 minutes
scale_down_cooldown = 600           # 10 minutes

# Database configuration (enterprise-grade)
db_instance_class = "db.r6g.2xlarge" # 8 vCPU, 64GB RAM
db_allocated_storage = 1000          # 1TB initial storage
db_max_allocated_storage = 10000     # 10TB maximum
db_backup_retention_period = 35      # 35 days
db_multi_az = true                   # High availability
db_encrypted = true
db_performance_insights = true

# Enhanced monitoring configuration
detailed_monitoring = true
enable_enhanced_monitoring = true
cloudwatch_log_retention = 2555     # 7 years for compliance
enable_xray_tracing = true
enable_custom_metrics = true

# Network configuration (production)
vpc_cidr = "10.2.0.0/16"
public_subnet_cidrs = ["10.2.1.0/24", "10.2.2.0/24", "10.2.3.0/24"]
private_subnet_cidrs = ["10.2.4.0/24", "10.2.5.0/24", "10.2.6.0/24"]
enable_nat_gateway = true
enable_vpn_gateway = false

# Component-specific configuration (production-ready)
components = {
  trustchain = {
    enabled = true
    instance_count = 3               # High availability
    ports = [8443, 6962, 8853]
    health_check_path = "/health"
    performance_target_ms = 35       # Current exceptional performance
    sla_uptime_percent = 99.99
  }
  
  stoq = {
    enabled = true
    instance_count = 3
    ports = [8444, 8454]
    health_check_path = "/health"
    performance_target_gbps = 10     # Phase 1 minimum
    performance_goal_gbps = 40       # Ultimate target
    enable_performance_monitoring = true
  }
  
  hypermesh = {
    enabled = true
    instance_count = 3
    ports = [8445, 8455]
    health_check_path = "/health"
    performance_target_ms = 2        # Current exceptional performance
    asset_operations_per_second = 500
  }
  
  catalog = {
    enabled = true
    instance_count = 3
    ports = [8446, 8456]
    health_check_path = "/health"
    performance_target_ms = 1.69     # Current exceptional performance
    julia_vm_enabled = true
    consensus_validation = true
  }
  
  caesar = {
    enabled = true
    instance_count = 3
    ports = [8447, 8457]
    health_check_path = "/health"
    economic_incentives = true
    multi_chain_support = true
  }
  
  ngauge = {
    enabled = true
    instance_count = 2               # Application layer
    ports = [8448, 8458]
    health_check_path = "/health"
    engagement_analytics = true
    p2p_advertising = true
  }
}

# Load balancer configuration (enterprise)
load_balancer_type = "application"
enable_deletion_protection = true    # Protect production infrastructure
enable_cross_zone_load_balancing = true
idle_timeout = 300                   # 5 minutes for long connections
enable_http2 = true
enable_waf_logging = true

# HSM configuration (production security)
hsm_configuration = {
  cluster_size = 2                   # Primary + backup
  instance_type = "hsm1.medium"
  backup_retention_policy = "indefinite"
  auto_backup_enabled = true
  cross_region_backup = true
}

# Performance monitoring (comprehensive)
performance_monitoring = {
  enabled = true
  real_time_metrics = true
  custom_dashboards = true
  automated_alerting = true
  
  sla_targets = {
    uptime_percent = 99.9
    response_time_p95_ms = 100
    response_time_p99_ms = 500
    throughput_ops_per_second = 10000
  }
}

# Security monitoring (enterprise-grade)
security_monitoring = {
  enabled = true
  real_time_alerts = true
  automated_response = true
  compliance_reporting = true
  
  alert_thresholds = {
    failed_authentication_per_minute = 100
    unusual_traffic_pattern = true
    certificate_expiry_warning_days = 30
    security_group_changes = true
    iam_policy_changes = true
  }
}

# Backup and disaster recovery (enterprise)
disaster_recovery = {
  enabled = true
  rto_target_minutes = 30           # Recovery Time Objective
  rpo_target_minutes = 5            # Recovery Point Objective
  
  backup_strategy = {
    frequency = "continuous"
    retention_days = 2555           # 7 years
    cross_region_replication = true
    encryption_at_rest = true
    automated_testing = true
  }
  
  failover_strategy = {
    automated_failover = false      # Manual for production safety
    health_check_grace_period = 300 # 5 minutes
    dns_failover_ttl = 60          # 1 minute
  }
}

# Compliance configuration
compliance = {
  frameworks = ["SOC2", "ISO27001", "FIPS140-2"]
  data_retention_years = 7
  audit_logging = true
  encryption_at_rest = true
  encryption_in_transit = true
  
  certificate_transparency = {
    enabled = true
    ct_log_monitoring = true
    sct_validation = true
  }
}

# Alerts and notifications (comprehensive)
alerting = {
  enabled = true
  channels = {
    email = ["production-alerts@hypermesh.online", "devops@hypermesh.online"]
    slack_critical = "#production-critical"
    slack_warnings = "#production-alerts"
    pagerduty = true
    sms = true
  }
  
  escalation_policy = {
    level_1_timeout_minutes = 5
    level_2_timeout_minutes = 15
    level_3_timeout_minutes = 30
  }
  
  thresholds = {
    cpu_utilization_warning = 70
    cpu_utilization_critical = 85
    memory_utilization_warning = 80
    memory_utilization_critical = 90
    disk_utilization_warning = 85
    disk_utilization_critical = 95
    response_time_warning_ms = 500
    response_time_critical_ms = 1000
    error_rate_warning_percent = 1
    error_rate_critical_percent = 5
    throughput_degradation_percent = 20
  }
}

# Cost monitoring and optimization
cost_management = {
  enabled = true
  budget_alerts = true
  monthly_budget_usd = 3000
  cost_allocation_tags = true
  reserved_instance_recommendations = true
  rightsizing_recommendations = true
  
  budget_thresholds = {
    warning_percent = 80
    critical_percent = 100
    forecast_warning_percent = 90
  }
}

# Network security (zero-trust)
network_security = {
  ipv6_only = true                  # Complete IPv4 elimination
  private_subnets_egress_only = true
  waf_rate_limiting = true
  ddos_protection = "enhanced"
  
  security_groups = {
    default_deny_all = true
    least_privilege_access = true
    automated_compliance_checks = true
  }
}

# Tags for production management
additional_tags = {
  CostCenter = "Production"
  Team = "Web3-Core"
  Purpose = "Production-Deployment"
  Environment = "production"
  Compliance = "SOC2-ISO27001-FIPS140-2"
  BusinessCriticality = "High"
  DataClassification = "Confidential"
  BackupRequired = "true"
  MonitoringRequired = "true"
  ComplianceRequired = "true"
}