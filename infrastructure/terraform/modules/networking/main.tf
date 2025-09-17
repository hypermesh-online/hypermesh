# Networking Module - IPv6-Only VPC for TrustChain

# IPv6-Only VPC
resource "aws_vpc" "trustchain" {
  cidr_block                       = "10.0.0.0/16" # Required even for IPv6-only
  assign_generated_ipv6_cidr_block = true
  enable_dns_hostnames            = true
  enable_dns_support              = true

  tags = {
    Name        = "trustchain-vpc-${var.environment}"
    Environment = var.environment
    Purpose     = "TrustChain IPv6-Only Infrastructure"
  }
}

# Internet Gateway for IPv6
resource "aws_internet_gateway" "trustchain" {
  vpc_id = aws_vpc.trustchain.id

  tags = {
    Name        = "trustchain-igw-${var.environment}"
    Environment = var.environment
  }
}

# Egress-Only Internet Gateway for IPv6 private subnets
resource "aws_egress_only_internet_gateway" "trustchain" {
  vpc_id = aws_vpc.trustchain.id

  tags = {
    Name        = "trustchain-eigw-${var.environment}"
    Environment = var.environment
  }
}

# Public Subnets (IPv6-only)
resource "aws_subnet" "public" {
  count = length(var.availability_zones)

  vpc_id                          = aws_vpc.trustchain.id
  availability_zone               = var.availability_zones[count.index]
  cidr_block                      = cidrsubnet("10.0.0.0/16", 8, count.index + 1)
  ipv6_cidr_block                = var.ipv6_subnets.public[count.index]
  assign_ipv6_address_on_creation = true
  map_public_ip_on_launch        = false # IPv6-only, no IPv4 public IPs

  tags = {
    Name        = "trustchain-public-${var.availability_zones[count.index]}-${var.environment}"
    Environment = var.environment
    Type        = "public"
    Tier        = "web"
  }
}

# Private Subnets (IPv6-only)
resource "aws_subnet" "private" {
  count = length(var.availability_zones)

  vpc_id                          = aws_vpc.trustchain.id
  availability_zone               = var.availability_zones[count.index]
  cidr_block                      = cidrsubnet("10.0.0.0/16", 8, count.index + 10)
  ipv6_cidr_block                = var.ipv6_subnets.private[count.index]
  assign_ipv6_address_on_creation = true

  tags = {
    Name        = "trustchain-private-${var.availability_zones[count.index]}-${var.environment}"
    Environment = var.environment
    Type        = "private"
    Tier        = "data"
  }
}

# Route Table for Public Subnets
resource "aws_route_table" "public" {
  vpc_id = aws_vpc.trustchain.id

  # IPv6 route to Internet Gateway
  route {
    ipv6_cidr_block = "::/0"
    gateway_id      = aws_internet_gateway.trustchain.id
  }

  tags = {
    Name        = "trustchain-public-rt-${var.environment}"
    Environment = var.environment
    Type        = "public"
  }
}

# Route Table for Private Subnets
resource "aws_route_table" "private" {
  vpc_id = aws_vpc.trustchain.id

  # IPv6 route to Egress-Only Internet Gateway
  route {
    ipv6_cidr_block        = "::/0"
    egress_only_gateway_id = aws_egress_only_internet_gateway.trustchain.id
  }

  tags = {
    Name        = "trustchain-private-rt-${var.environment}"
    Environment = var.environment
    Type        = "private"
  }
}

# Route Table Associations - Public
resource "aws_route_table_association" "public" {
  count = length(aws_subnet.public)

  subnet_id      = aws_subnet.public[count.index].id
  route_table_id = aws_route_table.public.id
}

# Route Table Associations - Private
resource "aws_route_table_association" "private" {
  count = length(aws_subnet.private)

  subnet_id      = aws_subnet.private[count.index].id
  route_table_id = aws_route_table.private.id
}

# VPC Flow Logs for Security Monitoring
resource "aws_flow_log" "trustchain" {
  iam_role_arn    = aws_iam_role.flow_log.arn
  log_destination = aws_cloudwatch_log_group.flow_log.arn
  traffic_type    = "ALL"
  vpc_id          = aws_vpc.trustchain.id
}

# CloudWatch Log Group for VPC Flow Logs
resource "aws_cloudwatch_log_group" "flow_log" {
  name              = "/aws/vpc/flowlogs/trustchain-${var.environment}"
  retention_in_days = 30

  tags = {
    Name        = "trustchain-flow-logs-${var.environment}"
    Environment = var.environment
  }
}

# IAM Role for VPC Flow Logs
resource "aws_iam_role" "flow_log" {
  name = "trustchain-flowlog-role-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "vpc-flow-logs.amazonaws.com"
        }
      }
    ]
  })
}

# IAM Policy for VPC Flow Logs
resource "aws_iam_role_policy" "flow_log" {
  name = "trustchain-flowlog-policy-${var.environment}"
  role = aws_iam_role.flow_log.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents",
          "logs:DescribeLogGroups",
          "logs:DescribeLogStreams"
        ]
        Effect = "Allow"
        Resource = "*"
      }
    ]
  })
}

