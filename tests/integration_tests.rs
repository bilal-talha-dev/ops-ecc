use ops_ecc::cli::{
    AuditArgs, CloudProvider, ConfigScope, DoctorArgs, HookEvent, InitArgs, OutputFormat, Preset,
    ScaffoldArgs, ScaffoldTemplate, Severity, TargetTool, UpdateArgs,
};
use ops_ecc::doctor::{check_all_tools, check_hook_bindings};
use ops_ecc::hook::{execute_hook, run_guard_cmd, run_post_edit};
use ops_ecc::init::run_init;
use ops_ecc::scaffold::run_scaffold;
use ops_ecc::scanner::{run_audit, scan_path};
use ops_ecc::update::run_update;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_init_creates_harnesses_and_timestamped_backup() {
    let dir = tempdir().expect("tempdir");
    let repo = dir.path();

    // Create sample project files for stack detection
    fs::write(
        repo.join("package.json"),
        r#"{"name": "web", "dependencies": {"next": "14.2.0"}}"#,
    )
    .unwrap();
    fs::write(repo.join("package-lock.json"), "{}").unwrap();
    fs::write(repo.join("main.tf"), "terraform {}
").unwrap();

    let args = InitArgs {
        yes: true,
        preset: Preset::All,
        tools: vec![
            TargetTool::Claude,
            TargetTool::Gemini,
            TargetTool::Cursor,
            TargetTool::Universal,
        ],
        scope: ConfigScope::Project,
        cloud: CloudProvider::Both,
        strict: true,
        path: repo.to_path_buf(),
    };

    let report1 = run_init(args.clone()).expect("first init");
    assert!(report1.backup_dir.is_none());
    assert!(report1.detected_stack.has_nextjs);
    assert!(report1.detected_stack.has_terraform);
    assert!(repo.join(".claude/settings.json").exists());
    assert!(repo.join(".claude/rules/01-fullstack-sdd-tdd.md").exists());
    assert!(repo.join(".claude/agents/devsecops-auditor.md").exists());
    assert!(repo.join(".claude/commands/sec-audit.md").exists());
    assert!(repo
        .join(".claude/skills/terraform-aws-azure/SKILL.md")
        .exists());
    assert!(repo.join(".gemini/settings.json").exists());
    assert!(repo.join("GEMINI.md").exists());
    assert!(repo.join(".cursorrules").exists());
    assert!(repo.join("AGENTS.md").exists());

    // Verify hook bindings via doctor helper
    let (claude_ok, gemini_ok) = check_hook_bindings(repo);
    assert!(claude_ok);
    assert!(gemini_ok);

    // Second init must create a `.ops-ecc-backup-<YYYYMMDD-HHMMSS>/` directory
    let report2 = run_init(args).expect("second init");
    assert!(report2.backup_dir.is_some());
    let bkp = report2.backup_dir.unwrap();
    assert!(bkp.exists());
    assert!(bkp.join(".claude/settings.json").exists());
}

#[test]
fn test_audit_detects_secrets_dockerfile_compose_iac_gha_and_lockfile() {
    let dir = tempdir().expect("tempdir");
    let repo = dir.path();

    // 1. Secret + unpinned dependency in package.json (without lockfile)
    fs::write(
        repo.join("package.json"),
        r#"{
  "name": "vuln-app",
  "dependencies": {
    "left-pad": "*"
  }
}"#,
    )
    .unwrap();

    fs::write(
        repo.join(".env.production"),
        "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
DATABASE_URL=postgres://admin:SuperSecret123@db.internal:5432/prod
",
    )
    .unwrap();

    // 2. Vulnerable Dockerfile
    fs::write(
        repo.join("Dockerfile"),
        r#"FROM node:latest
USER root
ENV API_SECRET_KEY="hardcoded"
RUN curl -sSL https://example.com/install.sh | bash
EXPOSE 22 3000
CMD ["node", "index.js"]
"#,
    )
    .unwrap();

    // 3. Vulnerable docker-compose.yml
    fs::write(
        repo.join("docker-compose.yml"),
        "services:
  api:
    image: node:20
    privileged: true
    network_mode: host
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
",
    )
    .unwrap();

    // 4. Vulnerable Terraform (.tf)
    fs::write(
        repo.join("insecure.tf"),
        r#"
terraform {
  backend "s3" {
    bucket = "my-tf-state"
    key    = "state"
  }
}

resource "aws_security_group" "open_ssh" {
  name = "open-ssh"
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_s3_bucket_acl" "bad_acl" {
  bucket = "my-bucket"
  acl    = "public-read"
}

resource "aws_db_instance" "unencrypted_db" {
  identifier        = "prod-db"
  engine            = "postgres"
  storage_encrypted = false
}
"#,
    )
    .unwrap();

    // 5. Vulnerable GitHub Actions Workflow
    let wf_dir = repo.join(".github/workflows");
    fs::create_dir_all(&wf_dir).unwrap();
    fs::write(
        wf_dir.join("ci.yml"),
        r#"
name: Insecure CI
on: [pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@main
      - name: Unsafe echo
        env:
          AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
        run: |
          echo "PR title: ${{ github.event.pull_request.title }}"
"#,
    )
    .unwrap();

    let summary = scan_path(repo, Severity::Low).expect("scan_path");
    let rule_ids: Vec<&str> = summary.findings.iter().map(|f| f.rule_id.as_str()).collect();

    assert!(rule_ids.contains(&"SEC-001"), "Should detect AKIA key");
    assert!(rule_ids.contains(&"SEC-006"), "Should detect Postgres password URI");
    assert!(rule_ids.contains(&"DOCKER-001"), "Should detect :latest tag");
    assert!(rule_ids.contains(&"DOCKER-002"), "Should detect USER root");
    assert!(rule_ids.contains(&"DOCKER-003"), "Should detect missing HEALTHCHECK");
    assert!(rule_ids.contains(&"DOCKER-004"), "Should detect ENV secret");
    assert!(rule_ids.contains(&"DOCKER-005"), "Should detect curl | bash");
    assert!(rule_ids.contains(&"K8S-001"), "Should detect privileged: true");
    assert!(rule_ids.contains(&"K8S-002"), "Should detect network_mode: host");
    assert!(rule_ids.contains(&"K8S-003"), "Should detect docker.sock mount");
    assert!(rule_ids.contains(&"IAC-001"), "Should detect 0.0.0.0/0 on port 22");
    assert!(rule_ids.contains(&"IAC-002"), "Should detect storage_encrypted = false");
    assert!(rule_ids.contains(&"IAC-003"), "Should detect public-read ACL");
    assert!(rule_ids.contains(&"IAC-005"), "Should detect missing prevent_destroy");
    assert!(rule_ids.contains(&"IAC-006"), "Should detect missing S3 state lock");
    assert!(rule_ids.contains(&"CICD-001"), "Should detect mutable @main action");
    assert!(rule_ids.contains(&"CICD-002"), "Should detect static AWS_SECRET_ACCESS_KEY");
    assert!(rule_ids.contains(&"CICD-003"), "Should detect GHA script injection");
    assert!(rule_ids.contains(&"DEP-001"), "Should detect missing lockfile");
    assert!(rule_ids.contains(&"DEP-002"), "Should detect wildcard dependency");

    // Verify SARIF 2.1.0 output generation
    let sarif_file = repo.join("report.sarif");
    let audit_args = AuditArgs {
        path: repo.to_path_buf(),
        strict: true,
        severity: Severity::Low,
        format: OutputFormat::Sarif,
        output: Some(sarif_file.clone()),
    };
    run_audit(&audit_args).expect("run_audit sarif");
    let sarif_text = fs::read_to_string(&sarif_file).expect("read sarif");
    let sarif_val: serde_json::Value = serde_json::from_str(&sarif_text).expect("valid sarif json");
    assert_eq!(sarif_val["version"], "2.1.0");
    assert_eq!(sarif_val["runs"][0]["tool"]["driver"]["name"], "ops-ecc");
    assert!(!sarif_val["runs"][0]["results"].as_array().unwrap().is_empty());
}

#[test]
fn test_hooks_guard_cmd_post_edit_and_pre_commit() {
    // 1. guard-cmd blocks destructive commands with exit code 2
    for bad_cmd in [
        "terraform destroy -auto-approve",
        "aws s3 rb s3://prod-bucket --force",
        "psql -c 'DROP DATABASE production;'",
        "rm -rf /",
        "kubectl delete namespace prod",
    ] {
        let out = run_guard_cmd(bad_cmd);
        assert!(!out.allowed, "Command should be blocked: {}", bad_cmd);
        assert_eq!(out.exit_code, 2);
    }

    // Safe command passes
    let ok_cmd = run_guard_cmd("terraform plan -out=tfplan && cargo test");
    assert!(ok_cmd.allowed);
    assert_eq!(ok_cmd.exit_code, 0);

    // 2. post-edit catches invalid HCL syntax and passes valid hardened HCL
    let dir = tempdir().expect("tempdir");
    let broken_tf = dir.path().join("broken.tf");
    fs::write(&broken_tf, r#"resource "aws_s3_bucket" "b" { unclosed = "#).unwrap();
    let res_broken = run_post_edit(&broken_tf).expect("post_edit broken");
    assert!(!res_broken.allowed);
    assert_eq!(res_broken.exit_code, 2);

    let valid_tf = dir.path().join("valid.tf");
    fs::write(
        &valid_tf,
        r#"resource "aws_s3_bucket" "b" {
  bucket = "my-safe-bucket"
}
"#,
    )
    .unwrap();
    let res_valid = run_post_edit(&valid_tf).expect("post_edit valid");
    assert!(res_valid.allowed);
    assert_eq!(res_valid.exit_code, 0);

    // 3. pre-commit blocks when high/critical findings exist in workspace
    fs::write(
        dir.path().join("leak.env"),
        "AWS_KEY=AKIAIOSFODNN7EXAMPLE
",
    )
    .unwrap();
    let pre_res = execute_hook(&HookEvent::PreCommit {
        path: dir.path().to_path_buf(),
        strict: true,
    })
    .expect("pre_commit");
    assert!(!pre_res.allowed);
    assert_eq!(pre_res.exit_code, 2);
}

#[test]
fn test_all_scaffolds_extract_and_pass_strict_audit() {
    let templates = [
        ScaffoldTemplate::TerraformAws,
        ScaffoldTemplate::TerraformAzure,
        ScaffoldTemplate::CfnAws,
        ScaffoldTemplate::DockerProd,
        ScaffoldTemplate::CicdOidc,
        ScaffoldTemplate::EdgeProxy,
    ];

    for tpl in templates {
        let dir = tempdir().expect("tempdir");
        let out_dir = dir.path().join(tpl.dir_name());
        let written = run_scaffold(&ScaffoldArgs {
            template: tpl,
            out: out_dir.clone(),
        })
        .expect("scaffold");
        assert!(!written.is_empty());

        // Every production scaffold embedded in ops-ecc must pass High/Critical audit cleanly!
        let summary = scan_path(&out_dir, Severity::High).expect("scan scaffold");
        assert!(
            summary.findings.is_empty(),
            "Scaffold `{}` had unexpected High/Critical findings: {:?}",
            tpl.dir_name(),
            summary.findings
        );
    }
}

#[test]
fn test_update_preserves_user_custom_blocks() {
    let dir = tempdir().expect("tempdir");
    let repo = dir.path();

    run_init(InitArgs {
        yes: true,
        preset: Preset::All,
        tools: vec![TargetTool::Claude, TargetTool::Universal],
        scope: ConfigScope::Project,
        cloud: CloudProvider::Both,
        strict: true,
        path: repo.to_path_buf(),
    })
    .expect("init");

    let rule_path = repo.join(".claude/rules/01-fullstack-sdd-tdd.md");
    let original = fs::read_to_string(&rule_path).unwrap();
    let customized = original.replace(
        "<!-- Add team-specific full-stack overrides inside this block. Preserved across `ops-ecc update`. -->",
        "CUSTOM_TEAM_RULE: Always use UUIDv7 primary keys in PostgreSQL.",
    );
    fs::write(&rule_path, &customized).unwrap();

    let updated_count = run_update(&UpdateArgs {
        path: repo.to_path_buf(),
        scope: ConfigScope::Project,
    })
    .expect("update");
    assert!(updated_count > 0);

    let after_update = fs::read_to_string(&rule_path).unwrap();
    assert!(
        after_update.contains("CUSTOM_TEAM_RULE: Always use UUIDv7 primary keys in PostgreSQL."),
        "Custom block inside USER_CUSTOM_START/END must be preserved across ops-ecc update"
    );
}

#[test]
fn test_doctor_parallel_tool_check_executes() {
    let dir = tempdir().expect("tempdir");
    let checks = check_all_tools();
    assert_eq!(checks.len(), 13);
    ops_ecc::doctor::run_doctor(&DoctorArgs {
        path: dir.path().to_path_buf(),
    })
    .expect("run_doctor");
}
