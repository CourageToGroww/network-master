output "server_public_ip" {
  description = "Public IP of the Network Master server (Elastic IP)"
  value       = aws_eip.server.public_ip
}

output "ssh_command" {
  description = "SSH command to connect to the server"
  value       = "ssh -i ~/.ssh/${var.ssh_key_name}.pem ec2-user@${aws_eip.server.public_ip}"
}

output "dashboard_url" {
  description = "URL to access the Network Master dashboard"
  value       = var.domain_name != "" ? "https://${var.domain_name}" : "http://${aws_eip.server.public_ip}"
}

output "agent_install_cmd" {
  description = "Command to install the agent on a Windows PC (run as Administrator)"
  value       = "nm-agent.exe install --server ${aws_eip.server.public_ip}:80"
}
