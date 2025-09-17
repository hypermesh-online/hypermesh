# Storage Module - S3 Buckets for TrustChain

# KMS Key for S3 Encryption
resource "aws_kms_key" "trustchain" {
  description             = "TrustChain S3 encryption key for ${var.environment}"
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
        Sid    = "Allow TrustChain Services"
        Effect = "Allow"
        Principal = {
          Service = [
            "s3.amazonaws.com",
            "logs.amazonaws.com"
          ]
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
    Name        = "trustchain-s3-key-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain S3 Encryption"
  }
}

resource "aws_kms_alias" "trustchain" {
  name          = "alias/trustchain-s3-${var.environment}"
  target_key_id = aws_kms_key.trustchain.key_id
}

# Certificate Transparency Logs Bucket
resource "aws_s3_bucket" "ct_logs" {
  bucket = "trustchain-ct-logs-${var.environment}-${random_id.bucket_suffix.hex}"

  tags = {
    Name        = "trustchain-ct-logs-${var.environment}"
    Environment = var.environment
    Purpose     = "Certificate Transparency Logs"
    DataType    = "CT-Logs"
    Compliance  = "Required-7-Years"
  }
}

resource "aws_s3_bucket_versioning" "ct_logs" {
  bucket = aws_s3_bucket.ct_logs.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_encryption" "ct_logs" {
  bucket = aws_s3_bucket.ct_logs.id

  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        kms_master_key_id = aws_kms_key.trustchain.arn
        sse_algorithm     = "aws:kms"
      }
      bucket_key_enabled = true
    }
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "ct_logs" {
  bucket = aws_s3_bucket.ct_logs.id

  rule {
    id     = "ct_logs_lifecycle"
    status = "Enabled"

    transition {
      days          = 30
      storage_class = "STANDARD_IA"
    }

    transition {
      days          = 90
      storage_class = "GLACIER"
    }

    transition {
      days          = 365
      storage_class = "DEEP_ARCHIVE"
    }

    expiration {
      days = var.certificate_transparency_retention_days # 7 years for legal compliance
    }

    noncurrent_version_transition {
      noncurrent_days = 30
      storage_class   = "STANDARD_IA"
    }

    noncurrent_version_expiration {
      noncurrent_days = 90
    }
  }
}

resource "aws_s3_bucket_public_access_block" "ct_logs" {
  bucket = aws_s3_bucket.ct_logs.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Certificates Storage Bucket
resource "aws_s3_bucket" "certificates" {
  bucket = "trustchain-certificates-${var.environment}-${random_id.bucket_suffix.hex}"

  tags = {
    Name        = "trustchain-certificates-${var.environment}"
    Environment = var.environment
    Purpose     = "Certificate Storage"
    DataType    = "Certificates"
    Compliance  = "Required"
  }
}

resource "aws_s3_bucket_versioning" "certificates" {
  bucket = aws_s3_bucket.certificates.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_encryption" "certificates" {
  bucket = aws_s3_bucket.certificates.id

  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        kms_master_key_id = aws_kms_key.trustchain.arn
        sse_algorithm     = "aws:kms"
      }
      bucket_key_enabled = true
    }
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "certificates" {
  bucket = aws_s3_bucket.certificates.id

  rule {
    id     = "certificates_lifecycle"
    status = "Enabled"

    transition {
      days          = 30
      storage_class = "STANDARD_IA"
    }

    transition {
      days          = 90
      storage_class = "GLACIER"
    }

    # Keep certificates longer than CT logs
    expiration {
      days = var.certificate_transparency_retention_days + 365 # 8 years
    }

    noncurrent_version_expiration {
      noncurrent_days = 365
    }
  }
}

resource "aws_s3_bucket_public_access_block" "certificates" {
  bucket = aws_s3_bucket.certificates.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Configuration and Keys Bucket (Highly Sensitive)
resource "aws_s3_bucket" "config" {
  bucket = "trustchain-config-${var.environment}-${random_id.bucket_suffix.hex}"

  tags = {
    Name        = "trustchain-config-${var.environment}"
    Environment = var.environment
    Purpose     = "Configuration and Keys"
    DataType    = "Configuration"
    Security    = "High"
  }
}

resource "aws_s3_bucket_versioning" "config" {
  bucket = aws_s3_bucket.config.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_encryption" "config" {
  bucket = aws_s3_bucket.config.id

  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        kms_master_key_id = aws_kms_key.trustchain.arn
        sse_algorithm     = "aws:kms"
      }
      bucket_key_enabled = true
    }
  }
}

resource "aws_s3_bucket_public_access_block" "config" {
  bucket = aws_s3_bucket.config.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Backup Bucket (Cross-Region Replication Target)
resource "aws_s3_bucket" "backup" {
  count = var.enable_cross_region_backup ? 1 : 0

  provider = aws.backup_region
  bucket   = "trustchain-backup-${var.environment}-${random_id.bucket_suffix.hex}"

  tags = {
    Name        = "trustchain-backup-${var.environment}"
    Environment = var.environment
    Purpose     = "Cross-Region Backup"
    DataType    = "Backup"
  }
}

resource "aws_s3_bucket_versioning" "backup" {
  count = var.enable_cross_region_backup ? 1 : 0

  provider = aws.backup_region
  bucket   = aws_s3_bucket.backup[0].id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_encryption" "backup" {
  count = var.enable_cross_region_backup ? 1 : 0

  provider = aws.backup_region
  bucket   = aws_s3_bucket.backup[0].id

  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        sse_algorithm = "AES256"
      }
    }
  }
}

