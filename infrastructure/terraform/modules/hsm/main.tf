# HSM Module - AWS CloudHSM for TrustChain Certificate Keys

# CloudHSM Cluster
resource "aws_cloudhsm_v2_cluster" "trustchain" {
  count = var.enable_hsm ? 1 : 0

  hsm_type   = "hsm1.medium"
  subnet_ids = var.subnet_ids

  tags = {
    Name        = "trustchain-hsm-cluster-${var.environment}"
    Environment = var.environment
    Purpose     = "Certificate Authority Key Security"
    Compliance  = "FIPS-140-2-Level-3"
  }
}

# CloudHSM Instance (Primary)
resource "aws_cloudhsm_v2_hsm" "trustchain_primary" {
  count = var.enable_hsm ? 1 : 0

  cluster_id        = aws_cloudhsm_v2_cluster.trustchain[0].cluster_id
  subnet_id         = var.subnet_ids[0]
  availability_zone = data.aws_subnet.hsm_subnets[0].availability_zone

  tags = {
    Name        = "trustchain-hsm-primary-${var.environment}"
    Environment = var.environment
    Role        = "Primary"
  }
}

# CloudHSM Instance (Secondary for HA)
resource "aws_cloudhsm_v2_hsm" "trustchain_secondary" {
  count = var.enable_hsm && var.enable_ha ? 1 : 0

  cluster_id        = aws_cloudhsm_v2_cluster.trustchain[0].cluster_id
  subnet_id         = var.subnet_ids[1]
  availability_zone = data.aws_subnet.hsm_subnets[1].availability_zone

  tags = {
    Name        = "trustchain-hsm-secondary-${var.environment}"
    Environment = var.environment
    Role        = "Secondary"
  }
}

# Security Group for CloudHSM
resource "aws_security_group" "cloudhsm" {
  count = var.enable_hsm ? 1 : 0

  name        = "trustchain-cloudhsm-${var.environment}"
  description = "Security group for TrustChain CloudHSM cluster"
  vpc_id      = var.vpc_id

  # HSM client communication
  ingress {
    from_port        = 2223
    to_port          = 2225
    protocol         = "tcp"
    cidr_blocks      = [data.aws_vpc.hsm_vpc.cidr_block]
    ipv6_cidr_blocks = [data.aws_vpc.hsm_vpc.ipv6_cidr_block]
    description      = "HSM client communication"
  }

  # NTLS (Network TLS)
  ingress {
    from_port        = 1792
    to_port          = 1792
    protocol         = "tcp"
    cidr_blocks      = [data.aws_vpc.hsm_vpc.cidr_block]
    ipv6_cidr_blocks = [data.aws_vpc.hsm_vpc.ipv6_cidr_block]
    description      = "NTLS communication"
  }

  # HSM management
  ingress {
    from_port        = 22
    to_port          = 22
    protocol         = "tcp"
    cidr_blocks      = ["10.0.0.0/8"] # Restrict to private networks only
    description      = "SSH management (private only)"
  }

  egress {
    from_port        = 0
    to_port          = 0
    protocol         = "-1"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
    description      = "All outbound traffic"
  }

  tags = {
    Name        = "trustchain-cloudhsm-sg-${var.environment}"
    Environment = var.environment
    Purpose     = "CloudHSM Security"
  }
}

# CloudWatch Log Group for HSM Audit Logs
resource "aws_cloudwatch_log_group" "hsm_audit" {
  count = var.enable_hsm ? 1 : 0

  name              = "/aws/cloudhsm/trustchain-${var.environment}"
  retention_in_days = 365 # Keep HSM audit logs for 1 year minimum

  tags = {
    Name        = "trustchain-hsm-audit-logs-${var.environment}"
    Environment = var.environment
    Purpose     = "HSM Audit Logging"
    Compliance  = "Required"
  }
}

# IAM Role for HSM Client Access
resource "aws_iam_role" "hsm_client" {
  count = var.enable_hsm ? 1 : 0
  name  = "trustchain-hsm-client-${var.environment}"

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
    Name        = "trustchain-hsm-client-role-${var.environment}"
    Environment = var.environment
  }
}

# IAM Policy for HSM Client Operations
resource "aws_iam_role_policy" "hsm_client" {
  count = var.enable_hsm ? 1 : 0
  name  = "trustchain-hsm-client-policy-${var.environment}"
  role  = aws_iam_role.hsm_client[0].id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "cloudhsm:DescribeClusters",
          "cloudhsm:DescribeBackups",
          "cloudhsm:ListTags"
        ]
        Resource = var.enable_hsm ? aws_cloudhsm_v2_cluster.trustchain[0].cluster_id : "*"
      },
      {
        Effect = "Allow"
        Action = [
          "ec2:DescribeNetworkInterfaces",
          "ec2:CreateNetworkInterface",
          "ec2:DeleteNetworkInterface",
          "ec2:AttachNetworkInterface",
          "ec2:DetachNetworkInterface"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents",
          "logs:DescribeLogGroups",
          "logs:DescribeLogStreams"
        ]
        Resource = var.enable_hsm ? aws_cloudwatch_log_group.hsm_audit[0].arn : "*"
      }
    ]
  })
}

