# Compute Module - EC2 Instances for TrustChain

# Placement Group for Enhanced Networking
resource "aws_placement_group" "trustchain" {
  name     = "trustchain-cluster-${var.environment}"
  strategy = "cluster"

  tags = {
    Name        = "trustchain-placement-group-${var.environment}"
    Environment = var.environment
    Purpose     = "Enhanced Networking Performance"
  }
}

# Launch Template for TrustChain Instances
resource "aws_launch_template" "trustchain" {
  name_prefix   = "trustchain-${var.environment}-"
  image_id      = var.ami_id
  instance_type = var.instance_type
  key_name      = var.key_pair_name

  vpc_security_group_ids = [var.security_group_id]

  # IPv6-only configuration
  metadata_options {
    http_endpoint               = "enabled"
    http_tokens                 = "required"
    http_put_response_hop_limit = 2
    instance_metadata_tags      = "enabled"
  }

  monitoring {
    enabled = true
  }

  # Enhanced networking
  ebs_optimized = true
  
  network_interfaces {
    associate_ipv6_address_count = 1
    delete_on_termination       = true
    device_index               = 0
    ipv6_address_count         = 1
    security_groups            = [var.security_group_id]
  }

  # EBS configuration
  block_device_mappings {
    device_name = "/dev/sda1"
    ebs {
      volume_size           = var.ebs_volume_size_gb
      volume_type           = var.ebs_volume_type
      iops                  = var.ebs_iops
      throughput           = var.ebs_throughput_mbps
      encrypted            = true
      delete_on_termination = true
    }
  }

  # Additional EBS volume for TrustChain data
  block_device_mappings {
    device_name = "/dev/sdf"
    ebs {
      volume_size           = 50
      volume_type           = "gp3"
      iops                  = 3000
      throughput           = 125
      encrypted            = true
      delete_on_termination = false
    }
  }

  # User data for instance initialization
  user_data = base64encode(templatefile("${path.module}/user_data.sh", {
    environment = var.environment
    hostname_prefix = "trustchain"
  }))

  tag_specifications {
    resource_type = "instance"
    tags = {
      Name        = "trustchain-${var.environment}"
      Environment = var.environment
      Purpose     = "TrustChain Certificate Authority"
      Component   = "CA-CT-DNS"
    }
  }

  tag_specifications {
    resource_type = "volume"
    tags = {
      Name        = "trustchain-volume-${var.environment}"
      Environment = var.environment
      Purpose     = "TrustChain Storage"
    }
  }

  tags = {
    Name        = "trustchain-launch-template-${var.environment}"
    Environment = var.environment
  }
}

# Auto Scaling Group for High Availability
resource "aws_autoscaling_group" "trustchain" {
  name                = "trustchain-asg-${var.environment}"
  vpc_zone_identifier = var.subnet_ids
  min_size            = var.min_instances
  max_size            = var.max_instances
  desired_capacity    = var.desired_instances
  health_check_type   = "ELB"
  health_check_grace_period = 300
  placement_group     = aws_placement_group.trustchain.id

  launch_template {
    id      = aws_launch_template.trustchain.id
    version = "$Latest"
  }

  # Instance refresh configuration
  instance_refresh {
    strategy = "Rolling"
    preferences {
      min_healthy_percentage = 50
      instance_warmup       = 300
    }
  }

  tag {
    key                 = "Name"
    value               = "trustchain-asg-${var.environment}"
    propagate_at_launch = false
  }

  tag {
    key                 = "Environment"
    value               = var.environment
    propagate_at_launch = true
  }

  tag {
    key                 = "Purpose"
    value               = "TrustChain Certificate Authority"
    propagate_at_launch = true
  }

  tag {
    key                 = "Component"
    value               = "CA-CT-DNS"
    propagate_at_launch = true
  }
}

# CloudWatch Auto Scaling Policy - Scale Up
resource "aws_autoscaling_policy" "scale_up" {
  name                   = "trustchain-scale-up-${var.environment}"
  scaling_adjustment     = 1
  adjustment_type        = "ChangeInCapacity"
  cooldown              = 300
  autoscaling_group_name = aws_autoscaling_group.trustchain.name
}

# CloudWatch Auto Scaling Policy - Scale Down
resource "aws_autoscaling_policy" "scale_down" {
  name                   = "trustchain-scale-down-${var.environment}"
  scaling_adjustment     = -1
  adjustment_type        = "ChangeInCapacity"
  cooldown              = 300
  autoscaling_group_name = aws_autoscaling_group.trustchain.name
}

