# Backup Module - AWS Backup and Disaster Recovery for TrustChain

# KMS Key for Backup Encryption
resource "aws_kms_key" "backup" {
  description             = "TrustChain backup encryption key for ${var.environment}"
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
        Sid    = "Allow AWS Backup Service"
        Effect = "Allow"
        Principal = {
          Service = "backup.amazonaws.com"
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
    Name        = "trustchain-backup-key-${var.environment}"
    Environment = var.environment
    Purpose     = "Backup Encryption"
  }
}

resource "aws_kms_alias" "backup" {
  name          = "alias/trustchain-backup-${var.environment}"
  target_key_id = aws_kms_key.backup.key_id
}

# AWS Backup Vault
resource "aws_backup_vault" "trustchain" {
  name        = "trustchain-backup-vault-${var.environment}"
  kms_key_arn = aws_kms_key.backup.arn

  tags = {
    Name        = "trustchain-backup-vault-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain Data Protection"
  }
}

# Backup Vault Lock Policy (for compliance)
resource "aws_backup_vault_lock_configuration" "trustchain" {
  count = var.environment == "prod" ? 1 : 0
  
  backup_vault_name   = aws_backup_vault.trustchain.name
  changeable_for_days = 3
  max_retention_days  = var.max_retention_days
  min_retention_days  = var.min_retention_days
}

# IAM Role for AWS Backup
resource "aws_iam_role" "backup" {
  name = "trustchain-backup-role-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "backup.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name        = "trustchain-backup-role-${var.environment}"
    Environment = var.environment
  }
}

# Attach AWS managed policies for backup
resource "aws_iam_role_policy_attachment" "backup_service" {
  role       = aws_iam_role.backup.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSBackupServiceRolePolicyForBackup"
}

resource "aws_iam_role_policy_attachment" "backup_restores" {
  role       = aws_iam_role.backup.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSBackupServiceRolePolicyForRestores"
}

resource "aws_iam_role_policy_attachment" "backup_s3" {
  role       = aws_iam_role.backup.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSBackupServiceRolePolicyForS3Backup"
}

resource "aws_iam_role_policy_attachment" "backup_s3_restore" {
  role       = aws_iam_role.backup.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSBackupServiceRolePolicyForS3Restore"
}

# Backup Plan for TrustChain Critical Data
resource "aws_backup_plan" "trustchain_critical" {
  name = "trustchain-critical-backup-${var.environment}"

  # Critical data - Multiple daily backups
  rule {
    rule_name         = "critical_data_backup"
    target_vault_name = aws_backup_vault.trustchain.name
    schedule          = "cron(0 */6 * * ? *)" # Every 6 hours

    start_window      = 60   # 1 hour
    completion_window = 300  # 5 hours

    lifecycle {
      cold_storage_after = 30
      delete_after       = var.critical_retention_days
    }

    recovery_point_tags = {
      BackupType  = "Critical"
      Environment = var.environment
      Frequency   = "6-Hourly"
    }

    copy_action {
      destination_vault_arn = var.cross_region_vault_arn != "" ? var.cross_region_vault_arn : aws_backup_vault.trustchain.arn

      lifecycle {
        cold_storage_after = 30
        delete_after       = var.critical_retention_days
      }
    }
  }

  # Standard data - Daily backups
  rule {
    rule_name         = "standard_data_backup"
    target_vault_name = aws_backup_vault.trustchain.name
    schedule          = "cron(0 2 * * ? *)" # Daily at 2 AM

    start_window      = 60   # 1 hour
    completion_window = 480  # 8 hours

    lifecycle {
      cold_storage_after = 30
      delete_after       = var.standard_retention_days
    }

    recovery_point_tags = {
      BackupType  = "Standard"
      Environment = var.environment
      Frequency   = "Daily"
    }

    copy_action {
      destination_vault_arn = var.cross_region_vault_arn != "" ? var.cross_region_vault_arn : aws_backup_vault.trustchain.arn

      lifecycle {
        cold_storage_after = 30
        delete_after       = var.standard_retention_days
      }
    }
  }

  # Weekly backups for long-term retention
  rule {
    rule_name         = "weekly_backup"
    target_vault_name = aws_backup_vault.trustchain.name
    schedule          = "cron(0 3 ? * SUN *)" # Weekly on Sunday at 3 AM

    start_window      = 60   # 1 hour
    completion_window = 480  # 8 hours

    lifecycle {
      cold_storage_after = 90
      delete_after       = var.long_term_retention_days
    }

    recovery_point_tags = {
      BackupType  = "Weekly"
      Environment = var.environment
      Frequency   = "Weekly"
    }

    copy_action {
      destination_vault_arn = var.cross_region_vault_arn != "" ? var.cross_region_vault_arn : aws_backup_vault.trustchain.arn

      lifecycle {
        cold_storage_after = 90
        delete_after       = var.long_term_retention_days
      }
    }
  }

  tags = {
    Name        = "trustchain-critical-backup-plan-${var.environment}"
    Environment = var.environment
    DataType    = "Critical"
  }
}

# Backup Selection for S3 Buckets
resource "aws_backup_selection" "s3_buckets" {
  iam_role_arn = aws_iam_role.backup.arn
  name         = "trustchain-s3-backup-${var.environment}"
  plan_id      = aws_backup_plan.trustchain_critical.id

  resources = var.s3_bucket_arns

  condition {
    string_equals {
      key   = "aws:ResourceTag/Environment"
      value = var.environment
    }
  }

  condition {
    string_like {
      key   = "aws:ResourceTag/Purpose"
      value = "*TrustChain*"
    }
  }
}

