use crate::assets::{extract_embedded_subdir, list_embedded_markdown};
use crate::cli::{CloudProvider, ConfigScope, InitArgs, Preset, TargetTool};
use crate::detect::DetectedStack;
use crate::update::write_markdown_preserving_custom;
use anyhow::{Context, Result};
use chrono::Local;
use console::Term;
use dialoguer::{Confirm, MultiSelect, Select};
use owo_colors::OwoColorize;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct InitReport {
    pub target_dir: PathBuf,
    pub backup_dir: Option<PathBuf>,
    pub detected_stack: DetectedStack,
    pub written_files: Vec<PathBuf>,
}

pub fn run_init(mut args: InitArgs) -> Result<InitReport> {
    let base_dir: PathBuf = match args.scope {
        ConfigScope::Project => args.path.clone(),
        ConfigScope::Global => dirs::home_dir().unwrap_or_else(|| args.path.clone()),
    };

    fs::create_dir_all(&base_dir)
        .with_context(|| format!("Failed to create target directory: {}", base_dir.display()))?;

    let detected_stack = DetectedStack::detect(&base_dir);

    // Interactive wizard when `--yes` is not supplied and stdout is a terminal
    if !args.yes && Term::stdout().is_term() {
        prompt_interactive_options(&mut args, &detected_stack)?;
    }

    // Create timestamped backup if `.claude/`, `.gemini/`, `.cursorrules`, or `AGENTS.md` already exist
    let backup_dir = create_backup_if_needed(&base_dir)?;

    let mut written_files = Vec::new();
    let rules = filter_rules_for_preset(&list_embedded_markdown("rules")?, args.preset);
    let agents = list_embedded_markdown("agents")?;
    let commands = list_embedded_markdown("commands")?;

    let strict_flag = if args.strict { " --strict" } else { "" };

    // 1. Claude Code (.claude/)
    if args.tools.contains(&TargetTool::Claude) {
        let claude_dir = base_dir.join(".claude");
        fs::create_dir_all(&claude_dir)?;

        for (name, content) in &rules {
            let p = claude_dir.join("rules").join(name);
            write_markdown_preserving_custom(&p, content)?;
            written_files.push(p);
        }
        for (name, content) in &agents {
            let p = claude_dir.join("agents").join(name);
            write_markdown_preserving_custom(&p, content)?;
            written_files.push(p);
        }
        for (name, content) in &commands {
            let p = claude_dir.join("commands").join(name);
            write_markdown_preserving_custom(&p, content)?;
            written_files.push(p);
        }
        let skill_paths = extract_embedded_subdir("skills", &claude_dir.join("skills"))?;
        for sp in skill_paths {
            written_files.push(PathBuf::from(sp));
        }

        let claude_settings_path = claude_dir.join("settings.json");
        let claude_settings = json!({
            "harness": "ops-ecc",
            "version": env!("CARGO_PKG_VERSION"),
            "preset": args.preset,
            "cloud": args.cloud,
            "strictGate": args.strict,
            "detectedStack": detected_stack.summary_badges(),
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "ops-ecc hook guard-cmd \"$CLAUDE_TOOL_INPUT_COMMAND\""
                            }
                        ]
                    }
                ],
                "PostToolUse": [
                    {
                        "matcher": "Edit|Write|MultiEdit",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "ops-ecc hook post-edit \"$CLAUDE_FILE_PATH\""
                            }
                        ]
                    }
                ],
                "PreCommit": [
                    {
                        "hooks": [
                            {
                                "type": "command",
                                "command": format!("ops-ecc hook pre-commit{}", strict_flag)
                            }
                        ]
                    }
                ]
            }
        });
        fs::write(
            &claude_settings_path,
            serde_json::to_string_pretty(&claude_settings)?,
        )?;
        written_files.push(claude_settings_path);
    }

    // 2. Gemini CLI (.gemini/)
    if args.tools.contains(&TargetTool::Gemini) {
        let gemini_dir = base_dir.join(".gemini");
        fs::create_dir_all(&gemini_dir)?;

        for (name, content) in &rules {
            let p = gemini_dir.join("rules").join(name);
            write_markdown_preserving_custom(&p, content)?;
            written_files.push(p);
        }
        for (name, content) in &agents {
            let p = gemini_dir.join("agents").join(name);
            write_markdown_preserving_custom(&p, content)?;
            written_files.push(p);
        }
        for (name, content) in &commands {
            let p = gemini_dir.join("commands").join(name);
            write_markdown_preserving_custom(&p, content)?;
            written_files.push(p);
        }
        let skill_paths = extract_embedded_subdir("skills", &gemini_dir.join("skills"))?;
        for sp in skill_paths {
            written_files.push(PathBuf::from(sp));
        }

        let gemini_settings_path = gemini_dir.join("settings.json");
        let gemini_settings = json!({
            "harness": "ops-ecc",
            "version": env!("CARGO_PKG_VERSION"),
            "preset": args.preset,
            "cloud": args.cloud,
            "strictGate": args.strict,
            "contextFileName": ["GEMINI.md", "AGENTS.md"],
            "hooks": {
                "BeforeTool": [
                    {
                        "matcher": "run_shell_command",
                        "command": "ops-ecc hook guard-cmd \"$GEMINI_COMMAND\""
                    }
                ],
                "AfterTool": [
                    {
                        "matcher": "write_file|replace",
                        "command": "ops-ecc hook post-edit \"$GEMINI_FILE_PATH\""
                    }
                ],
                "PreCommit": [
                    {
                        "command": format!("ops-ecc hook pre-commit{}", strict_flag)
                    }
                ]
            }
        });
        fs::write(
            &gemini_settings_path,
            serde_json::to_string_pretty(&gemini_settings)?,
        )?;
        written_files.push(gemini_settings_path);

        let gemini_md_path = base_dir.join("GEMINI.md");
        let unified_doc = build_unified_harness_markdown(&args, &detected_stack, &rules);
        write_markdown_preserving_custom(&gemini_md_path, &unified_doc)?;
        written_files.push(gemini_md_path);
    }

    // 3. Cursor (.cursorrules)
    if args.tools.contains(&TargetTool::Cursor) {
        let cursor_path = base_dir.join(".cursorrules");
        let unified_doc = build_unified_harness_markdown(&args, &detected_stack, &rules);
        write_markdown_preserving_custom(&cursor_path, &unified_doc)?;
        written_files.push(cursor_path);
    }

    // 4. Universal AGENTS.md
    if args.tools.contains(&TargetTool::Universal) {
        let agents_md_path = base_dir.join("AGENTS.md");
        let unified_doc = build_unified_harness_markdown(&args, &detected_stack, &rules);
        write_markdown_preserving_custom(&agents_md_path, &unified_doc)?;
        written_files.push(agents_md_path);
    }

    println!(
        "{} Initialized `ops-ecc` harness in `{}` ({} files configured).",
        "[ops-ecc]".bold().green(),
        base_dir.display(),
        written_files.len()
    );
    println!(
        "  {} {:?}",
        "Detected Stack:".bold().cyan(),
        detected_stack.summary_badges()
    );
    if let Some(ref bkp) = backup_dir {
        println!(
            "  {} Created timestamped backup at `{}`",
            "Backup:".bold().yellow(),
            bkp.display()
        );
    }

    Ok(InitReport {
        target_dir: base_dir,
        backup_dir,
        detected_stack,
        written_files,
    })
}

