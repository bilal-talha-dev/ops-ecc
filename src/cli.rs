use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "ops-ecc",
    version,
    author = "Muhammad Bilal Talha <bilal@ops-ecc.dev>",
    about = "Blazing-fast Full-Stack + Heavy DevOps & DevSecOps Agent Harness CLI for Claude Code (.claude/), Gemini CLI (.gemini/), Cursor (.cursorrules), and Universal AGENTS.md",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize ops-ecc rules, agents, skills, slash commands, and native Rust hooks
    Init(InitArgs),

    /// Audit local machine readiness, DevSecOps CLI tools, and installed hook bindings
    Doctor(DoctorArgs),

    /// High-speed parallel DevOps + DevSecOps static security & hygiene scanner
    Audit(AuditArgs),

    /// Native sub-5ms Rust hook handlers wired into Claude Code, Gemini CLI, and Git
    Hook {
        #[command(subcommand)]
        event: HookEvent,
    },

    /// Generate production-hardened IaC, Docker, OIDC CI/CD, and Edge Proxy templates
    Scaffold(ScaffoldArgs),

    /// Update ops-ecc agents, skills, and rules while preserving USER_CUSTOM blocks
    Update(UpdateArgs),

    /// Generate shell completions for bash, zsh, powershell, or fish
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    /// Non-interactive mode with sensible production defaults
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Harness preset profile
    #[arg(long, value_enum, default_value_t = Preset::All)]
    pub preset: Preset,

    /// Target AI CLIs to configure (comma-separated: claude,gemini,cursor,universal)
    #[arg(long, value_enum, value_delimiter = ',', default_values_t = vec![
        TargetTool::Claude,
        TargetTool::Gemini,
        TargetTool::Cursor,
        TargetTool::Universal
    ])]
    pub tools: Vec<TargetTool>,

    /// Configuration scope: project (./) or global (~/.claude, ~/.gemini)
    #[arg(long, value_enum, default_value_t = ConfigScope::Project)]
    pub scope: ConfigScope,

    /// Cloud provider profile
    #[arg(long, value_enum, default_value_t = CloudProvider::Both)]
    pub cloud: CloudProvider,

    /// Enable strict DevSecOps gate mode (exit code 1/2 blocking commits and commands on Critical/High findings)
    #[arg(long)]
    pub strict: bool,

    /// Target project directory (defaults to current working directory)
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Fullstack,
    Devops,
    Devsecops,
    All,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum TargetTool {
    Claude,
    Gemini,
    Cursor,
    Universal,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigScope {
    Project,
    Global,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudProvider {
    Aws,
    Azure,
    Both,
}

#[derive(Args, Debug, Clone)]
pub struct DoctorArgs {
    /// Target repository path to inspect for `.claude/settings.json` and `.gemini/settings.json`
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Args, Debug, Clone)]
pub struct AuditArgs {
    /// Directory or file path to scan
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Exit with non-zero code (1) if findings at or above the threshold severity exist
    #[arg(long)]
    pub strict: bool,

    /// Minimum severity threshold to report
    #[arg(long, value_enum, default_value_t = Severity::Low)]
    pub severity: Severity,

    /// Output report format (table, json, sarif)
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,

    /// Optional file path to write the report output
    #[arg(long, short = 'o')]
    pub output: Option<PathBuf>,
}

#[derive(
    ValueEnum, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::High => "HIGH",
            Severity::Medium => "MEDIUM",
            Severity::Low => "LOW",
        }
    }

    pub fn sarif_level(&self) -> &'static str {
        match self {
            Severity::Critical | Severity::High => "error",
            Severity::Medium => "warning",
            Severity::Low => "note",
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
    Sarif,
}

#[derive(Subcommand, Debug, Clone)]
pub enum HookEvent {
    /// Validate edited .tf, .yaml/.yml, Dockerfile, or .json file immediately after edit
    PostEdit {
        /// Path to the edited file
        file_path: PathBuf,
    },
    /// Scan staged git changes (or workspace) for secrets, 0.0.0.0/0 rules, :latest tags, and wildcard IAM
    PreCommit {
        /// Target repository directory
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Enforce strict blocking mode
        #[arg(long, default_value_t = true)]
        strict: bool,
    },
    /// Inspect shell commands before execution and block destructive operations
    GuardCmd {
        /// Shell command string to inspect
        command: String,
    },
}

#[derive(Args, Debug, Clone)]
pub struct ScaffoldArgs {
    /// Production template to scaffold
    #[arg(value_enum)]
    pub template: ScaffoldTemplate,

    /// Destination directory where scaffold files will be written
    #[arg(long, default_value = ".")]
    pub out: PathBuf,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaffoldTemplate {
    #[value(name = "terraform-aws")]
    TerraformAws,
    #[value(name = "terraform-azure")]
    TerraformAzure,
    #[value(name = "cfn-aws")]
    CfnAws,
    #[value(name = "docker-prod")]
    DockerProd,
    #[value(name = "cicd-oidc")]
    CicdOidc,
    #[value(name = "edge-proxy")]
    EdgeProxy,
}

impl ScaffoldTemplate {
    pub fn dir_name(&self) -> &'static str {
        match self {
            ScaffoldTemplate::TerraformAws => "terraform-aws",
            ScaffoldTemplate::TerraformAzure => "terraform-azure",
            ScaffoldTemplate::CfnAws => "cfn-aws",
            ScaffoldTemplate::DockerProd => "docker-prod",
            ScaffoldTemplate::CicdOidc => "cicd-oidc",
            ScaffoldTemplate::EdgeProxy => "edge-proxy",
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct UpdateArgs {
    /// Target directory to update
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Configuration scope: project or global
    #[arg(long, value_enum, default_value_t = ConfigScope::Project)]
    pub scope: ConfigScope,
}
