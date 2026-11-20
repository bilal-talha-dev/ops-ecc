use crate::cli::Severity;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub rule_id: String,
    pub category: String,
    pub severity: Severity,
    pub file: String,
    pub line: usize,
    pub message: String,
    pub remediation: String,
    pub snippet: String,
}

pub struct CompiledRule {
    pub id: &'static str,
    pub category: &'static str,
    pub severity: Severity,
    pub regex: Regex,
    pub message: &'static str,
    pub remediation: &'static str,
}

// ==============================================================================
// 1. SECRETS & CREDENTIAL PATTERNS
// ==============================================================================
pub static SECRET_RULES: Lazy<Vec<CompiledRule>> = Lazy::new(|| {
    vec![
        CompiledRule {
            id: "SEC-001",
            category: "Secrets & Credentials",
            severity: Severity::Critical,
            regex: Regex::new(r"\b(AKIA|ASIA)[0-9A-Z]{16}\b").unwrap(),
            message: "Hardcoded AWS Access Key ID detected",
            remediation: "Remove the AWS key immediately, rotate it in AWS IAM, and switch to IAM Roles / OIDC federation or AWS Secrets Manager.",
        },
        CompiledRule {
            id: "SEC-002",
            category: "Secrets & Credentials",
            severity: Severity::Critical,
            regex: Regex::new(r#"(?i)(aws_secret_access_key|aws_secret_key|secret_access_key)\s*[:=]\s*["'][A-Za-z0-9/+=]{40}["']"#).unwrap(),
            message: "Hardcoded AWS Secret Access Key detected",
            remediation: "Revoke the exposed secret key in AWS IAM and inject credentials dynamically via OIDC or environment secret stores.",
        },
        CompiledRule {
            id: "SEC-003",
            category: "Secrets & Credentials",
            severity: Severity::Critical,
            regex: Regex::new(r"(?i)(DefaultEndpointsProtocol=https?;AccountName=[^;\s]+;AccountKey=[A-Za-z0-9+/=]{20,}|Endpoint=sb://[^;\s]+;SharedAccessKeyName=[^;\s]+;SharedAccessKey=[A-Za-z0-9+/=]{16,})").unwrap(),
            message: "Hardcoded Azure Storage or Service Bus connection string with embedded key detected",
            remediation: "Use Azure Managed Identity (`DefaultAzureCredential`) or reference secrets from Azure Key Vault.",
        },
        CompiledRule {
            id: "SEC-004",
            category: "Secrets & Credentials",
            severity: Severity::Critical,
            regex: Regex::new(r"-----BEGIN (RSA|EC|OPENSSH|DSA|PRIVATE) (PRIVATE )?KEY-----").unwrap(),
            message: "PEM / OpenSSH Private Key embedded in source file",
            remediation: "Remove the private key file from the repository, purge git history, and load keys from a mounted secret volume or KMS.",
        },
        CompiledRule {
            id: "SEC-005",
            category: "Secrets & Credentials",
            severity: Severity::High,
            regex: Regex::new(r"\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b").unwrap(),
            message: "Hardcoded signed JWT token detected",
            remediation: "Do not commit live or long-lived JWT tokens; generate short-lived tokens dynamically during tests or runtime.",
        },
        CompiledRule {
            id: "SEC-006",
            category: "Secrets & Credentials",
            severity: Severity::Critical,
            regex: Regex::new(r#"(?i)\b(postgres|postgresql|mongodb|mongodb\+srv|redis|rediss|mysql)://[^:\s"']+:[^@\s"']+@[A-Za-z0-9._-]+"#).unwrap(),
            message: "Database or cache connection URI with hardcoded password detected",
            remediation: "Inject `DATABASE_URL` / `REDIS_URL` from runtime secret management or use IAM database authentication.",
        },
        CompiledRule {
            id: "SEC-007",
            category: "Secrets & Credentials",
            severity: Severity::Critical,
            regex: Regex::new(r"\b(ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{50,}|sk-proj-[A-Za-z0-9_-]{20,}|sk-ant-api03-[A-Za-z0-9_-]{20,}|AIzaSy[A-Za-z0-9_-]{33})\b").unwrap(),
            message: "Hardcoded API Token (GitHub / OpenAI / Anthropic / Gemini) detected",
            remediation: "Revoke the API token immediately in the provider console and load it from an unversioned secret store.",
        },
    ]
});

// ==============================================================================
// 2. DOCKERFILE HARDENING & CIS BENCHMARK SCANNER
// ==============================================================================
static RE_FROM_LATEST: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*FROM\s+([^\s:@]+:latest|\S+)\s*(AS\s+\S+)?\s*$").unwrap());
static RE_USER_ROOT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*USER\s+(root|0(:0)?)\s*$").unwrap());
static RE_USER_ANY: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^\s*USER\s+\S+").unwrap());
static RE_HEALTHCHECK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*HEALTHCHECK\b").unwrap());
static RE_ENV_ARG_SECRET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^\s*(ENV|ARG)\s+[A-Z0-9_]*(SECRET|PASSWORD|PASSWD|PRIVATE_KEY|API_KEY|ACCESS_KEY|TOKEN)[A-Z0-9_]*\s*=").unwrap()
});
static RE_CURL_PIPE_SH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(curl|wget)\b[^
|]*\|\s*(sudo\s+)?(sh|bash|zsh)\b").unwrap()
});
static RE_EXPOSE_SENSITIVE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^\s*EXPOSE\s+.*\b(22|3389|5432|3306|1433|27017|6379)\b").unwrap()
});

