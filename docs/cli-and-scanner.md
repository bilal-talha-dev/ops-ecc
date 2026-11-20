# CLI Commands & DevSecOps Scanner Reference

## Command Reference

### 1. `ops-ecc init`
Initializes AI harness configurations (`rules/`, `agents/`, `commands/`, `skills/`, `settings.json`).
```bash
ops-ecc init [OPTIONS]
```
| Flag | Values | Description |
| :--- | :--- | :--- |
| `--yes`, `-y` | flag | Skip interactive prompts and apply defaults |
| `--preset` | `fullstack`, `devops`, `devsecops`, `all` | Filter ruleset profile (default: `all`) |
| `--tools` | `claude,gemini,cursor,universal` | Comma-separated AI tools to configure |
| `--scope` | `project`, `global` | Write to `./` or `~/` (default: `project`) |
| `--cloud` | `aws`, `azure`, `both` | Target cloud profile (default: `both`) |
| `--strict` | flag | Enable exit code `1`/`2` blocking on Critical/High findings |
| `--path` | `<DIR>` | Target repository directory (default: `.`) |

---

### 2. `ops-ecc doctor`
Checks 13 local DevOps/DevSecOps tools in parallel (`git`, `docker`, `terraform`, `tflint`, `checkov`, `cfn-lint`, `hadolint`, `trivy`, `gitleaks`, `aws`, `az`, `node`, `python3`) and verifies `.claude/settings.json` and `.gemini/settings.json` hook bindings.
```bash
ops-ecc doctor [--path <DIR>]
```

---

### 3. `ops-ecc audit [PATH]`
Runs the `.gitignore`-aware parallel Rust DevSecOps scanner (`rayon` + `ignore` + `hcl-rs`).
```bash
ops-ecc audit [PATH] [--strict] [--severity <critical|high|medium|low>] [--format <table|json|sarif>] [--output <FILE>]
```
Examples:
```bash
# Rich terminal table
ops-ecc audit .

# Fail CI pipeline if any High or Critical issue is found
ops-ecc audit . --strict --severity high

# Export SARIF 2.1.0 for GitHub Code Scanning Security Tab
ops-ecc audit . --format sarif --output ops-ecc.sarif
```

---

### 4. `ops-ecc hook <EVENT>`
Native `<5ms` Rust hooks designed for Claude Code, Gemini CLI, and Git:
- `ops-ecc hook post-edit <file_path>` — Validates `.tf`, `.yaml`/`.yml`, `Dockerfile`, `.json` via built-in HCL/YAML/JSON parsers + DevSecOps rules (and invokes `tflint`, `cfn-lint`, `hadolint` if installed).
- `ops-ecc hook pre-commit [--path .] [--strict]` — Scans staged git changes (or workspace) for High/Critical issues; exits `2` to block commits.
- `ops-ecc hook guard-cmd "<command>"` — Blocks destructive commands (`terraform destroy`, `aws s3 rb --force`, `DROP DATABASE`, `DROP TABLE`, `rm -rf /`, `kubectl delete namespace prod`, force-push to `main`/`master`/`prod`).

---

### 5. `ops-ecc scaffold <TEMPLATE> [--out <DIR>]`
Extracts production-hardened templates embedded in the binary (every scaffold passes `ops-ecc audit --strict` with zero High/Critical findings):

| Template Name | Contents |
| :--- | :--- |
| `terraform-aws` | Modular Multi-AZ VPC, ECS Fargate, Multi-AZ encrypted RDS Postgres (`prevent_destroy`), S3 + KMS + Public Access Block, SQS + DLQ, S3/DynamoDB remote state locking |
| `terraform-azure` | VNet, isolated subnets, NSG, HTTPS-only ZRS Storage Account, TLS 1.2 Azure SQL (`prevent_destroy`), `azurerm` remote state |
| `cfn-aws` | Hardened CloudFormation stack with KMS CMK, `DeletionPolicy: Retain`, least-privilege IAM, and isolated RDS SecurityGroup |
| `docker-prod` | Multi-stage non-root Node.js & Python Dockerfiles (`USER 10001:10001`, `HEALTHCHECK`), hardened `docker-compose.yml` (`read_only: true`, `cap_drop: [ALL]`, `mem_limit`), `.dockerignore` |
| `cicd-oidc` | GitHub Actions workflow with `id-token: write` AWS OIDC, 40-char SHA-pinned actions, `ops-ecc audit --format sarif`, Trivy scan, and ECS rolling deploy |
| `edge-proxy` | Hardened Nginx reverse proxy (TLS 1.2/1.3, HSTS, CSP, rate-limiting) + Cloudflare Zero Trust Tunnel (`cloudflared-config.yml`) |