# Systems Manager Parameter for HSM Cluster ID
resource "aws_ssm_parameter" "hsm_cluster_id" {
  count = var.enable_hsm ? 1 : 0

  name  = "/trustchain/${var.environment}/hsm/cluster_id"
  type  = "SecureString"
  value = aws_cloudhsm_v2_cluster.trustchain[0].cluster_id

  tags = {
    Name        = "trustchain-hsm-cluster-id-${var.environment}"
    Environment = var.environment
    Purpose     = "HSM Configuration"
  }
}

# Systems Manager Parameter for HSM Cluster State
resource "aws_ssm_parameter" "hsm_cluster_state" {
  count = var.enable_hsm ? 1 : 0

  name  = "/trustchain/${var.environment}/hsm/cluster_state"
  type  = "String"
  value = aws_cloudhsm_v2_cluster.trustchain[0].cluster_state

  tags = {
    Name        = "trustchain-hsm-cluster-state-${var.environment}"
    Environment = var.environment
    Purpose     = "HSM Status"
  }
}

# CloudWatch Metric Alarm for HSM Cluster State
resource "aws_cloudwatch_metric_alarm" "hsm_cluster_state" {
  count = var.enable_hsm ? 1 : 0

  alarm_name          = "trustchain-hsm-cluster-state-${var.environment}"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "ClusterState"
  namespace          = "AWS/CloudHSM"
  period             = "300"
  statistic          = "Maximum"
  threshold          = "3" # ACTIVE state value
  alarm_description  = "HSM cluster not in ACTIVE state"
  treat_missing_data = "breaching"

  dimensions = {
    ClusterId = var.enable_hsm ? aws_cloudhsm_v2_cluster.trustchain[0].cluster_id : ""
  }

  tags = {
    Name        = "trustchain-hsm-cluster-state-${var.environment}"
    Environment = var.environment
  }
}

# CloudWatch Metric Alarm for HSM Instance Health
resource "aws_cloudwatch_metric_alarm" "hsm_instance_health" {
  count = var.enable_hsm ? 1 : 0

  alarm_name          = "trustchain-hsm-instance-health-${var.environment}"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "HsmInstanceHealth"
  namespace          = "AWS/CloudHSM"
  period             = "300"
  statistic          = "Average"
  threshold          = "1"
  alarm_description  = "HSM instance health check failed"

  dimensions = {
    ClusterId = var.enable_hsm ? aws_cloudhsm_v2_cluster.trustchain[0].cluster_id : ""
  }

  tags = {
    Name        = "trustchain-hsm-instance-health-${var.environment}"
    Environment = var.environment
  }
}

# Data sources
data "aws_vpc" "hsm_vpc" {
  id = var.vpc_id
}

data "aws_subnet" "hsm_subnets" {
  count = length(var.subnet_ids)
  id    = var.subnet_ids[count.index]
}

# HSM Backup (automated)
resource "aws_cloudhsm_v2_cluster" "backup_schedule" {
  count = var.enable_hsm && var.enable_backup ? 1 : 0

  # CloudHSM automatically creates daily backups, but we can force manual backups
  # Manual backup creation would be handled via AWS CLI or SDK in application code
}

# KMS Key for HSM Backup Encryption (if using cross-region backup)
resource "aws_kms_key" "hsm_backup" {
  count = var.enable_hsm && var.enable_backup ? 1 : 0

  description             = "TrustChain HSM backup encryption key"
  deletion_window_in_days = var.environment == "prod" ? 30 : 7
  enable_key_rotation     = true

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "Enable IAM User Permissions"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action   = "kms:*"
        Resource = "*"
      },
      {
        Sid    = "Allow CloudHSM Service"
        Effect = "Allow"
        Principal = {
          Service = "cloudhsm.amazonaws.com"
        }
        Action = [
          "kms:Encrypt",
          "kms:Decrypt",
          "kms:ReEncrypt*",
          "kms:GenerateDataKey*",
          "kms:DescribeKey"
        ]
        Resource = "*"
      }
    ]
  })

  tags = {
    Name        = "trustchain-hsm-backup-key-${var.environment}"
    Environment = var.environment
    Purpose     = "HSM Backup Encryption"
  }
}

resource "aws_kms_alias" "hsm_backup" {
  count = var.enable_hsm && var.enable_backup ? 1 : 0

  name          = "alias/trustchain-hsm-backup-${var.environment}"
  target_key_id = aws_kms_key.hsm_backup[0].key_id
}

# Data source for current account
data "aws_caller_identity" "current" {}