pub fn scan_dockerfile(rel_path: &str, content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut has_user = false;
    let mut has_healthcheck = false;

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(caps) = RE_FROM_LATEST.captures(trimmed) {
            let image = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if !image.eq_ignore_ascii_case("scratch")
                && (image.ends_with(":latest") || (!image.contains(':') && !image.contains('@')))
            {
                findings.push(Finding {
                    rule_id: "DOCKER-001".to_string(),
                    category: "Dockerfile CIS".to_string(),
                    severity: Severity::High,
                    file: rel_path.to_string(),
                    line: line_no,
                    message: format!(
                        "Base image `{}` uses `:latest` or omits an immutable version/SHA256 tag",
                        image
                    ),
                    remediation:
                        "Pin the base image to an explicit semantic version or `@sha256:...` digest."
                            .to_string(),
                    snippet: trimmed.to_string(),
                });
            }
        }

        if RE_USER_ANY.is_match(trimmed) {
            has_user = true;
            if RE_USER_ROOT.is_match(trimmed) {
                findings.push(Finding {
                    rule_id: "DOCKER-002".to_string(),
                    category: "Dockerfile CIS".to_string(),
                    severity: Severity::High,
                    file: rel_path.to_string(),
                    line: line_no,
                    message: "Container explicitly switches to `USER root` (CIS Docker Benchmark 4.1)".to_string(),
                    remediation: "Create and run as a non-root user (e.g., `USER 10001:10001`).".to_string(),
                    snippet: trimmed.to_string(),
                });
            }
        }

        if RE_HEALTHCHECK.is_match(trimmed) && !trimmed.to_uppercase().contains("NONE") {
            has_healthcheck = true;
        }

        if RE_ENV_ARG_SECRET.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "DOCKER-004".to_string(),
                category: "Dockerfile CIS".to_string(),
                severity: Severity::Critical,
                file: rel_path.to_string(),
                line: line_no,
                message: "Sensitive secret passed via Dockerfile `ENV` or `ARG` instruction (persisted in image layers)".to_string(),
                remediation: "Mount secrets at runtime or use BuildKit `--mount=type=secret` instead of `ENV`/`ARG`.".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_CURL_PIPE_SH.is_match(trimmed) && !trimmed.contains("sha256sum") {
            findings.push(Finding {
                rule_id: "DOCKER-005".to_string(),
                category: "Dockerfile CIS".to_string(),
                severity: Severity::High,
                file: rel_path.to_string(),
                line: line_no,
                message: "Remote script piped directly to shell (`curl | bash`) without SHA256 checksum verification".to_string(),
                remediation: "Download the installer, verify its cryptographic checksum with `sha256sum -c`, and then execute it.".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_EXPOSE_SENSITIVE.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "DOCKER-006".to_string(),
                category: "Dockerfile CIS".to_string(),
                severity: Severity::Medium,
                file: rel_path.to_string(),
                line: line_no,
                message: "Dockerfile exposes sensitive administrative or database port (`22`, `3389`, `5432`, `6379`)".to_string(),
                remediation: "Do not expose SSH/RDP in containers; keep database ports restricted to internal container networks.".to_string(),
                snippet: trimmed.to_string(),
            });
        }
    }

    if !has_user {
        findings.push(Finding {
            rule_id: "DOCKER-002".to_string(),
            category: "Dockerfile CIS".to_string(),
            severity: Severity::High,
            file: rel_path.to_string(),
            line: 1,
            message: "Dockerfile is missing a non-root `USER` instruction (defaults to `root`)"
                .to_string(),
            remediation:
                "Add a dedicated unprivileged user (`USER 10001:10001`) before `CMD`/`ENTRYPOINT`."
                    .to_string(),
            snippet: "FROM ... (no USER instruction found)".to_string(),
        });
    }

    if !has_healthcheck {
        findings.push(Finding {
            rule_id: "DOCKER-003".to_string(),
            category: "Dockerfile CIS".to_string(),
            severity: Severity::Medium,
            file: rel_path.to_string(),
            line: 1,
            message: "Dockerfile is missing a `HEALTHCHECK` instruction (CIS Docker Benchmark 4.6)"
                .to_string(),
            remediation:
                "Add `HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD ...` to enable orchestrator health probes."
                    .to_string(),
            snippet: "Missing HEALTHCHECK".to_string(),
        });
    }

    findings
}

