use crate::assets::{extract_embedded_subdir, list_embedded_markdown};
use crate::cli::{ConfigScope, UpdateArgs};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::fs;
use std::path::{Path, PathBuf};

pub const CUSTOM_START_MARKER: &str = "<!-- USER_CUSTOM_START -->";
pub const CUSTOM_END_MARKER: &str = "<!-- USER_CUSTOM_END -->";

/// Merge new template markdown content with an existing file on disk, preserving
/// all sections between `<!-- USER_CUSTOM_START -->` and `<!-- USER_CUSTOM_END -->`.
pub fn merge_preserving_custom_blocks(existing_content: &str, new_template: &str) -> String {
    let existing_blocks = extract_custom_blocks(existing_content);
    if existing_blocks.is_empty() {
        return new_template.to_string();
    }

    let mut result = String::new();
    let mut remaining = new_template;
    let mut block_idx = 0usize;

    while let Some(start_pos) = remaining.find(CUSTOM_START_MARKER) {
        let after_start = &remaining[start_pos + CUSTOM_START_MARKER.len()..];
        if let Some(end_rel) = after_start.find(CUSTOM_END_MARKER) {
            result.push_str(&remaining[..start_pos + CUSTOM_START_MARKER.len()]);
            if let Some(user_block) = existing_blocks.get(block_idx) {
                result.push_str(user_block);
            } else {
                result.push_str(&after_start[..end_rel]);
            }
            result.push_str(CUSTOM_END_MARKER);
            remaining = &after_start[end_rel + CUSTOM_END_MARKER.len()..];
            block_idx += 1;
        } else {
            break;
        }
    }
    result.push_str(remaining);

    // If the existing file had additional custom blocks not present in the template, append them
    while block_idx < existing_blocks.len() {
        result.push_str("\n\n");
        result.push_str(CUSTOM_START_MARKER);
        result.push_str(&existing_blocks[block_idx]);
        result.push_str(CUSTOM_END_MARKER);
        result.push('\n');
        block_idx += 1;
    }

    result
}

fn extract_custom_blocks(content: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut cursor = content;
    while let Some(s) = cursor.find(CUSTOM_START_MARKER) {
        let after = &cursor[s + CUSTOM_START_MARKER.len()..];
        if let Some(e) = after.find(CUSTOM_END_MARKER) {
            blocks.push(after[..e].to_string());
            cursor = &after[e + CUSTOM_END_MARKER.len()..];
        } else {
            break;
        }
    }
    blocks
}

pub fn write_markdown_preserving_custom(dest_path: &Path, new_template: &str) -> Result<()> {
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let final_text = if dest_path.exists() {
        let existing = fs::read_to_string(dest_path).unwrap_or_default();
        merge_preserving_custom_blocks(&existing, new_template)
    } else {
        new_template.to_string()
    };

    fs::write(dest_path, final_text)
        .with_context(|| format!("Failed to write {}", dest_path.display()))?;
    Ok(())
}

pub fn run_update(args: &UpdateArgs) -> Result<usize> {
    let base_dir: PathBuf = match args.scope {
        ConfigScope::Project => args.path.clone(),
        ConfigScope::Global => dirs::home_dir().unwrap_or_else(|| args.path.clone()),
    };

    let mut updated_files = 0usize;
    let rules = list_embedded_markdown("rules")?;
    let agents = list_embedded_markdown("agents")?;
    let commands = list_embedded_markdown("commands")?;

    for harness_dir in [".claude", ".gemini"] {
        let root = base_dir.join(harness_dir);
        if root.exists() {
            for (name, content) in &rules {
                write_markdown_preserving_custom(&root.join("rules").join(name), content)?;
                updated_files += 1;
            }
            for (name, content) in &agents {
                write_markdown_preserving_custom(&root.join("agents").join(name), content)?;
                updated_files += 1;
            }
            for (name, content) in &commands {
                write_markdown_preserving_custom(&root.join("commands").join(name), content)?;
                updated_files += 1;
            }
            let skills_written = extract_embedded_subdir("skills", &root.join("skills"))?;
            updated_files += skills_written.len();
        }
    }

    // Update universal AGENTS.md and .cursorrules if they exist
    for doc_name in ["AGENTS.md", ".cursorrules", "CLAUDE.md", "GEMINI.md"] {
        let doc_path = base_dir.join(doc_name);
        if doc_path.exists() {
            let mut combined = String::from(
                "# ops-ecc Universal Full-Stack + DevOps & DevSecOps Agent Harness

                 <!-- USER_CUSTOM_START -->
                 <!-- Add project-specific instructions here. Preserved across `ops-ecc update`. -->
                 <!-- USER_CUSTOM_END -->

",
            );
            for (_, content) in &rules {
                combined.push_str(content);
                combined.push_str("

");
            }
            write_markdown_preserving_custom(&doc_path, &combined)?;
            updated_files += 1;
        }
    }

    println!(
        "{} Updated {} harness files in `{}` (all `USER_CUSTOM` blocks preserved).",
        "[ops-ecc]".bold().green(),
        updated_files,
        base_dir.display()
    );

    Ok(updated_files)
}
