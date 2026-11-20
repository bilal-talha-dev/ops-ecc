use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectedStack {
    pub has_node: bool,
    pub has_nextjs: bool,
    pub has_python: bool,
    pub has_rust: bool,
    pub has_docker: bool,
    pub has_terraform: bool,
    pub has_cloudformation: bool,
    pub has_github_actions: bool,
    pub has_gitlab_ci: bool,
    pub has_azure_pipelines: bool,
}

impl DetectedStack {
    pub fn detect(root: &Path) -> Self {
        let mut stack = Self::default();

        let pkg_json = root.join("package.json");
        if pkg_json.exists() {
            stack.has_node = true;
            if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                if content.contains("\"next\"") {
                    stack.has_nextjs = true;
                }
            }
        }

        if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
            stack.has_python = true;
        }

        if root.join("Cargo.toml").exists() {
            stack.has_rust = true;
        }

        if root.join("Dockerfile").exists()
            || root.join("docker-compose.yml").exists()
            || root.join("docker-compose.yaml").exists()
        {
            stack.has_docker = true;
        }

        if root.join(".github/workflows").exists() {
            stack.has_github_actions = true;
        }

        if root.join(".gitlab-ci.yml").exists() {
            stack.has_gitlab_ci = true;
        }

        if root.join("azure-pipelines.yml").exists() {
            stack.has_azure_pipelines = true;
        }

        // Shallow walk (max depth 3) for .tf and CloudFormation templates
        for entry in WalkDir::new(root)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if file_name.ends_with(".tf") {
                stack.has_terraform = true;
            } else if file_name == "template.yaml"
                || file_name == "template.yml"
                || file_name == "template.json"
                || file_name.ends_with(".cfn.yaml")
                || file_name.ends_with(".cfn.json")
            {
                stack.has_cloudformation = true;
            } else if file_name.starts_with("Dockerfile") {
                stack.has_docker = true;
            }
        }

        stack
    }

    pub fn summary_badges(&self) -> Vec<&'static str> {
        let mut badges = Vec::new();
        if self.has_nextjs {
            badges.push("Next.js/Node");
        } else if self.has_node {
            badges.push("Node.js/TypeScript");
        }
        if self.has_python {
            badges.push("Python (FastAPI/Django)");
        }
        if self.has_rust {
            badges.push("Rust");
        }
        if self.has_docker {
            badges.push("Docker/Compose");
        }
        if self.has_terraform {
            badges.push("Terraform (.tf)");
        }
        if self.has_cloudformation {
            badges.push("AWS CloudFormation");
        }
        if self.has_github_actions {
            badges.push("GitHub Actions");
        }
        if self.has_gitlab_ci {
            badges.push("GitLab CI");
        }
        if self.has_azure_pipelines {
            badges.push("Azure Pipelines");
        }
        if badges.is_empty() {
            badges.push("Polyglot Full-Stack + DevSecOps");
        }
        badges
    }
}