# CloudWatch Metric Alarm - High CPU
resource "aws_cloudwatch_metric_alarm" "high_cpu" {
  alarm_name          = "trustchain-high-cpu-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "CPUUtilization"
  namespace          = "AWS/EC2"
  period             = "120"
  statistic          = "Average"
  threshold          = "80"
  alarm_description  = "This metric monitors TrustChain instance CPU utilization"

  dimensions = {
    AutoScalingGroupName = aws_autoscaling_group.trustchain.name
  }

  alarm_actions = [aws_autoscaling_policy.scale_up.arn]

  tags = {
    Name        = "trustchain-high-cpu-alarm-${var.environment}"
    Environment = var.environment
  }
}

# CloudWatch Metric Alarm - Low CPU
resource "aws_cloudwatch_metric_alarm" "low_cpu" {
  alarm_name          = "trustchain-low-cpu-${var.environment}"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "CPUUtilization"
  namespace          = "AWS/EC2"
  period             = "120"
  statistic          = "Average"
  threshold          = "10"
  alarm_description  = "This metric monitors TrustChain instance CPU utilization"

  dimensions = {
    AutoScalingGroupName = aws_autoscaling_group.trustchain.name
  }

  alarm_actions = [aws_autoscaling_policy.scale_down.arn]

  tags = {
    Name        = "trustchain-low-cpu-alarm-${var.environment}"
    Environment = var.environment
  }
}

# CloudWatch Custom Metric Alarm - Certificate Operations
resource "aws_cloudwatch_metric_alarm" "high_certificate_operations" {
  alarm_name          = "trustchain-high-cert-ops-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "CertificateOperations"
  namespace          = "TrustChain/Performance"
  period             = "60"
  statistic          = "Sum"
  threshold          = var.max_certificate_operations_per_minute
  alarm_description  = "High certificate operation load"

  alarm_actions = [aws_autoscaling_policy.scale_up.arn]

  tags = {
    Name        = "trustchain-high-cert-ops-alarm-${var.environment}"
    Environment = var.environment
  }
}

# IAM Role for EC2 Instances
resource "aws_iam_role" "trustchain_instance" {
  name = "trustchain-instance-role-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name        = "trustchain-instance-role-${var.environment}"
    Environment = var.environment
  }
}

# IAM Instance Profile
resource "aws_iam_instance_profile" "trustchain" {
  name = "trustchain-instance-profile-${var.environment}"
  role = aws_iam_role.trustchain_instance.name

  tags = {
    Name        = "trustchain-instance-profile-${var.environment}"
    Environment = var.environment
  }
}

# IAM Policy for TrustChain Operations
resource "aws_iam_role_policy" "trustchain_operations" {
  name = "trustchain-operations-${var.environment}"
  role = aws_iam_role.trustchain_instance.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:DeleteObject",
          "s3:ListBucket"
        ]
        Resource = [
          "arn:aws:s3:::trustchain-*",
          "arn:aws:s3:::trustchain-*/*"
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt",
          "kms:DescribeKey",
          "kms:Sign",
          "kms:GetPublicKey"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "cloudhsm:DescribeClusters",
          "cloudhsm:DescribeBackups"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "cloudwatch:PutMetricData",
          "cloudwatch:GetMetricStatistics",
          "cloudwatch:ListMetrics"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents",
          "logs:DescribeLogStreams"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "ssm:GetParameter",
          "ssm:GetParameters",
          "ssm:GetParametersByPath"
        ]
        Resource = "arn:aws:ssm:*:*:parameter/trustchain/*"
      }
    ]
  })
}

# CloudWatch Agent Policy
resource "aws_iam_role_policy_attachment" "cloudwatch_agent" {
  role       = aws_iam_role.trustchain_instance.name
  policy_arn = "arn:aws:iam::aws:policy/CloudWatchAgentServerPolicy"
}

# SSM Managed Instance Core Policy
resource "aws_iam_role_policy_attachment" "ssm_managed_instance" {
  role       = aws_iam_role.trustchain_instance.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonSSMManagedInstanceCore"
}

# Systems Manager Parameters for Configuration
resource "aws_ssm_parameter" "trustchain_config" {
  for_each = {
    "domain_name"           = var.domain_name
    "environment"          = var.environment
    "consensus_proofs"     = jsonencode(var.consensus_proof_types)
    "certificate_rotation" = tostring(var.certificate_rotation_hours)
    "performance_target"   = tostring(var.target_response_time_ms)
  }

  name  = "/trustchain/${var.environment}/${each.key}"
  type  = "String"
  value = each.value

  tags = {
    Name        = "trustchain-config-${each.key}-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain Configuration"
  }
}

# Data source to get current instances (for outputs)
data "aws_instances" "trustchain" {
  instance_tags = {
    "aws:autoscaling:groupName" = aws_autoscaling_group.trustchain.name
  }

  depends_on = [aws_autoscaling_group.trustchain]
}