# Backup Selection for EC2 Instances
resource "aws_backup_selection" "ec2_instances" {
  count = length(var.instance_ids) > 0 ? 1 : 0
  
  iam_role_arn = aws_iam_role.backup.arn
  name         = "trustchain-ec2-backup-${var.environment}"
  plan_id      = aws_backup_plan.trustchain_critical.id

  resources = [for id in var.instance_ids : "arn:aws:ec2:${data.aws_region.current.name}:${data.aws_caller_identity.current.account_id}:instance/${id}"]

  condition {
    string_equals {
      key   = "aws:ResourceTag/Environment"
      value = var.environment
    }
  }
}

# CloudWatch Alarms for Backup Monitoring
resource "aws_cloudwatch_metric_alarm" "backup_job_failed" {
  alarm_name          = "trustchain-backup-job-failed-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "1"
  metric_name        = "NumberOfBackupJobsFailed"
  namespace          = "AWS/Backup"
  period             = "300"
  statistic          = "Sum"
  threshold          = "0"
  alarm_description  = "TrustChain backup job failed"

  dimensions = {
    BackupVaultName = aws_backup_vault.trustchain.name
  }

  alarm_actions = compact([var.sns_topic_arn])

  tags = {
    Name        = "trustchain-backup-failed-alarm-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_cloudwatch_metric_alarm" "backup_vault_size" {
  alarm_name          = "trustchain-backup-vault-size-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "1"
  metric_name        = "BackupVaultSizeBytes"
  namespace          = "AWS/Backup"
  period             = "86400" # Daily check
  statistic          = "Average"
  threshold          = "1099511627776" # 1TB
  alarm_description  = "TrustChain backup vault size growing too large"

  dimensions = {
    BackupVaultName = aws_backup_vault.trustchain.name
  }

  alarm_actions = compact([var.sns_topic_arn])

  tags = {
    Name        = "trustchain-backup-size-alarm-${var.environment}"
    Environment = var.environment
  }
}

# Backup Report Plan
resource "aws_backup_report_plan" "trustchain" {
  name        = "trustchain-backup-report-${var.environment}"
  description = "Backup compliance report for TrustChain"

  report_delivery_channel {
    destination_bucket = var.backup_reports_bucket != "" ? var.backup_reports_bucket : "trustchain-backup-reports-${var.environment}-${random_id.bucket_suffix.hex}"
    s3_bucket_region   = data.aws_region.current.name
    s3_key_prefix      = "backup-reports/"
  }

  report_setting {
    report_template = "BACKUP_JOB_REPORT"
    
    framework_arns = [
      aws_backup_framework.trustchain_compliance.arn
    ]
  }

  tags = {
    Name        = "trustchain-backup-report-${var.environment}"
    Environment = var.environment
    Purpose     = "Backup Compliance Reporting"
  }
}

# Backup Framework for Compliance
resource "aws_backup_framework" "trustchain_compliance" {
  name        = "trustchain-compliance-framework-${var.environment}"
  description = "Backup compliance framework for TrustChain"

  control {
    name = "BACKUP_RECOVERY_POINT_ENCRYPTED"

    input_parameter {
      parameter_name  = "requiredEncryptionKeyArns"
      parameter_value = aws_kms_key.backup.arn
    }
  }

  control {
    name = "BACKUP_RECOVERY_POINT_MINIMUM_RETENTION_CHECK"

    input_parameter {
      parameter_name  = "requiredRetentionDays"
      parameter_value = tostring(var.min_retention_days)
    }
  }

  control {
    name = "BACKUP_PLAN_MIN_FREQUENCY_AND_MIN_RETENTION_CHECK"

    input_parameter {
      parameter_name  = "requiredFrequencyUnit"
      parameter_value = "hours"
    }

    input_parameter {
      parameter_name  = "requiredFrequencyValue"
      parameter_value = "24"
    }

    input_parameter {
      parameter_name  = "requiredRetentionDays"
      parameter_value = tostring(var.min_retention_days)
    }
  }

  tags = {
    Name        = "trustchain-compliance-framework-${var.environment}"
    Environment = var.environment
    Purpose     = "Backup Compliance"
  }
}

# EventBridge Rule for Backup Events
resource "aws_cloudwatch_event_rule" "backup_events" {
  name        = "trustchain-backup-events-${var.environment}"
  description = "Capture backup job events for TrustChain"

  event_pattern = jsonencode({
    source      = ["aws.backup"]
    detail-type = ["Backup Job State Change"]
    detail = {
      state = ["COMPLETED", "FAILED", "ABORTED"]
      backupVaultName = [aws_backup_vault.trustchain.name]
    }
  })

  tags = {
    Name        = "trustchain-backup-events-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_cloudwatch_event_target" "backup_events_log" {
  rule      = aws_cloudwatch_event_rule.backup_events.name
  target_id = "LogBackupEvents"
  arn       = aws_cloudwatch_log_group.backup_events.arn
}

# CloudWatch Log Group for Backup Events
resource "aws_cloudwatch_log_group" "backup_events" {
  name              = "/aws/events/trustchain-backup-${var.environment}"
  retention_in_days = 90

  tags = {
    Name        = "trustchain-backup-events-${var.environment}"
    Environment = var.environment
    Purpose     = "Backup Event Logging"
  }
}

# Data sources
data "aws_caller_identity" "current" {}
data "aws_region" "current" {}

# Random ID for bucket naming
resource "random_id" "bucket_suffix" {
  byte_length = 4
}