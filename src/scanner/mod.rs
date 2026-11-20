pub mod rules;
pub mod sarif;

use crate::cli::{AuditArgs, OutputFormat, Severity};
use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement, Table};
use ignore::WalkBuilder;
use owo_colors::OwoColorize;
use rayon::prelude::*;
use rules::{
    scan_cloudformation, scan_compose_or_k8s, scan_dockerfile, scan_github_workflow,
    scan_lockfile_hygiene, scan_terraform, Finding, SECRET_RULES,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct AuditSummary {
    pub findings: Vec<Finding>,
    pub scanned_files: usize,
    pub elapsed_ms: u128,
}

/// Scan a single file and return all matching findings.
pub fn scan_single_file(root: &Path, file_path: &Path) -> Vec<Finding> {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(), // Skip binary or unreadable files
    };

    let rel_path = file_path
        .strip_prefix(root)
        .unwrap_or(file_path)
        .display()
        .to_string()
        .replace('\\', "/");

    // Skip `.ops-ecc-backup-*` or `.claude` / `.gemini` embedded markdown documentation
    if rel_path.contains(".ops-ecc-backup-")
        || rel_path.starts_with(".claude/")
        || rel_path.starts_with(".gemini/")
        || rel_path == "AGENTS.md"
        || rel_path == "GEMINI.md"
        || rel_path == ".cursorrules"
    {
        return Vec::new();
    }

    let mut findings = Vec::new();

    // 1. Secret scanning across all text files (excluding lockfiles)
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    if !file_name.ends_with(".lock") && file_name != "package-lock.json" && !file_name.ends_with(".sarif") {
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            for rule in SECRET_RULES.iter() {
                if rule.regex.is_match(line) {
                    findings.push(Finding {
                        rule_id: rule.id.to_string(),
                        category: rule.category.to_string(),
                        severity: rule.severity,
                        file: rel_path.clone(),
                        line: idx + 1,
                        message: rule.message.to_string(),
                        remediation: rule.remediation.to_string(),
                        snippet: mask_snippet(trimmed),
                    });
                }
            }
        }
    }

    // 2. Dockerfile CIS scanner
    if file_name == "Dockerfile"
        || file_name.starts_with("Dockerfile.")
        || file_name.ends_with(".dockerfile")
    {
        findings.extend(scan_dockerfile(&rel_path, &content));
    }

    // 3. Terraform (.tf) scanner
    if file_name.ends_with(".tf") {
        findings.extend(scan_terraform(&rel_path, &content));
    }

    // 4. YAML / JSON files: GitHub Actions, CloudFormation, Docker Compose, Kubernetes
    if file_name.ends_with(".yml") || file_name.ends_with(".yaml") || file_name.ends_with(".json") {
        if rel_path.contains(".github/workflows/") {
            findings.extend(scan_github_workflow(&rel_path, &content));
        }
        findings.extend(scan_cloudformation(&rel_path, &content));
        if file_name.ends_with(".yml") || file_name.ends_with(".yaml") {
            findings.extend(scan_compose_or_k8s(&rel_path, &content));
        }
    }

    findings
}

fn mask_snippet(line: &str) -> String {
    if line.len() > 120 {
        format!("{}...", &line[..117])
    } else {
        line.to_string()
    }
}

