variable "aws_region" {
  description = "AWS region to deploy into"
  type        = string
  default     = "us-east-1"
}

variable "ssh_key_name" {
  description = "Name of the SSH key pair in AWS (without .pem extension)"
  type        = string
}

variable "admin_ip" {
  description = "Your public IP for SSH access in CIDR notation (e.g. 1.2.3.4/32)"
  type        = string
}

variable "environment" {
  description = "Environment name, used in resource naming"
  type        = string
  default     = "prod"
}

variable "instance_type" {
  description = "EC2 instance type"
  type        = string
  default     = "t2.micro"
}

variable "jwt_secret" {
  description = "JWT signing secret — generate with: openssl rand -hex 32"
  type        = string
  sensitive   = true
}

variable "db_password" {
  description = "PostgreSQL password for nm_user"
  type        = string
  sensitive   = true
}

variable "admin_password" {
  description = "Initial admin dashboard password"
  type        = string
  sensitive   = true
}

variable "domain_name" {
  description = "Domain name for SSL via Let's Encrypt (optional — leave empty for IP-only)"
  type        = string
  default     = ""
}