---

### 6. `ops-ecc update [--path .] [--scope <project|global>]`
Updates `ops-ecc` rules, agents, skills, and commands while preserving any team-specific instructions placed between:
```markdown
<!-- USER_CUSTOM_START -->
Your custom rules here...
<!-- USER_CUSTOM_END -->
```

---

## Built-in Security Rule Catalog

| Rule ID | Category | Severity | Description |
| :--- | :--- | :--- | :--- |
| `SEC-001` | Secrets & Credentials | CRITICAL | Hardcoded AWS Access Key ID (`AKIA...` / `ASIA...`) |
| `SEC-002` | Secrets & Credentials | CRITICAL | Hardcoded AWS Secret Access Key |
| `SEC-003` | Secrets & Credentials | CRITICAL | Azure Storage or Service Bus connection string with embedded key |
| `SEC-004` | Secrets & Credentials | CRITICAL | Embedded PEM / OpenSSH Private Key |
| `SEC-005` | Secrets & Credentials | HIGH | Hardcoded signed JWT token |
| `SEC-006` | Secrets & Credentials | CRITICAL | Database/Cache URI (`postgres://`, `mongodb://`, `redis://`, `mysql://`) with embedded password |
| `SEC-007` | Secrets & Credentials | CRITICAL | Hardcoded GitHub (`ghp_`), OpenAI (`sk-proj-`), Anthropic (`sk-ant-`), or Gemini (`AIzaSy`) token |
| `DOCKER-001` | Dockerfile CIS | HIGH | `:latest` or unpinned base image tag |
| `DOCKER-002` | Dockerfile CIS | HIGH | `USER root` or missing non-root `USER` instruction |
| `DOCKER-003` | Dockerfile CIS | MEDIUM | Missing `HEALTHCHECK` instruction |
| `DOCKER-004` | Dockerfile CIS | CRITICAL | Secret passed via `ENV` or `ARG` |
| `DOCKER-005` | Dockerfile CIS | HIGH | `curl \| bash` / `wget \| sh` without `sha256sum` verification |
| `DOCKER-006` | Dockerfile CIS | MEDIUM | Exposed sensitive port (`22`, `3389`, `5432`, `3306`, `1433`, `27017`, `6379`) |
| `K8S-001` | Container / K8s | CRITICAL | `privileged: true` |
| `K8S-002` | Container / K8s | HIGH | `network_mode: host`, `pid: host`, `hostNetwork: true`, `hostPID: true` |
| `K8S-003` | Container / K8s | CRITICAL | Unprotected `/` or `/var/run/docker.sock` mount |
| `K8S-004` | Container / K8s | HIGH | Plaintext secret in container `environment` |
| `K8S-005` | Container / K8s | MEDIUM | Missing `mem_limit`/`cpus` (Compose) or `resources.limits` (K8s) |
| `IAC-000` | Terraform / IaC | HIGH | Invalid Terraform HCL syntax (`hcl-rs` parser failure) |
| `IAC-001` | Terraform / CloudFormation | CRITICAL | `0.0.0.0/0` or `::/0` ingress on SSH/RDP/DB ports |
| `IAC-002` | Terraform / CloudFormation | HIGH | Unencrypted storage or database (`storage_encrypted = false`) |
| `IAC-003` | Terraform / CloudFormation | CRITICAL | Public S3 ACL (`public-read`) or `publicly_accessible = true` database |
| `IAC-004` | Terraform / CloudFormation | HIGH | Wildcard IAM policy (`Action: "*"` or `Resource: "*"`) |
| `IAC-005` | Terraform / CloudFormation | MEDIUM | Stateful DB missing `prevent_destroy = true` / `DeletionPolicy: Retain` |
| `IAC-006` | Terraform / IaC | MEDIUM | `backend "s3"` missing state locking (`dynamodb_table` / `use_lockfile`) |
| `CICD-001` | CI/CD Supply Chain | HIGH | GitHub Action `uses:` pinned to mutable branch/tag instead of 40-char SHA |
| `CICD-002` | CI/CD Supply Chain | HIGH | Static `AWS_SECRET_ACCESS_KEY` / `AZURE_CREDENTIALS` without OIDC |
| `CICD-003` | CI/CD Supply Chain | CRITICAL | GitHub Actions script injection via `${{ github.event.* }}` in `run:` |
| `DEP-001` | Dependency Hygiene | LOW/MED | Missing lockfile (`package-lock.json`, `pnpm-lock.yaml`, `uv.lock`, `poetry.lock`) |
| `DEP-002` | Dependency Hygiene | HIGH | Wildcard (`*` or `latest`) dependency version in `package.json` |
