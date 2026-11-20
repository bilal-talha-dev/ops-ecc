---
name: devsecops-iam-cis
description: Zero-Trust DevSecOps engineering guide covering OWASP Top 10, CIS Cloud & Docker Benchmarks, IAM privilege boundary design, and SARIF reporting.
---

# Skill: DevSecOps Zero-Trust, IAM & CIS Benchmarks (`devsecops-iam-cis`)

## 1. Secret Remediation Workflow
1. Replace any hardcoded credential (`AKIA...`, private keys, JWT secrets, DB passwords) with runtime secret injection from AWS Secrets Manager, SSM Parameter Store, or Azure Key Vault.
2. Rotate any secret that was ever written to disk or staged in git.

## 2. IAM Least-Privilege & Permission Boundaries
1. Split IAM roles by execution context: build/CI OIDC role vs. ECS task execution role (pulling images/secrets) vs. ECS application task role (accessing specific S3 bucket prefix & SQS queue ARN).
2. Require `aws:SecureTransport: "true"` on all S3 bucket policies and attach IAM Permission Boundaries to delegated roles.
