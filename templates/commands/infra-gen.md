---
description: Generate or update production-hardened Terraform (AWS/Azure) or AWS CloudFormation modules synchronized with the application codebase.
---

# `/infra-gen` — Production Infrastructure-as-Code Generator

1. Scan the repository for environment variables, database connections, object storage buckets, message queues, and exposed ports.
2. Generate or update modular Terraform (`.tf`) or CloudFormation (`template.yaml`) resources adhering to:
   - Remote state locking (`s3` + DynamoDB or `azurerm` Blob).
   - KMS/CMK encryption at rest on all storage, databases, and queues.
   - Private networking, zero `0.0.0.0/0` admin/DB ingress, and least-privilege IAM/RBAC policies.
   - `prevent_destroy = true` / `DeletionPolicy: Retain` on stateful resources.
3. Run `ops-ecc audit --strict` to verify zero security findings in the generated IaC.
