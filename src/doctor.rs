use crate::cli::DoctorArgs;
use anyhow::Result;
use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement, Table};
use owo_colors::OwoColorize;
use rayon::prelude::*;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub binary: &'static str,
    pub category: &'static str,
    pub version_arg: &'static str,
    pub install_hint: &'static str,
}

#[derive(Debug, Clone)]
pub struct ToolCheckResult {
    pub spec: ToolSpec,
    pub installed: bool,
    pub version: String,
}

pub const REQUIRED_TOOLS: &[ToolSpec] = &[
    ToolSpec {
        binary: "git",
        category: "VCS",
        version_arg: "--version",
        install_hint: "brew install git | apt install git | winget install Git.Git",
    },
    ToolSpec {
        binary: "docker",
        category: "Container Runtime",
        version_arg: "--version",
        install_hint: "https://docs.docker.com/get-docker/",
    },
    ToolSpec {
        binary: "terraform",
        category: "IaC Engine",
        version_arg: "--version",
        install_hint: "brew tap hashicorp/tap && brew install hashicorp/tap/terraform",
    },
    ToolSpec {
        binary: "tflint",
        category: "Terraform Linter",
        version_arg: "--version",
        install_hint: "brew install tflint | choco install tflint",
    },
    ToolSpec {
        binary: "checkov",
        category: "IaC Policy Scanner",
        version_arg: "--version",
        install_hint: "pipx install checkov | brew install checkov",
    },
    ToolSpec {
        binary: "cfn-lint",
        category: "CloudFormation Linter",
        version_arg: "--version",
        install_hint: "pipx install cfn-lint | brew install cfn-lint",
    },
    ToolSpec {
        binary: "hadolint",
        category: "Dockerfile CIS Linter",
        version_arg: "--version",
        install_hint: "brew install hadolint | scoop install hadolint",
    },
    ToolSpec {
        binary: "trivy",
        category: "Container/SBOM Scanner",
        version_arg: "--version",
        install_hint: "brew install trivy | apt install trivy",
    },
    ToolSpec {
        binary: "gitleaks",
        category: "Secret Scanner",
        version_arg: "version",
        install_hint: "brew install gitleaks | choco install gitleaks",
    },
    ToolSpec {
        binary: "aws",
        category: "AWS Cloud CLI",
        version_arg: "--version",
        install_hint: "brew install awscli | winget install Amazon.AWSCLI",
    },
    ToolSpec {
        binary: "az",
        category: "Azure Cloud CLI",
        version_arg: "--version",
        install_hint: "brew install azure-cli | winget install Microsoft.AzureCLI",
    },
    ToolSpec {
        binary: "node",
        category: "Full-Stack JS Runtime",
        version_arg: "--version",
        install_hint: "brew install node | fnm install --lts",
    },
    ToolSpec {
        binary: "python3",
        category: "Python Runtime",
        version_arg: "--version",
        install_hint: "brew install python | apt install python3",
    },
];

pub fn check_all_tools() -> Vec<ToolCheckResult> {
    REQUIRED_TOOLS
        .par_iter()
        .map(|spec| {
            let output = Command::new(spec.binary).arg(spec.version_arg).output();
            match output {
                Ok(out) if out.status.success() => {
                    let raw = String::from_utf8_lossy(&out.stdout);
                    let err_raw = String::from_utf8_lossy(&out.stderr);
                    let combined = if raw.trim().is_empty() {
                        err_raw.lines().next().unwrap_or("detected").trim().to_string()
                    } else {
                        raw.lines().next().unwrap_or("detected").trim().to_string()
                    };
                    ToolCheckResult {
                        spec: spec.clone(),
                        installed: true,
                        version: combined,
                    }
                }
                _ => ToolCheckResult {
                    spec: spec.clone(),
                    installed: false,
                    version: "Not found in PATH".to_string(),
                },
            }
        })
        .collect()
}

pub fn check_hook_bindings(repo_path: &Path) -> (bool, bool) {
    let claude_settings = repo_path.join(".claude/settings.json");
    let gemini_settings = repo_path.join(".gemini/settings.json");

    let claude_ok = std::fs::read_to_string(&claude_settings)
        .map(|s| s.contains("ops-ecc hook"))
        .unwrap_or(false);
    let gemini_ok = std::fs::read_to_string(&gemini_settings)
        .map(|s| s.contains("ops-ecc hook"))
        .unwrap_or(false);

    (claude_ok, gemini_ok)
}

pub fn run_doctor(args: &DoctorArgs) -> Result<()> {
    println!(
        "{}",
        "=== ops-ecc Doctor: Parallel Machine & Harness Readiness Audit ==="
            .bold()
            .cyan()
    );

    let results = check_all_tools();
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Tool").add_attribute(Attribute::Bold),
            Cell::new("Category").add_attribute(Attribute::Bold),
            Cell::new("Status").add_attribute(Attribute::Bold),
            Cell::new("Detected Version / Remediation").add_attribute(Attribute::Bold),
        ]);

    let mut installed_count = 0usize;
    for r in &results {
        if r.installed {
            installed_count += 1;
            table.add_row(vec![
                Cell::new(r.spec.binary).fg(Color::Cyan),
                Cell::new(r.spec.category),
                Cell::new("READY").fg(Color::Green).add_attribute(Attribute::Bold),
                Cell::new(&r.version),
            ]);
        } else {
            table.add_row(vec![
                Cell::new(r.spec.binary).fg(Color::Yellow),
                Cell::new(r.spec.category),
                Cell::new("OPTIONAL / MISSING")
                    .fg(Color::Yellow)
                    .add_attribute(Attribute::Bold),
                Cell::new(format!("Install: {}", r.spec.install_hint)),
            ]);
        }
    }

    let (claude_ok, gemini_ok) = check_hook_bindings(&args.path);
    table.add_row(vec![
        Cell::new(".claude/settings.json").fg(Color::Magenta),
        Cell::new("Claude Code Hooks"),
        if claude_ok {
            Cell::new("WIRED").fg(Color::Green).add_attribute(Attribute::Bold)
        } else {
            Cell::new("NOT INITIALIZED").fg(Color::Yellow)
        },
        Cell::new(if claude_ok {
            "ops-ecc native Rust hooks active (post-edit, guard-cmd, pre-commit)"
        } else {
            "Run: ops-ecc init --yes"
        }),
    ]);

    table.add_row(vec![
        Cell::new(".gemini/settings.json").fg(Color::Magenta),
        Cell::new("Gemini CLI Hooks"),
        if gemini_ok {
            Cell::new("WIRED").fg(Color::Green).add_attribute(Attribute::Bold)
        } else {
            Cell::new("NOT INITIALIZED").fg(Color::Yellow)
        },
        Cell::new(if gemini_ok {
            "ops-ecc native Rust hooks active (post-edit, guard-cmd, pre-commit)"
        } else {
            "Run: ops-ecc init --yes"
        }),
    ]);

    println!("{table}");
    println!(
        "
{} {}/{} external tools detected. Note: `ops-ecc audit` and `ops-ecc hook` include built-in Rust HCL/YAML/JSON/Dockerfile/Secret scanners and run standalone even when external linters are absent.",
        "Summary:".bold(),
        installed_count,
        results.len()
    );

    Ok(())
}