// ==============================================================================
// 3. DOCKER COMPOSE & KUBERNETES MANIFEST SCANNER
// ==============================================================================
static RE_PRIVILEGED_TRUE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bprivileged\s*:\s*true\b").unwrap());
static RE_HOST_NET_PID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\b(network_mode\s*:\s*["']?host["']?|pid\s*:\s*["']?host["']?|hostNetwork\s*:\s*true|hostPID\s*:\s*true|hostIPC\s*:\s*true)"#).unwrap()
});
static RE_DOCKER_SOCK_MOUNT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(/var/run/docker\.sock|\s-\s*["']?/:[^"'\s]+)"#).unwrap()
});
static RE_COMPOSE_PLAIN_SECRET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)^\s*[-a-z0-9_]*(PASSWORD|SECRET|API_KEY|PRIVATE_KEY|ACCESS_KEY)[-a-z0-9_]*\s*[:=]\s*["']?[A-Za-z0-9!@#$%^&*()_+-=]{6,}["']?\s*$"#).unwrap()
});

pub fn scan_compose_or_k8s(rel_path: &str, content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let is_compose = rel_path.contains("docker-compose")
        || rel_path.ends_with("compose.yml")
        || rel_path.ends_with("compose.yaml");
    let is_k8s = content.contains("apiVersion:") && content.contains("kind:");

    if !is_compose && !is_k8s {
        return findings;
    }

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if RE_PRIVILEGED_TRUE.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "K8S-001".to_string(),
                category: "Container / K8s Security".to_string(),
                severity: Severity::Critical,
                file: rel_path.to_string(),
                line: line_no,
                message: "Container workload runs with `privileged: true` (full host kernel capabilities)".to_string(),
                remediation: "Set `privileged: false`, drop `ALL` capabilities (`cap_drop: [ALL]`), and enable `no-new-privileges:true`.".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_HOST_NET_PID.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "K8S-002".to_string(),
                category: "Container / K8s Security".to_string(),
                severity: Severity::High,
                file: rel_path.to_string(),
                line: line_no,
                message: "Container workload shares host network or PID namespace (`network_mode: host` / `hostNetwork: true` / `hostPID: true`)".to_string(),
                remediation: "Isolate workloads in bridge/overlay networks with dedicated PID namespaces.".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_DOCKER_SOCK_MOUNT.is_match(trimmed) && !trimmed.ends_with(":ro") {
            findings.push(Finding {
                rule_id: "K8S-003".to_string(),
                category: "Container / K8s Security".to_string(),
                severity: Severity::Critical,
                file: rel_path.to_string(),
                line: line_no,
                message: "Sensitive host path (`/` or `/var/run/docker.sock`) mounted into container without read-only restriction".to_string(),
                remediation: "Remove `/var/run/docker.sock` or host root mounts to prevent trivial container escape.".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_COMPOSE_PLAIN_SECRET.is_match(trimmed)
            && !trimmed.contains("${")
            && !trimmed.contains("_FILE")
            && !trimmed.contains("secretKeyRef")
        {
            findings.push(Finding {
                rule_id: "K8S-004".to_string(),
                category: "Container / K8s Security".to_string(),
                severity: Severity::High,
                file: rel_path.to_string(),
                line: line_no,
                message: "Plaintext secret value hardcoded in container environment configuration".to_string(),
                remediation: "Use Docker Compose `secrets:` / `.env` variable interpolation or Kubernetes `secretKeyRef`.".to_string(),
                snippet: trimmed.to_string(),
            });
        }
    }

    if is_compose
        && content.contains("services:")
        && !content.contains("mem_limit")
        && !content.contains("memory:")
    {
        findings.push(Finding {
            rule_id: "K8S-005".to_string(),
            category: "Container / K8s Security".to_string(),
            severity: Severity::Medium,
            file: rel_path.to_string(),
            line: 1,
            message: "Docker Compose services lack explicit CPU/memory resource caps (`mem_limit` / `cpus`)".to_string(),
            remediation: "Define `mem_limit: 512m` and `cpus: 1.0` (or `deploy.resources.limits`) to prevent host OOM starvation.".to_string(),
            snippet: "services: (missing mem_limit / cpus)".to_string(),
        });
    } else if is_k8s
        && (content.contains("kind: Deployment")
            || content.contains("kind: StatefulSet")
            || content.contains("kind: DaemonSet")
            || content.contains("kind: Pod"))
        && !content.contains("limits:")
    {
        findings.push(Finding {
            rule_id: "K8S-005".to_string(),
            category: "Container / K8s Security".to_string(),
            severity: Severity::Medium,
            file: rel_path.to_string(),
            line: 1,
            message: "Kubernetes workload specification omits `resources.limits` (CPU/Memory)".to_string(),
            remediation: "Specify `resources.requests` and `resources.limits` for `cpu` and `memory` on every container.".to_string(),
            snippet: "kind: Deployment/Pod (missing resources.limits)".to_string(),
        });
    }

    findings
}

// ==============================================================================
// 4. TERRAFORM (.tf) & CLOUDFORMATION (.yaml/.json) DEVSECOPS SCANNER
// ==============================================================================
static RE_OPEN_CIDR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"("0\.0\.0\.0/0"|'0\.0\.0\.0/0'|0\.0\.0\.0/0|"::/0"|::/0)"#).unwrap());
static RE_SENSITIVE_PORT_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\b(from_port|to_port|destination_port_range|FromPort|ToPort)\s*[:=]\s*['"]?(22|3389|5432|3306|1433|27017|6379)['"]?\b"#).unwrap()
});
static RE_UNENCRYPTED_FLAG: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(encrypted|storage_encrypted|StorageEncrypted|https_traffic_only_enabled|enable_https_traffic_only)\s*[:=]\s*false\b").unwrap()
});
static RE_PUBLIC_S3_ACL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\b(acl\s*=\s*["']public-read(-write)?["']|AccessControl\s*:\s*["']?PublicRead(Write)?["']?|block_public_acls\s*=\s*false|BlockPublicAcls\s*:\s*false|publicly_accessible\s*=\s*true|PubliclyAccessible\s*:\s*true)"#).unwrap()
});
static RE_IAM_WILDCARD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)("Action"\s*:\s*"\*"|"Resource"\s*:\s*"\*"|\bAction\s*:\s*["']?\*["']?|\bResource\s*:\s*["']?\*["']?|\bactions\s*=\s*\[\s*"\*"\s*\]|\bresources\s*=\s*\[\s*"\*"\s*\])"#).unwrap()
});

