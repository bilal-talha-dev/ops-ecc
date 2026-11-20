# Rule 03: DevSecOps Zero-Trust, CIS Benchmarks & IAM Least-Privilege

<!-- USER_CUSTOM_START -->
<!-- Add custom security compliance exemptions or security gate overrides here. -->
<!-- USER_CUSTOM_END -->

## 1. Zero Hardcoded Secrets & Credential Hygiene
1. Never commit AWS Access Keys (`AKIA...`), Secret Keys, Azure Storage/ServiceBus connection strings, PEM private keys, JWT signing secrets, or database URIs with embedded passwords.
2. Pre-commit and CI gates run `ops-ecc audit --strict` and `ops-ecc hook pre-commit` to block any secret leak before it enters git history.

## 2. Network & Compute Hardening (CIS Benchmark Alignment)
1. **Zero Public Administrative/Database Ingress**:
   - Security Groups (`aws_security_group`) and Azure NSGs (`azurerm_network_security_group`) MUST NEVER allow `0.0.0.0/0` or `::/0` ingress on SSH (`22`), RDP (`3389`), PostgreSQL (`5432`), MySQL (`3306`), MSSQL (`1433`), MongoDB (`27017`), or Redis (`6379`).
   - Route administrative access exclusively through AWS Systems Manager Session Manager, Cloudflare Zero Trust Tunnels, or private bastions.
2. **Encryption Everywhere**:
   - Enforce KMS/CMK encryption at rest on EBS volumes (`encrypted = true`), RDS instances (`storage_encrypted = true`), S3 buckets (`aws_s3_bucket_server_side_encryption_configuration`), SQS queues (`kms_master_key_id`), and Azure Storage/SQL (`https_only = true`, TLS 1.2+).
   - S3 buckets MUST include `aws_s3_bucket_public_access_block` with all four flags set to `true`.

## 3. IAM & RBAC Least-Privilege
1. Never author IAM policies with `"Action": "*"` or `"Resource": "*"` on mutating or data-plane permissions.
2. Scope every IAM statement to exact actions (e.g., `s3:GetObject`, `sqs:SendMessage`) and specific resource ARNs with condition keys (`aws:PrincipalOrgID`, `aws:SourceVpce`, `aws:SecureTransport`).

## 4. Container & Kubernetes Security
1. Every `Dockerfile` MUST use pinned image tags or SHA256 digests (never `:latest`), run as a non-root `USER`, define a `HEALTHCHECK`, and use multi-stage builds to exclude compilers/SDKs from runtime images.
2. Docker Compose and Kubernetes workloads MUST NOT set `privileged: true`, `network_mode: host`, or `pid: host`, and MUST enforce explicit CPU and memory resource limits.
