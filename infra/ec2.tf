# infra/ec2.tf

data "aws_ami" "al2023" {
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

# Backend EC2 (private subnet)
resource "aws_instance" "backend" {
  ami                    = data.aws_ami.al2023.id
  instance_type          = var.backend_instance_type
  key_name               = var.ssh_key_name
  subnet_id              = aws_subnet.private.id
  vpc_security_group_ids = [aws_security_group.backend.id]

  root_block_device {
    volume_size = 30
    volume_type = "gp3"
    encrypted   = true
  }

  user_data = templatefile("${path.module}/scripts/backend-userdata.sh", {
    db_password = var.db_password
    jwt_secret  = var.jwt_secret
  })

  tags = { Name = "${var.project_name}-backend" }
}

# Frontend EC2 (public subnet)
resource "aws_instance" "frontend" {
  ami                         = data.aws_ami.al2023.id
  instance_type               = var.frontend_instance_type
  key_name                    = var.ssh_key_name
  subnet_id                   = aws_subnet.public_a.id
  vpc_security_group_ids      = [aws_security_group.frontend.id]
  associate_public_ip_address = true

  root_block_device {
    volume_size = 20
    volume_type = "gp3"
    encrypted   = true
  }

  user_data = templatefile("${path.module}/scripts/frontend-userdata.sh", {})

  tags = { Name = "${var.project_name}-frontend" }
}
