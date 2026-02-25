.PHONY: up deploy infra ssh destroy

# Provision infrastructure (if needed) then build and deploy the app.
# This is the one command for a fresh setup.
up:
	cd infra && terraform init -upgrade && terraform apply -auto-approve
	cd infra && ./deploy.sh

# Build and deploy app only. Use this for code updates after 'make infra'.
deploy:
	cd infra && ./deploy.sh

# Provision / update AWS infrastructure only (no app deploy).
infra:
	cd infra && terraform init -upgrade && terraform apply -auto-approve

# Open an SSH session to the server.
ssh:
	$(eval IP := $(shell cd infra && terraform output -raw server_public_ip 2>/dev/null))
	$(eval KEY := $(shell grep ssh_key_name infra/terraform.tfvars 2>/dev/null | sed 's/.*= *"\(.*\)"/\1/'))
	ssh -i ~/.ssh/$(KEY).pem ec2-user@$(IP)

# Tear down all AWS infrastructure. Will prompt for confirmation.
destroy:
	cd infra && terraform destroy