pub fn scan_terraform(rel_path: &str, content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Parse HCL using `hcl-rs` to verify syntactic validity
    if let Err(err) = hcl::parse(content) {
        findings.push(Finding {
            rule_id: "IAC-000".to_string(),
            category: "Terraform / IaC".to_string(),
            severity: Severity::High,
            file: rel_path.to_string(),
            line: 1,
            message: format!("Invalid Terraform HCL syntax: {}", err),
            remediation: "Fix the HCL syntax error and run `ops-ecc hook post-edit` to verify.".to_string(),
            snippet: err.to_string(),
        });
    }

    static RE_STORAGE_ENC_TRUE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"storage_encrypted\s*=\s*true").unwrap());
    static RE_PREVENT_DESTROY_TRUE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"prevent_destroy\s*=\s*true").unwrap());

    let mut in_egress_block = false;
    let mut egress_depth: i32 = 0;
    let mut has_open_ingress_cidr = false;
    let mut open_cidr_line = 1usize;

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("egress") && trimmed.contains('{') {
            in_egress_block = true;
            egress_depth = 1;
            continue;
        }
        if in_egress_block {
            egress_depth += trimmed.matches('{').count() as i32;
            egress_depth -= trimmed.matches('}').count() as i32;
            if egress_depth <= 0 {
                in_egress_block = false;
                egress_depth = 0;
            }
            continue;
        }
        if RE_OPEN_CIDR.is_match(trimmed) {
            has_open_ingress_cidr = true;
            open_cidr_line = idx + 1;
        }
    }

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        if RE_OPEN_CIDR.is_match(trimmed) {
            open_cidr_line = line_no;
        }

        if has_open_ingress_cidr && RE_SENSITIVE_PORT_LINE.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "IAC-001".to_string(),
                category: "Terraform / IaC".to_string(),
                severity: Severity::Critical,
                file: rel_path.to_string(),
                line: line_no.max(open_cidr_line),
                message: "Security Group / NSG exposes SSH (22), RDP (3389), or Database port (5432/3306/1433/27017/6379) to `0.0.0.0/0` or `::/0`".to_string(),
                remediation: "Restrict ingress `cidr_blocks` to private VPC CIDRs or upstream security group IDs; use SSM Session Manager or Cloudflare Zero Trust Tunnels for access.".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_UNENCRYPTED_FLAG.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "IAC-002".to_string(),
                category: "Terraform / IaC".to_string(),
                severity: Severity::High,
                file: rel_path.to_string(),
                line: line_no,
                message: "Cloud storage or database explicitly disables encryption at rest or HTTPS transit".to_string(),
                remediation: "Set `encrypted = true` / `storage_encrypted = true` with a customer-managed KMS key (`kms_key_id`).".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_PUBLIC_S3_ACL.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "IAC-003".to_string(),
                category: "Terraform / IaC".to_string(),
                severity: Severity::Critical,
                file: rel_path.to_string(),
                line: line_no,
                message: "Public storage ACL (`public-read` / `block_public_acls = false`) or `publicly_accessible = true` database detected".to_string(),
                remediation: "Enforce `aws_s3_bucket_public_access_block` with all four flags `true` and set `publicly_accessible = false` on RDS.".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_IAM_WILDCARD.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "IAC-004".to_string(),
                category: "Terraform / IaC".to_string(),
                severity: Severity::High,
                file: rel_path.to_string(),
                line: line_no,
                message: r#"Overly permissive wildcard IAM policy (`Action = "*"` or `Resource = "*"`) violates least-privilege"#.to_string(),
                remediation: "Scope IAM policy actions to specific service operations and restrict `Resource` to explicit ARNs.".to_string(),
                snippet: trimmed.to_string(),
            });
        }
    }

    // Check for stateful database resources missing `prevent_destroy = true` or `storage_encrypted = true`
    if (content.contains("\"aws_db_instance\"")
        || content.contains("\"aws_rds_cluster\"")
        || content.contains("\"azurerm_mssql_database\""))
        && !RE_PREVENT_DESTROY_TRUE.is_match(content)
    {
        findings.push(Finding {
            rule_id: "IAC-005".to_string(),
            category: "Terraform / IaC".to_string(),
            severity: Severity::Medium,
            file: rel_path.to_string(),
            line: 1,
            message: "Stateful database resource omits `lifecycle { prevent_destroy = true }` safeguard".to_string(),
            remediation: "Add `lifecycle { prevent_destroy = true }` and `deletion_protection = true` to production database blocks.".to_string(),
            snippet: r#"resource "aws_db_instance" / "azurerm_mssql_database" (missing prevent_destroy)"#.to_string(),
        });
    }

    if (content.contains("\"aws_db_instance\"") || content.contains("\"aws_rds_cluster\""))
        && !RE_STORAGE_ENC_TRUE.is_match(content)
    {
        findings.push(Finding {
            rule_id: "IAC-002".to_string(),
            category: "Terraform / IaC".to_string(),
            severity: Severity::High,
            file: rel_path.to_string(),
            line: 1,
            message: "RDS database resource does not enable `storage_encrypted = true`".to_string(),
            remediation: "Set `storage_encrypted = true` and attach a KMS key via `kms_key_id`.".to_string(),
            snippet: r#"resource "aws_db_instance" (missing storage_encrypted = true)"#.to_string(),
        });
    }

    // Check for Terraform S3 backend missing state locking (`dynamodb_table` or `use_lockfile`)
    if content.contains("backend \"s3\"")
        && !content.contains("dynamodb_table")
        && !content.contains("use_lockfile")
    {
        findings.push(Finding {
            rule_id: "IAC-006".to_string(),
            category: "Terraform / IaC".to_string(),
            severity: Severity::Medium,
            file: rel_path.to_string(),
            line: 1,
            message: r#"Terraform `backend "s3"` configuration is missing state locking (`dynamodb_table` or `use_lockfile = true`)"#.to_string(),
            remediation: r#"Configure `dynamodb_table = "...-tfstate-locks"` and `encrypt = true` inside `backend "s3"`."#.to_string(),
            snippet: r#"backend "s3" { ... }"#.to_string(),
        });
    }

    findings
}