/// Parallel `.gitignore`-aware repository scanner using `ignore` + `rayon`.
pub fn scan_path(target_path: &Path, min_severity: Severity) -> Result<AuditSummary> {
    let start = Instant::now();

    if target_path.is_file() {
        let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
        let mut findings = scan_single_file(parent, target_path);
        findings.retain(|f| f.severity >= min_severity);
        return Ok(AuditSummary {
            findings,
            scanned_files: 1,
            elapsed_ms: start.elapsed().as_millis(),
        });
    }

    let mut files: Vec<PathBuf> = Vec::new();
    let mut walker_builder = WalkBuilder::new(target_path);
    WalkBuilder::hidden(&mut walker_builder, false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .filter_entry(|entry| {
            let name = entry.file_name().to_str().unwrap_or("");
            name != ".git"
                && name != "node_modules"
                && name != "target"
                && !name.starts_with(".ops-ecc-backup-")
        })
        ;
    let walker = walker_builder.build();

    for entry in walker.flatten() {
        if entry.path().is_file() {
            files.push(entry.into_path());
        }
    }

    let scanned_files = files.len();

    let mut findings: Vec<Finding> = files
        .par_iter()
        .flat_map(|file| scan_single_file(target_path, file))
        .collect();

    findings.extend(scan_lockfile_hygiene(target_path));
    findings.retain(|f| f.severity >= min_severity);

    // Sort by severity descending, then file path and line number
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });

    Ok(AuditSummary {
        findings,
        scanned_files,
        elapsed_ms: start.elapsed().as_millis(),
    })
}

pub fn run_audit(args: &AuditArgs) -> Result<AuditSummary> {
    let summary = scan_path(&args.path, args.severity)?;

    let rendered = match args.format {
        OutputFormat::Json => serde_json::to_string_pretty(&summary.findings)?,
        OutputFormat::Sarif => sarif::to_sarif_json(&summary.findings)?,
        OutputFormat::Table => render_findings_table(&summary),
    };

    if let Some(ref out_file) = args.output {
        if let Some(parent) = out_file.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(out_file, &rendered)
            .with_context(|| format!("Failed to write audit output to {}", out_file.display()))?;
        println!(
            "{} Wrote {:?} audit report to `{}` ({} findings in {}ms)",
            "[ops-ecc]".bold().green(),
            args.format,
            out_file.display(),
            summary.findings.len(),
            summary.elapsed_ms
        );
    } else {
        println!("{}", rendered);
    }

    Ok(summary)
}

fn render_findings_table(summary: &AuditSummary) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}
",
        format!(
            "=== ops-ecc Parallel DevSecOps Audit ({} files scanned in {}ms) ===",
            summary.scanned_files, summary.elapsed_ms
        )
        .bold()
        .cyan()
    ));

    if summary.findings.is_empty() {
        out.push_str(&format!(
            "{}
",
            "✔ Zero security or hygiene findings detected.".bold().green()
        ));
        return out;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Severity").add_attribute(Attribute::Bold),
            Cell::new("Rule ID").add_attribute(Attribute::Bold),
            Cell::new("Category").add_attribute(Attribute::Bold),
            Cell::new("Location").add_attribute(Attribute::Bold),
            Cell::new("Finding & Remediation").add_attribute(Attribute::Bold),
        ]);

    for f in &summary.findings {
        let sev_cell = match f.severity {
            Severity::Critical => Cell::new("CRITICAL")
                .fg(Color::Red)
                .add_attribute(Attribute::Bold),
            Severity::High => Cell::new("HIGH")
                .fg(Color::DarkRed)
                .add_attribute(Attribute::Bold),
            Severity::Medium => Cell::new("MEDIUM")
                .fg(Color::Yellow)
                .add_attribute(Attribute::Bold),
            Severity::Low => Cell::new("LOW").fg(Color::Cyan),
        };

        table.add_row(vec![
            sev_cell,
            Cell::new(&f.rule_id).fg(Color::Magenta),
            Cell::new(&f.category),
            Cell::new(format!("{}:{}", f.file, f.line)),
            Cell::new(format!("{}
-> Fix: {}", f.message, f.remediation)),
        ]);
    }

    out.push_str(&format!("{table}
"));
    let crit = summary
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .count();
    let high = summary
        .findings
        .iter()
        .filter(|f| f.severity == Severity::High)
        .count();
    let med = summary
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Medium)
        .count();
    let low = summary
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Low)
        .count();

    out.push_str(&format!(
        "Total Findings: {} (Critical: {}, High: {}, Medium: {}, Low: {})
",
        summary.findings.len(),
        crit,
        high,
        med,
        low
    ));
    out
}