# Cross-Region Replication IAM Role
resource "aws_iam_role" "replication" {
  count = var.enable_cross_region_backup ? 1 : 0
  name  = "trustchain-replication-role-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "s3.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name        = "trustchain-replication-role-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_iam_role_policy" "replication" {
  count = var.enable_cross_region_backup ? 1 : 0
  name  = "trustchain-replication-policy-${var.environment}"
  role  = aws_iam_role.replication[0].id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = [
          "s3:GetObjectVersionForReplication",
          "s3:GetObjectVersionAcl"
        ]
        Effect = "Allow"
        Resource = [
          "${aws_s3_bucket.ct_logs.arn}/*",
          "${aws_s3_bucket.certificates.arn}/*"
        ]
      },
      {
        Action = [
          "s3:ListBucket"
        ]
        Effect = "Allow"
        Resource = [
          aws_s3_bucket.ct_logs.arn,
          aws_s3_bucket.certificates.arn
        ]
      },
      {
        Action = [
          "s3:ReplicateObject",
          "s3:ReplicateDelete"
        ]
        Effect = "Allow"
        Resource = var.enable_cross_region_backup ? "${aws_s3_bucket.backup[0].arn}/*" : ""
      },
      {
        Action = [
          "kms:Decrypt"
        ]
        Effect = "Allow"
        Resource = aws_kms_key.trustchain.arn
      }
    ]
  })
}

# Cross-Region Replication Configuration
resource "aws_s3_bucket_replication_configuration" "ct_logs" {
  count = var.enable_cross_region_backup ? 1 : 0

  role   = aws_iam_role.replication[0].arn
  bucket = aws_s3_bucket.ct_logs.id

  rule {
    id     = "replicate_ct_logs"
    status = "Enabled"

    destination {
      bucket        = aws_s3_bucket.backup[0].arn
      storage_class = "STANDARD_IA"
    }
  }

  depends_on = [aws_s3_bucket_versioning.ct_logs]
}

resource "aws_s3_bucket_replication_configuration" "certificates" {
  count = var.enable_cross_region_backup ? 1 : 0

  role   = aws_iam_role.replication[0].arn
  bucket = aws_s3_bucket.certificates.id

  rule {
    id     = "replicate_certificates"
    status = "Enabled"

    destination {
      bucket        = aws_s3_bucket.backup[0].arn
      storage_class = "STANDARD"
    }
  }

  depends_on = [aws_s3_bucket_versioning.certificates]
}

# S3 Bucket Notifications for Monitoring
resource "aws_s3_bucket_notification" "ct_logs" {
  bucket = aws_s3_bucket.ct_logs.id

  cloudwatch_configuration {
    cloudwatch_configuration_id = "ct-logs-events"
    events                     = ["s3:ObjectCreated:*", "s3:ObjectRemoved:*"]
  }
}

# Data source
data "aws_caller_identity" "current" {}

# Random ID for bucket naming
resource "random_id" "bucket_suffix" {
  byte_length = 4
}

# CloudWatch Metrics for S3 Buckets
resource "aws_cloudwatch_metric_alarm" "ct_logs_size" {
  alarm_name          = "trustchain-ct-logs-size-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "1"
  metric_name        = "BucketSizeBytes"
  namespace          = "AWS/S3"
  period             = "86400" # Daily check
  statistic          = "Average"
  threshold          = "1073741824000" # 1TB
  alarm_description  = "CT logs bucket size monitoring"

  dimensions = {
    BucketName  = aws_s3_bucket.ct_logs.bucket
    StorageType = "StandardStorage"
  }

  tags = {
    Name        = "trustchain-ct-logs-size-${var.environment}"
    Environment = var.environment
  }
}

# S3 Bucket Policies for Access Control
resource "aws_s3_bucket_policy" "ct_logs" {
  bucket = aws_s3_bucket.ct_logs.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "DenyInsecureConnections"
        Effect = "Deny"
        Principal = "*"
        Action = "s3:*"
        Resource = [
          aws_s3_bucket.ct_logs.arn,
          "${aws_s3_bucket.ct_logs.arn}/*"
        ]
        Condition = {
          Bool = {
            "aws:SecureTransport" = "false"
          }
        }
      },
      {
        Sid    = "AllowTrustChainAccess"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:role/trustchain-instance-role-${var.environment}"
        }
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:DeleteObject",
          "s3:ListBucket"
        ]
        Resource = [
          aws_s3_bucket.ct_logs.arn,
          "${aws_s3_bucket.ct_logs.arn}/*"
        ]
      }
    ]
  })
}

resource "aws_s3_bucket_policy" "certificates" {
  bucket = aws_s3_bucket.certificates.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "DenyInsecureConnections"
        Effect = "Deny"
        Principal = "*"
        Action = "s3:*"
        Resource = [
          aws_s3_bucket.certificates.arn,
          "${aws_s3_bucket.certificates.arn}/*"
        ]
        Condition = {
          Bool = {
            "aws:SecureTransport" = "false"
          }
        }
      },
      {
        Sid    = "AllowTrustChainAccess"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:role/trustchain-instance-role-${var.environment}"
        }
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:DeleteObject",
          "s3:ListBucket"
        ]
        Resource = [
          aws_s3_bucket.certificates.arn,
          "${aws_s3_bucket.certificates.arn}/*"
        ]
      }
    ]
  })
}