pub fn scan_cloudformation(rel_path: &str, content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let is_cfn = content.contains("AWSTemplateFormatVersion")
        || content.contains("AWS::Serverless")
        || content.contains("AWS::RDS::")
        || content.contains("AWS::S3::Bucket")
        || content.contains("AWS::EC2::SecurityGroup")
        || content.contains("AWS::IAM::");

    if !is_cfn {
        return findings;
    }

    let has_open_cidr = RE_OPEN_CIDR.is_match(content);

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if has_open_cidr && RE_SENSITIVE_PORT_LINE.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "IAC-001".to_string(),
                category: "CloudFormation / IaC".to_string(),
                severity: Severity::Critical,
                file: rel_path.to_string(),
                line: line_no,
                message: "CloudFormation SecurityGroup allows `0.0.0.0/0` or `::/0` ingress on sensitive port (22/3389/5432/3306/6379)".to_string(),
                remediation: "Replace `CidrIp: 0.0.0.0/0` with `SourceSecurityGroupId` or private subnet CIDR.".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_UNENCRYPTED_FLAG.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "IAC-002".to_string(),
                category: "CloudFormation / IaC".to_string(),
                severity: Severity::High,
                file: rel_path.to_string(),
                line: line_no,
                message: "CloudFormation resource explicitly sets `StorageEncrypted: false` or disables encryption".to_string(),
                remediation: "Set `StorageEncrypted: true` and specify `KmsKeyId`.".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_PUBLIC_S3_ACL.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "IAC-003".to_string(),
                category: "CloudFormation / IaC".to_string(),
                severity: Severity::Critical,
                file: rel_path.to_string(),
                line: line_no,
                message: "CloudFormation S3 Bucket or RDS instance allows public access (`PublicRead` / `PubliclyAccessible: true`)".to_string(),
                remediation: "Configure `PublicAccessBlockConfiguration` with all four flags `true` and set `PubliclyAccessible: false`.".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_IAM_WILDCARD.is_match(trimmed) {
            findings.push(Finding {
                rule_id: "IAC-004".to_string(),
                category: "CloudFormation / IaC".to_string(),
                severity: Severity::High,
                file: rel_path.to_string(),
                line: line_no,
                message: "CloudFormation IAM Policy contains wildcard `Action: '*'` or `Resource: '*'`".to_string(),
                remediation: "Scope IAM `Action` and `Resource` to least-privilege ARNs using `!GetAtt` or `!Sub`.".to_string(),
                snippet: trimmed.to_string(),
            });
        }
    }

    if (content.contains("AWS::RDS::DBInstance")
        || content.contains("AWS::RDS::DBCluster")
        || content.contains("AWS::DynamoDB::Table"))
        && !content.contains("DeletionPolicy: Retain")
        && !content.contains("\"DeletionPolicy\": \"Retain\"")
    {
        findings.push(Finding {
            rule_id: "IAC-005".to_string(),
            category: "CloudFormation / IaC".to_string(),
            severity: Severity::Medium,
            file: rel_path.to_string(),
            line: 1,
            message: "Stateful CloudFormation database resource omits `DeletionPolicy: Retain`".to_string(),
            remediation: "Add `DeletionPolicy: Retain` and `UpdateReplacePolicy: Retain` to stateful database resources.".to_string(),
            snippet: "AWS::RDS::DBInstance / AWS::DynamoDB::Table (missing DeletionPolicy: Retain)".to_string(),
        });
    }

    findings
}

