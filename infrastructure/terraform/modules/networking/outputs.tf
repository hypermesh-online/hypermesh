# Networking Module Outputs

output "vpc_id" {
  description = "VPC ID"
  value       = aws_vpc.trustchain.id
}

output "vpc_ipv6_cidr_block" {
  description = "VPC IPv6 CIDR block"
  value       = aws_vpc.trustchain.ipv6_cidr_block
}

output "internet_gateway_id" {
  description = "Internet Gateway ID"
  value       = aws_internet_gateway.trustchain.id
}

output "egress_only_gateway_id" {
  description = "Egress-Only Internet Gateway ID"
  value       = aws_egress_only_internet_gateway.trustchain.id
}

output "public_subnet_ids" {
  description = "Public subnet IDs"
  value       = aws_subnet.public[*].id
}

output "private_subnet_ids" {
  description = "Private subnet IDs"
  value       = aws_subnet.private[*].id
}

output "public_subnet_ipv6_cidr_blocks" {
  description = "Public subnet IPv6 CIDR blocks"
  value       = aws_subnet.public[*].ipv6_cidr_block
}

output "private_subnet_ipv6_cidr_blocks" {
  description = "Private subnet IPv6 CIDR blocks"
  value       = aws_subnet.private[*].ipv6_cidr_block
}

output "public_route_table_id" {
  description = "Public route table ID"
  value       = aws_route_table.public.id
}

output "private_route_table_id" {
  description = "Private route table ID"
  value       = aws_route_table.private.id
}

output "vpc_endpoints" {
  description = "VPC endpoint information"
  value = {
    s3_endpoint_id         = aws_vpc_endpoint.s3.id
    cloudwatch_endpoint_id = aws_vpc_endpoint.cloudwatch.id
  }
}

output "flow_log_id" {
  description = "VPC Flow Log ID"
  value       = aws_flow_log.trustchain.id
}

output "availability_zones" {
  description = "Availability zones used"
  value       = var.availability_zones
}