# VPC Endpoint for S3 (for private subnet access)
resource "aws_vpc_endpoint" "s3" {
  vpc_id            = aws_vpc.trustchain.id
  service_name      = "com.amazonaws.${data.aws_region.current.name}.s3"
  vpc_endpoint_type = "Gateway"
  route_table_ids   = [aws_route_table.private.id]

  tags = {
    Name        = "trustchain-s3-endpoint-${var.environment}"
    Environment = var.environment
  }
}

# VPC Endpoint for CloudWatch (for private subnet monitoring)
resource "aws_vpc_endpoint" "cloudwatch" {
  vpc_id              = aws_vpc.trustchain.id
  service_name        = "com.amazonaws.${data.aws_region.current.name}.monitoring"
  vpc_endpoint_type   = "Interface"
  subnet_ids          = aws_subnet.private[*].id
  security_group_ids  = [aws_security_group.vpc_endpoints.id]
  private_dns_enabled = true

  tags = {
    Name        = "trustchain-cloudwatch-endpoint-${var.environment}"
    Environment = var.environment
  }
}

# Security Group for VPC Endpoints
resource "aws_security_group" "vpc_endpoints" {
  name        = "trustchain-vpc-endpoints-${var.environment}"
  description = "Security group for VPC endpoints"
  vpc_id      = aws_vpc.trustchain.id

  ingress {
    from_port        = 443
    to_port          = 443
    protocol         = "tcp"
    ipv6_cidr_blocks = [aws_vpc.trustchain.ipv6_cidr_block]
    description      = "HTTPS from VPC IPv6"
  }

  egress {
    from_port        = 0
    to_port          = 0
    protocol         = "-1"
    ipv6_cidr_blocks = ["::/0"]
    description      = "All outbound IPv6 traffic"
  }

  tags = {
    Name        = "trustchain-vpc-endpoints-sg-${var.environment}"
    Environment = var.environment
  }
}

# Data source for current region
data "aws_region" "current" {}

# Network ACLs for additional security
resource "aws_network_acl" "public" {
  vpc_id     = aws_vpc.trustchain.id
  subnet_ids = aws_subnet.public[*].id

  # Allow HTTPS inbound (IPv6)
  ingress {
    protocol         = "tcp"
    rule_no          = 100
    action           = "allow"
    ipv6_cidr_block  = "::/0"
    from_port        = 8443
    to_port          = 8443
  }

  # Allow CT inbound (IPv6)
  ingress {
    protocol         = "tcp"
    rule_no          = 110
    action           = "allow"
    ipv6_cidr_block  = "::/0"
    from_port        = 6962
    to_port          = 6962
  }

  # Allow DNS-over-QUIC inbound (IPv6)
  ingress {
    protocol         = "udp"
    rule_no          = 120
    action           = "allow"
    ipv6_cidr_block  = "::/0"
    from_port        = 8853
    to_port          = 8853
  }

  # Allow STOQ Protocol inbound (IPv6)
  ingress {
    protocol         = "udp"
    rule_no          = 130
    action           = "allow"
    ipv6_cidr_block  = "::/0"
    from_port        = 8444
    to_port          = 8444
  }

  # Allow HyperMesh Integration inbound (IPv6)
  ingress {
    protocol         = "tcp"
    rule_no          = 140
    action           = "allow"
    ipv6_cidr_block  = "::/0"
    from_port        = 8445
    to_port          = 8445
  }

  # Allow Integration API inbound (IPv6)
  ingress {
    protocol         = "tcp"
    rule_no          = 150
    action           = "allow"
    ipv6_cidr_block  = "::/0"
    from_port        = 8446
    to_port          = 8446
  }

  # Allow ephemeral ports for responses
  ingress {
    protocol         = "tcp"
    rule_no          = 200
    action           = "allow"
    ipv6_cidr_block  = "::/0"
    from_port        = 32768
    to_port          = 65535
  }

  # Allow all outbound
  egress {
    protocol         = "-1"
    rule_no          = 100
    action           = "allow"
    ipv6_cidr_block  = "::/0"
    from_port        = 0
    to_port          = 0
  }

  tags = {
    Name        = "trustchain-public-nacl-${var.environment}"
    Environment = var.environment
  }
}

resource "aws_network_acl" "private" {
  vpc_id     = aws_vpc.trustchain.id
  subnet_ids = aws_subnet.private[*].id

  # Allow all traffic from VPC IPv6 CIDR
  ingress {
    protocol         = "-1"
    rule_no          = 100
    action           = "allow"
    ipv6_cidr_block  = aws_vpc.trustchain.ipv6_cidr_block
    from_port        = 0
    to_port          = 0
  }

  # Allow all outbound
  egress {
    protocol         = "-1"
    rule_no          = 100
    action           = "allow"
    ipv6_cidr_block  = "::/0"
    from_port        = 0
    to_port          = 0
  }

  tags = {
    Name        = "trustchain-private-nacl-${var.environment}"
    Environment = var.environment
  }
}