// ==============================================================================
// 5. CI/CD SUPPLY-CHAIN SECURITY SCANNER (GitHub Actions / Pipelines)
// ==============================================================================
static RE_GHA_USES: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*-?\s*uses:\s*([^\s#]+)").unwrap());
static RE_GHA_SHA40: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"@[0-9a-fA-F]{40}$").unwrap());
static RE_GHA_LONG_LIVED_CRED: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(AWS_SECRET_ACCESS_KEY|AZURE_CREDENTIALS|AZURE_CLIENT_SECRET)\b").unwrap()
});
static RE_GHA_SCRIPT_INJECTION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\$\{\{\s*github\.(event\.(issue|pull_request|comment|review|discussion|commits|head_commit)\.[a-zA-Z0-9_.\[\]]+|head_ref)\s*\}\}").unwrap()
});

pub fn scan_github_workflow(rel_path: &str, content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let has_oidc = content.contains("id-token: write");

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(caps) = RE_GHA_USES.captures(trimmed) {
            let action_ref = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if !action_ref.starts_with("./")
                && !action_ref.starts_with("docker://")
                && !RE_GHA_SHA40.is_match(action_ref)
            {
                findings.push(Finding {
                    rule_id: "CICD-001".to_string(),
                    category: "CI/CD Supply Chain".to_string(),
                    severity: Severity::High,
                    file: rel_path.to_string(),
                    line: line_no,
                    message: format!(
                        "GitHub Action `{}` is pinned to a mutable branch/tag instead of an immutable 40-char commit SHA",
                        action_ref
                    ),
                    remediation: "Pin external `uses:` actions to a full 40-character commit SHA (e.g., `actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683`).".to_string(),
                    snippet: trimmed.to_string(),
                });
            }
        }

        if RE_GHA_LONG_LIVED_CRED.is_match(trimmed) && !has_oidc {
            findings.push(Finding {
                rule_id: "CICD-002".to_string(),
                category: "CI/CD Supply Chain".to_string(),
                severity: Severity::High,
                file: rel_path.to_string(),
                line: line_no,
                message: "CI/CD workflow uses long-lived static cloud credentials (`AWS_SECRET_ACCESS_KEY` / `AZURE_CREDENTIALS`) without OIDC federation".to_string(),
                remediation: "Configure `permissions: id-token: write` and authenticate via short-lived OIDC tokens.".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        if RE_GHA_SCRIPT_INJECTION.is_match(trimmed) && !trimmed.starts_with("PR_TITLE:") && !trimmed.starts_with("ISSUE_TITLE:") {
            findings.push(Finding {
                rule_id: "CICD-003".to_string(),
                category: "CI/CD Supply Chain".to_string(),
                severity: Severity::Critical,
                file: rel_path.to_string(),
                line: line_no,
                message: "Potential GitHub Actions script injection via unescaped `${{ github.event.* }}` expression".to_string(),
                remediation: r#"Bind the untrusted expression to a step `env:` variable first and reference it as a double-quoted shell variable (`"$UNTRUSTED_VAR"`)."#.to_string(),
                snippet: trimmed.to_string(),
            });
        }
    }

    findings
}

// ==============================================================================
// 6. DEPENDENCY & LOCKFILE HYGIENE SCANNER
// ==============================================================================
static RE_WILDCARD_DEP: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#""[^"]+"\s*:\s*"(\*|latest)""#).unwrap());

pub fn scan_lockfile_hygiene(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();

    let pkg_json = root.join("package.json");
    if pkg_json.exists() {
        let has_lock = root.join("package-lock.json").exists()
            || root.join("pnpm-lock.yaml").exists()
            || root.join("yarn.lock").exists()
            || root.join("bun.lockb").exists();
        if !has_lock {
            findings.push(Finding {
                rule_id: "DEP-001".to_string(),
                category: "Dependency Hygiene".to_string(),
                severity: Severity::Medium,
                file: "package.json".to_string(),
                line: 1,
                message: "Node.js project (`package.json`) is missing a committed lockfile (`package-lock.json` / `pnpm-lock.yaml` / `yarn.lock`)".to_string(),
                remediation: "Commit a deterministic lockfile and use `npm ci` or `pnpm install --frozen-lockfile` in CI/CD.".to_string(),
                snippet: "package.json (no lockfile found)".to_string(),
            });
        }

        if let Ok(content) = std::fs::read_to_string(&pkg_json) {
            for (idx, line) in content.lines().enumerate() {
                if RE_WILDCARD_DEP.is_match(line) {
                    findings.push(Finding {
                        rule_id: "DEP-002".to_string(),
                        category: "Dependency Hygiene".to_string(),
                        severity: Severity::High,
                        file: "package.json".to_string(),
                        line: idx + 1,
                        message: "Unpinned wildcard (`*` or `latest`) dependency version in `package.json` exposes builds to supply-chain hijacking".to_string(),
                        remediation: "Pin dependency versions explicitly and commit the lockfile.".to_string(),
                        snippet: line.trim().to_string(),
                    });
                }
            }
        }
    }

    let pyproject = root.join("pyproject.toml");
    if pyproject.exists()
        && !root.join("poetry.lock").exists()
        && !root.join("uv.lock").exists()
        && !root.join("requirements.lock").exists()
    {
        findings.push(Finding {
            rule_id: "DEP-001".to_string(),
            category: "Dependency Hygiene".to_string(),
            severity: Severity::Low,
            file: "pyproject.toml".to_string(),
            line: 1,
            message: "Python project (`pyproject.toml`) is missing `uv.lock` or `poetry.lock`".to_string(),
            remediation: "Generate and commit `uv.lock` or `poetry.lock` for reproducible dependency resolution.".to_string(),
            snippet: "pyproject.toml (no lockfile found)".to_string(),
        });
    }

    findings
}
