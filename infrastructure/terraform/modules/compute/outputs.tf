# Compute Module Outputs

output "autoscaling_group_name" {
  description = "Auto Scaling Group name"
  value       = aws_autoscaling_group.trustchain.name
}

output "autoscaling_group_arn" {
  description = "Auto Scaling Group ARN"
  value       = aws_autoscaling_group.trustchain.arn
}

output "launch_template_id" {
  description = "Launch template ID"
  value       = aws_launch_template.trustchain.id
}

output "instance_ids" {
  description = "Instance IDs"
  value       = data.aws_instances.trustchain.ids
}

output "instance_ipv6_addresses" {
  description = "Instance IPv6 addresses"
  value       = data.aws_instances.trustchain.ipv6_addresses
}

output "instance_dns_names" {
  description = "Instance DNS names"
  value       = data.aws_instances.trustchain.public_dns_names
}

output "iam_role_arn" {
  description = "IAM role ARN for instances"
  value       = aws_iam_role.trustchain_instance.arn
}

output "iam_instance_profile_name" {
  description = "IAM instance profile name"
  value       = aws_iam_instance_profile.trustchain.name
}

output "placement_group_id" {
  description = "Placement group ID"
  value       = aws_placement_group.trustchain.id
}

output "scale_up_policy_arn" {
  description = "Scale up policy ARN"
  value       = aws_autoscaling_policy.scale_up.arn
}

output "scale_down_policy_arn" {
  description = "Scale down policy ARN"
  value       = aws_autoscaling_policy.scale_down.arn
}

output "cloudwatch_alarms" {
  description = "CloudWatch alarm ARNs"
  value = {
    high_cpu                    = aws_cloudwatch_metric_alarm.high_cpu.arn
    low_cpu                     = aws_cloudwatch_metric_alarm.low_cpu.arn
    high_certificate_operations = aws_cloudwatch_metric_alarm.high_certificate_operations.arn
  }
}

output "ssm_parameters" {
  description = "Systems Manager parameter names"
  value       = [for param in aws_ssm_parameter.trustchain_config : param.name]
}

output "performance_configuration" {
  description = "Performance configuration summary"
  value = {
    instance_type                = var.instance_type
    min_instances               = var.min_instances
    max_instances               = var.max_instances
    desired_instances           = var.desired_instances
    enhanced_networking         = true
    placement_group_strategy    = "cluster"
    ebs_optimized              = true
    ebs_volume_type            = var.ebs_volume_type
    ebs_iops                   = var.ebs_iops
    ebs_throughput_mbps        = var.ebs_throughput_mbps
  }
}