use crate::cli::{HookEvent, Severity};
use crate::scanner::{scan_path, scan_single_file};
use anyhow::Result;
use once_cell::sync::Lazy;
use owo_colors::OwoColorize;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookOutcome {
    pub allowed: bool,
    pub exit_code: i32,
    pub reason: String,
}

static DESTRUCTIVE_CMD_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        (
            Regex::new(r"(?i)\bterraform\s+destroy\b").unwrap(),
            "Blocked destructive `terraform destroy` command. Run targeted `terraform plan -destroy` with human approval outside the AI agent loop.",
        ),
        (
            Regex::new(r"(?i)\baws\s+s3\s+(rb|rm)\b.*--force").unwrap(),
            "Blocked destructive `aws s3 rb/rm --force` command.",
        ),
        (
            Regex::new(r"(?i)\bDROP\s+(DATABASE|TABLE|SCHEMA)\b").unwrap(),
            "Blocked destructive SQL `DROP DATABASE / TABLE / SCHEMA` statement.",
        ),
        (
            Regex::new(r"(?i)\brm\s+-[a-zA-Z]*r[a-zA-Z]*f[a-zA-Z]*\s+(/|/\*|\$HOME|~)(\s|$)").unwrap(),
            "Blocked catastrophic `rm -rf /` or home directory deletion command.",
        ),
        (
            Regex::new(r"(?i)\bkubectl\s+delete\s+(namespace|ns)\s+(prod|production)\b").unwrap(),
            "Blocked destructive `kubectl delete namespace prod` command.",
        ),
        (
            Regex::new(r"(?i)\bgit\s+push\b.*(--force|-f)\b.*\b(main|master|prod)\b").unwrap(),
            "Blocked force-push to protected branch (`main`/`master`/`prod`).",
        ),
    ]
});

pub fn execute_hook(event: &HookEvent) -> Result<HookOutcome> {
    match event {
        HookEvent::GuardCmd { command } => Ok(run_guard_cmd(command)),
        HookEvent::PostEdit { file_path } => run_post_edit(file_path),
        HookEvent::PreCommit { path, strict } => run_pre_commit(path, *strict),
    }
}

/// Inspect a shell command before execution and block destructive operations in <1ms.
pub fn run_guard_cmd(command: &str) -> HookOutcome {
    for (pattern, message) in DESTRUCTIVE_CMD_PATTERNS.iter() {
        if pattern.is_match(command) {
            eprintln!(
                "{} {} (Command: `{}`)",
                "[ops-ecc guard-cmd BLOCKED]".bold().red(),
                message,
                command
            );
            return HookOutcome {
                allowed: false,
                exit_code: 2, // Claude Code / Gemini CLI hook block exit code
                reason: (*message).to_string(),
            };
        }
    }

    HookOutcome {
        allowed: true,
        exit_code: 0,
        reason: "Command passed ops-ecc guard-cmd inspection.".to_string(),
    }
}

