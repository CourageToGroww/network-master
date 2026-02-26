# ── Security Group ──

resource "aws_security_group" "server" {
  name        = "nm-server-${var.environment}"
  description = "Network Master server - agents and dashboard"
  vpc_id      = aws_vpc.main.id

  # HTTP — agents connect and dashboard loads here
  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "HTTP"
  }

  # HTTPS — when a domain + SSL cert is configured
  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "HTTPS"
  }

  # SSH — admin only
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = [var.admin_ip]
    description = "SSH from admin"
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = { Name = "nm-sg-${var.environment}" }
}

# ── IAM Role ──
# No AWS service access needed — binary is delivered via SCP from your machine.

resource "aws_iam_role" "server" {
  name = "nm-server-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "ec2.amazonaws.com" }
    }]
  })

  tags = { Name = "nm-server-role-${var.environment}" }
}

resource "aws_iam_instance_profile" "server" {
  name = "nm-server-${var.environment}"
  role = aws_iam_role.server.name
}

# ── AMI: Latest Amazon Linux 2023 x86_64 ──

data "aws_ami" "amazon_linux" {
  most_recent = true
  owners      = ["amazon"]

  filter {
    name   = "name"
    values = ["al2023-ami-*-x86_64"]
  }

  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }
}

# ── EC2 Instance ──

resource "aws_instance" "server" {
  ami                    = data.aws_ami.amazon_linux.id
  instance_type          = var.instance_type
  key_name               = var.ssh_key_name
  vpc_security_group_ids = [aws_security_group.server.id]
  subnet_id              = aws_subnet.public.id
  iam_instance_profile   = aws_iam_instance_profile.server.name

  root_block_device {
    volume_size = 30
    volume_type = "gp3"
    encrypted   = true
  }

  user_data = base64encode(templatefile("${path.module}/setup.sh", {
    db_password    = var.db_password
    jwt_secret     = var.jwt_secret
    admin_password = var.admin_password
    domain_name    = var.domain_name
  }))

  tags = { Name = "nm-server-${var.environment}" }
}

# ── Elastic IP — stable public address that survives instance restarts ──

resource "aws_eip" "server" {
  instance = aws_instance.server.id
  domain   = "vpc"
  tags     = { Name = "nm-eip-${var.environment}" }
}
