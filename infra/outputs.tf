# infra/outputs.tf

output "alb_dns_name" {
  description = "ALB DNS name (access the app here)"
  value       = aws_lb.main.dns_name
}

output "frontend_public_ip" {
  description = "Frontend EC2 public IP (for SSH)"
  value       = aws_instance.frontend.public_ip
}

output "backend_private_ip" {
  description = "Backend EC2 private IP"
  value       = aws_instance.backend.private_ip
}

output "ssh_frontend" {
  description = "SSH to frontend"
  value       = "ssh -i ~/.ssh/${var.ssh_key_name}.pem ec2-user@${aws_instance.frontend.public_ip}"
}

output "ssh_backend" {
  description = "SSH to backend (via frontend bastion)"
  value       = "ssh -i ~/.ssh/${var.ssh_key_name}.pem -J ec2-user@${aws_instance.frontend.public_ip} ec2-user@${aws_instance.backend.private_ip}"
}

output "dashboard_url" {
  description = "Dashboard URL"
  value       = "http://${aws_lb.main.dns_name}"
}

output "agent_install_cmd" {
  description = "Agent install command"
  value       = "nm-agent.exe install --server ${aws_lb.main.dns_name}:80"
}

output "nat_gateway_id" {
  description = "NAT Gateway ID (for stop/start scripts)"
  value       = aws_nat_gateway.main.id
}

output "nat_eip_allocation_id" {
  description = "NAT EIP allocation ID (for stop/start scripts)"
  value       = aws_eip.nat.id
}

output "backend_instance_id" {
  description = "Backend EC2 instance ID"
  value       = aws_instance.backend.id
}

output "frontend_instance_id" {
  description = "Frontend EC2 instance ID"
  value       = aws_instance.frontend.id
}

output "aws_region" {
  description = "AWS region"
  value       = var.aws_region
}

output "ssh_key_name" {
  description = "SSH key pair name"
  value       = var.ssh_key_name
}
