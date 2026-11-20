# `ops-ecc` — Blazing-Fast Full-Stack + Heavy DevOps & DevSecOps Agent Harness (Rust CLI)

`ops-ecc` is a single-binary, zero-stub Rust CLI tool and DevSecOps Agent Harness built for **Claude Code CLI (`.claude/`)**, **Gemini CLI (`.gemini/`)**, **Cursor (`.cursorrules`)**, and **Universal `AGENTS.md`**. It combines multi-agent Full-Stack/Cloud/DevSecOps rules, skills, and slash commands with a **sub-10ms parallel Rust static security scanner (`rayon` + `ignore` + `hcl-rs`)** and **sub-5ms native Rust pre-tool/post-edit/pre-commit hooks**.

## Key Capabilities

1. **`ops-ecc init`** — Interactive (`dialoguer`) or CI (`--yes`) harness initializer with stack auto-detection (`package.json`, `pyproject.toml`, `Cargo.toml`, `Dockerfile`, `*.tf`, `template.yaml`, `.github/workflows`) and automatic `.ops-ecc-backup-<YYYYMMDD-HHMMSS>/` snapshots.
2. **`ops-ecc doctor`** — Parallel readiness check across 13 DevOps/DevSecOps CLIs (`git`, `docker`, `terraform`, `tflint`, `checkov`, `cfn-lint`, `hadolint`, `trivy`, `gitleaks`, `aws`, `az`, `node`, `python3`) and `.claude`/`.gemini` hook verification.
3. **`ops-ecc audit [PATH]`** — Standalone parallel DevSecOps scanner covering:
   - **Secrets & Credentials**: AWS keys (`AKIA...`), Secret Keys, Azure Storage/ServiceBus strings, PEM private keys, JWTs, Postgres/Mongo/Redis URIs with embedded passwords, GitHub/OpenAI/Anthropic/Gemini tokens.
   - **Dockerfile CIS Benchmarks**: `:latest` or unpinned tags, `USER root` or missing `USER`, missing `HEALTHCHECK`, secrets in `ENV`/`ARG`, `curl | bash` without `sha256sum`, exposed sensitive ports (`22`, `3389`, `5432`, `6379`).
   - **Docker Compose & Kubernetes**: `privileged: true`, `network_mode: host`, `pid: host`, missing `mem_limit`/`cpus` or `resources.limits`, `/var/run/docker.sock` or `/` host mounts, plaintext environment secrets.
   - **Terraform (`.tf`) & CloudFormation (`.yaml`/`.json`)**: `hcl-rs` syntax validation, `0.0.0.0/0` or `::/0` ingress on SSH/RDP/DB ports, unencrypted EBS/RDS/S3/Azure storage, public S3 ACLs, wildcard IAM policies (`*`), missing `prevent_destroy` / `DeletionPolicy: Retain`, missing Terraform S3 state locking.
   - **CI/CD Supply Chain**: GitHub Actions pinned to mutable tags (`@main`, `@v4`) instead of 40-char commit SHAs, static `AWS_SECRET_ACCESS_KEY` instead of OIDC (`id-token: write`), and `${{ github.event.* }}` script injection.
   - **Dependency & Lockfile Hygiene**: Missing lockfiles and wildcard (`*`) dependency versions.
   - **Outputs**: Rich terminal tables (`--format table`), JSON (`--format json`), and **SARIF 2.1.0** (`--format sarif --output ops-ecc.sarif`) for GitHub Code Scanning.
4. **`ops-ecc hook <post-edit|pre-commit|guard-cmd>`** — Native Rust hooks blocking destructive commands (`terraform destroy`, `aws s3 rb --force`, `DROP DATABASE`, `rm -rf /`, `kubectl delete namespace prod`) and catching IaC/secret defects in `<5ms`.
5. **`ops-ecc scaffold <template>`** — Extracts production-hardened templates embedded in the binary (`terraform-aws`, `terraform-azure`, `cfn-aws`, `docker-prod`, `cicd-oidc`, `edge-proxy`).
6. **`ops-ecc update`** — Updates rules, agents, skills, and commands while preserving `<!-- USER_CUSTOM_START -->` ... `<!-- USER_CUSTOM_END -->` blocks.

## Quick Start

```bash
# Build & install from source
cargo build --release
cargo install --path .

# Run full unit & integration test suite
cargo test

# Initialize harness in any repository
ops-ecc init --yes --preset all --cloud both --strict

# Audit a repository and emit SARIF 2.1.0 report
ops-ecc audit . --strict --format sarif --output ops-ecc.sarif
```