fn filter_rules_for_preset<'a>(
    all_rules: &[ (String, &'a str) ],
    preset: Preset,
) -> Vec<(String, &'a str)> {
    all_rules
        .iter()
        .filter(|(name, _)| match preset {
            Preset::All => true,
            Preset::Fullstack => {
                name.starts_with("01-") || name.starts_with("03-")
            }
            Preset::Devops => {
                name.starts_with("02-") || name.starts_with("03-") || name.starts_with("04-")
            }
            Preset::Devsecops => {
                name.starts_with("02-") || name.starts_with("03-") || name.starts_with("04-")
            }
        })
        .cloned()
        .collect()
}

fn build_unified_harness_markdown(
    args: &InitArgs,
    stack: &DetectedStack,
    rules: &[(String, &str)],
) -> String {
    let mut doc = format!(
        "# ops-ecc Full-Stack + Heavy DevOps & DevSecOps Agent Harness

         - **Preset**: `{:?}`
         - **Cloud Profile**: `{:?}`
         - **Security Gate Mode**: `{}`
         - **Auto-Detected Stack**: `{}`

         <!-- USER_CUSTOM_START -->
         <!-- Add project-specific architecture or compliance notes here. Preserved across `ops-ecc update`. -->
         <!-- USER_CUSTOM_END -->

",
        args.preset,
        args.cloud,
        if args.strict { "Strict Gate (Exit 1/2 on Critical/High)" } else { "Advisory" },
        stack.summary_badges().join(", ")
    );

    for (_, content) in rules {
        doc.push_str(content);
        doc.push_str("

");
    }
    doc
}

fn create_backup_if_needed(base_dir: &Path) -> Result<Option<PathBuf>> {
    let targets = [".claude", ".gemini", ".cursorrules", "AGENTS.md"];
    let any_exists = targets.iter().any(|t| base_dir.join(t).exists());
    if !any_exists {
        return Ok(None);
    }

    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_dir = base_dir.join(format!(".ops-ecc-backup-{}", timestamp));
    fs::create_dir_all(&backup_dir)?;

    for target in &targets {
        let src = base_dir.join(target);
        if src.exists() {
            let dst = backup_dir.join(target);
            copy_path_recursive(&src, &dst)?;
        }
    }

    Ok(Some(backup_dir))
}

fn copy_path_recursive(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let child_src = entry.path();
            let child_dst = dst.join(entry.file_name());
            copy_path_recursive(&child_src, &child_dst)?;
        }
    } else if src.is_file() {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}

fn prompt_interactive_options(args: &mut InitArgs, stack: &DetectedStack) -> Result<()> {
    println!(
        "{} Auto-detected repository profile: {:?}",
        "[ops-ecc]".bold().cyan(),
        stack.summary_badges()
    );

    let tool_labels = vec![
        "Claude Code CLI (.claude/)",
        "Gemini CLI (.gemini/ + GEMINI.md)",
        "Cursor (.cursorrules)",
        "Universal (AGENTS.md)",
    ];
    let defaults = vec![true, true, true, true];
    if let Ok(selected) = MultiSelect::new()
        .with_prompt("a) Select Target AI CLIs to configure")
        .items(&tool_labels)
        .defaults(&defaults)
        .interact()
    {
        if !selected.is_empty() {
            args.tools = selected
                .into_iter()
                .map(|i| match i {
                    0 => TargetTool::Claude,
                    1 => TargetTool::Gemini,
                    2 => TargetTool::Cursor,
                    _ => TargetTool::Universal,
                })
                .collect();
        }
    }

    let scopes = vec!["Current Project (./)", "Global User Config (~/.claude, ~/.gemini)"];
    if let Ok(idx) = Select::new()
        .with_prompt("b) Select Configuration Scope")
        .items(&scopes)
        .default(0)
        .interact()
    {
        args.scope = if idx == 0 {
            ConfigScope::Project
        } else {
            ConfigScope::Global
        };
    }

    let presets = vec![
        "All (Full-Stack + Heavy DevOps + DevSecOps)",
        "Full-Stack (Next.js/React + Node/FastAPI/Django + Postgres/Prisma/Alembic + Redis)",
        "DevOps (AWS/Azure + Terraform/CloudFormation + Docker/K8s + Edge Proxy)",
        "DevSecOps (Zero-Trust + CIS Benchmarks + OIDC Supply-Chain + Secret Gate)",
    ];
    if let Ok(idx) = Select::new()
        .with_prompt("c) Select Primary Engineering Profile")
        .items(&presets)
        .default(0)
        .interact()
    {
        args.preset = match idx {
            1 => Preset::Fullstack,
            2 => Preset::Devops,
            3 => Preset::Devsecops,
            _ => Preset::All,
        };
    }

    let clouds = vec![
        "Both AWS & Azure (Multi-Cloud + Terraform + CloudFormation + Docker/K8s)",
        "AWS Primary (EC2, ECS Fargate, EKS, Lambda, RDS, S3, SQS, IAM, VPC, CloudFront)",
        "Azure Primary (VMs, AKS, Container Apps, Azure SQL, Blob, Service Bus, RBAC, VNet)",
    ];
    if let Ok(idx) = Select::new()
        .with_prompt("d) Select Cloud & IaC Profile")
        .items(&clouds)
        .default(0)
        .interact()
    {
        args.cloud = match idx {
            1 => CloudProvider::Aws,
            2 => CloudProvider::Azure,
            _ => CloudProvider::Both,
        };
    }

    let _cicd = Select::new()
        .with_prompt("e) Primary CI/CD Platform")
        .items(&[
            "GitHub Actions (OIDC + SARIF Code Scanning)",
            "GitLab CI",
            "Azure DevOps Pipelines",
        ])
        .default(0)
        .interact();

    if let Ok(strict) = Confirm::new()
        .with_prompt("f) Enable Strict DevSecOps Gate Mode (exit code 1/2 blocking Critical/High findings)?")
        .default(true)
        .interact()
    {
        args.strict = strict;
    }

    Ok(())
}
