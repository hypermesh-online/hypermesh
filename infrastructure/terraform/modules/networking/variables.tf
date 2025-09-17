# Networking Module Variables

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "availability_zones" {
  description = "List of availability zones"
  type        = list(string)
}

variable "ipv6_subnets" {
  description = "IPv6 subnet CIDR blocks"
  type = object({
    public  = list(string)
    private = list(string)
  })
}