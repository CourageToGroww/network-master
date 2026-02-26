.PHONY: up deploy infra configure ssh destroy

# Provision infrastructure then build and deploy the app.
# Runs interactive configuration first if terraform.tfvars doesn't exist.
up: configure
	cd infra && terraform init -upgrade && terraform apply -auto-approve
	cd infra && ./deploy.sh

# Build and deploy app only (infra already provisioned).
deploy:
	cd infra && ./deploy.sh

# Provision / update AWS infrastructure only (no app deploy).
infra: configure
	cd infra && terraform init -upgrade && terraform apply -auto-approve

# Interactive configuration — creates infra/terraform.tfvars.
# Skips if already configured (prompts to reconfigure).
configure:
	@cd infra && ./configure.sh

# Open an SSH session to the server.
ssh:
	$(eval IP := $(shell cd infra && terraform output -raw server_public_ip 2>/dev/null))
	$(eval KEY := $(shell grep ssh_key_name infra/terraform.tfvars 2>/dev/null | sed 's/.*= *"\(.*\)"/\1/'))
	ssh -i ~/.ssh/$(KEY).pem ec2-user@$(IP)

# Tear down all AWS infrastructure. Will prompt for confirmation.
destroy:
	cd infra && terraform destroy
