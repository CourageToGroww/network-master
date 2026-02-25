# infra/variables.tf

variable "aws_region" {
  description = "AWS region"
  default     = "us-east-1"
}

variable "project_name" {
  description = "Project name prefix for resources"
  default     = "network-master"
}

variable "ssh_key_name" {
  description = "Name of existing EC2 key pair"
  type        = string
}

variable "admin_cidr" {
  description = "CIDR block for SSH access (your IP/32)"
  type        = string
}

variable "backend_instance_type" {
  description = "EC2 instance type for backend"
  default     = "t3.small"
}

variable "frontend_instance_type" {
  description = "EC2 instance type for frontend"
  default     = "t3.micro"
}

variable "db_password" {
  description = "PostgreSQL password"
  type        = string
  sensitive   = true
}

variable "jwt_secret" {
  description = "JWT signing secret"
  type        = string
  sensitive   = true
}