/// Validate edited `.tf`, `.yaml`/`.yml`, `Dockerfile`, or `.json` files via built-in
/// HCL/YAML/JSON parsers + built-in DevSecOps rules + optional external linters.
pub fn run_post_edit(file_path: &Path) -> Result<HookOutcome> {
    if !file_path.exists() {
        return Ok(HookOutcome {
            allowed: true,
            exit_code: 0,
            reason: format!("File `{}` does not exist (skipped).", file_path.display()),
        });
    }

    let content = fs::read_to_string(file_path).unwrap_or_default();
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    // 1. Strict built-in syntax parsers (HCL, YAML, JSON)
    if file_name.ends_with(".tf") {
        if let Err(e) = hcl::parse(&content) {
            let msg = format!(
                "HCL syntax validation failed in `{}`: {}",
                file_path.display(),
                e
            );
            eprintln!("{} {}", "[ops-ecc post-edit]".bold().red(), msg);
            return Ok(HookOutcome {
                allowed: false,
                exit_code: 2,
                reason: msg,
            });
        }
        run_optional_external_linter("tflint", &[file_path.to_str().unwrap_or(".")]);
    } else if file_name.ends_with(".yaml") || file_name.ends_with(".yml") {
        if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
            let msg = format!(
                "YAML syntax validation failed in `{}`: {}",
                file_path.display(),
                e
            );
            eprintln!("{} {}", "[ops-ecc post-edit]".bold().red(), msg);
            return Ok(HookOutcome {
                allowed: false,
                exit_code: 2,
                reason: msg,
            });
        }
        if content.contains("AWSTemplateFormatVersion") {
            run_optional_external_linter("cfn-lint", &[file_path.to_str().unwrap_or(".")]);
        }
    } else if file_name.ends_with(".json") {
        if let Err(e) = serde_json::from_str::<serde_json::Value>(&content) {
            let msg = format!(
                "JSON syntax validation failed in `{}`: {}",
                file_path.display(),
                e
            );
            eprintln!("{} {}", "[ops-ecc post-edit]".bold().red(), msg);
            return Ok(HookOutcome {
                allowed: false,
                exit_code: 2,
                reason: msg,
            });
        }
    } else if file_name == "Dockerfile" || file_name.starts_with("Dockerfile.") {
        run_optional_external_linter("hadolint", &[file_path.to_str().unwrap_or("Dockerfile")]);
    }

    // 2. Run built-in DevSecOps rules on the edited file
    let parent = file_path.parent().unwrap_or_else(|| Path::new("."));
    let findings = scan_single_file(parent, file_path);
    let high_or_crit: Vec<_> = findings
        .iter()
        .filter(|f| f.severity >= Severity::High)
        .collect();

    if !high_or_crit.is_empty() {
        for f in &high_or_crit {
            eprintln!(
                "{} [{}] {}:{} — {} (Fix: {})",
                "[ops-ecc post-edit]".bold().red(),
                f.rule_id,
                f.file,
                f.line,
                f.message,
                f.remediation
            );
        }
        return Ok(HookOutcome {
            allowed: false,
            exit_code: 2,
            reason: format!(
                "{} High/Critical DevSecOps finding(s) in `{}`",
                high_or_crit.len(),
                file_path.display()
            ),
        });
    }

    Ok(HookOutcome {
        allowed: true,
        exit_code: 0,
        reason: format!("`{}` passed post-edit validation.", file_path.display()),
    })
}

fn run_optional_external_linter(bin: &str, args: &[&str]) {
    if let Ok(output) = Command::new(bin).args(args).output() {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            eprintln!(
                "{} External linter `{}` advisory output:
{}{}",
                "[ops-ecc linter]".yellow(),
                bin,
                stdout,
                stderr
            );
        }
    }
}

/// Scan staged git changes (or workspace files if not in a git index) for secrets,
/// `0.0.0.0/0` rules, `:latest` tags, and wildcard IAM policies.
pub fn run_pre_commit(repo_path: &Path, strict: bool) -> Result<HookOutcome> {
    let staged_files = get_staged_files(repo_path);

    let findings = if !staged_files.is_empty() {
        staged_files
            .iter()
            .flat_map(|f| scan_single_file(repo_path, f))
            .collect::<Vec<_>>()
    } else {
        scan_path(repo_path, Severity::High)?.findings
    };

    let blocking: Vec<_> = findings
        .into_iter()
        .filter(|f| f.severity >= Severity::High)
        .collect();

    if !blocking.is_empty() {
        eprintln!(
            "{} Pre-commit gate detected {} High/Critical security issue(s):",
            "[ops-ecc pre-commit BLOCKED]".bold().red(),
            blocking.len()
        );
        for f in &blocking {
            eprintln!(
                "  - [{} / {}] {}:{} -> {}
    Remediation: {}",
                f.severity.as_str(),
                f.rule_id,
                f.file,
                f.line,
                f.message,
                f.remediation
            );
        }
        let code = if strict { 2 } else { 0 };
        return Ok(HookOutcome {
            allowed: !strict,
            exit_code: code,
            reason: format!(
                "Pre-commit scan found {} High/Critical issue(s).",
                blocking.len()
            ),
        });
    }

    println!(
        "{} Pre-commit DevSecOps checks passed.",
        "[ops-ecc pre-commit]".bold().green()
    );
    Ok(HookOutcome {
        allowed: true,
        exit_code: 0,
        reason: "Pre-commit scan clean.".to_string(),
    })
}

fn get_staged_files(repo_path: &Path) -> Vec<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACM"])
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| repo_path.join(l))
            .filter(|p| p.exists())
            .collect(),
        _ => Vec::new(),
    }
}
