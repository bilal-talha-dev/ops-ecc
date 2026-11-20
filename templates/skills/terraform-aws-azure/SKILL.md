---
name: terraform-aws-azure
description: Production Terraform & OpenTofu patterns for AWS (VPC, ECS Fargate, RDS, S3, SQS, KMS) and Azure (VNet, Container Apps, AKS, Azure SQL, Blob) with state locking and CIS compliance.
---

# Skill: Production Terraform for AWS & Azure (`terraform-aws-azure`)

## 1. Remote State & Provider Locking Pattern
Always include a hardened `versions.tf` with encrypted remote state and state locking:
```hcl
terraform {
  required_version = ">= 1.7.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.60"
    }
  }
  backend "s3" {
    bucket         = "org-prod-tfstate-bucket"
    key            = "prod/main.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "org-prod-tfstate-locks"
  }
}
```

## 2. Mandatory Resource Hardening Checklist
- **VPC / VNet**: Minimum 2 Availability Zones, public subnets only for load balancers/NAT, private subnets for compute, isolated subnets for RDS/Azure SQL, and VPC Flow Logs enabled.
- **Security Groups / NSGs**: Explicit ingress port/protocol from upstream ALB security group ID only; never `0.0.0.0/0` on ports `22`, `3389`, `5432`, `3306`, `1433`, `27017`, `6379`.
- **RDS / Azure SQL**: `storage_encrypted = true`, `kms_key_id` specified, `deletion_protection = true`, `backup_retention_period = 14`, `publicly_accessible = false`, and `lifecycle { prevent_destroy = true }`.
- **S3 / Azure Blob**: `aws_s3_bucket_server_side_encryption_configuration` (AES256/aws:kms), `aws_s3_bucket_versioning` (`Enabled`), and `aws_s3_bucket_public_access_block` (all 4 booleans `true`).
