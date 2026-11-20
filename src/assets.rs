use anyhow::{anyhow, Context, Result};
use include_dir::{include_dir, Dir, DirEntry};
use std::fs;
use std::path::Path;

pub static TEMPLATES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// Recursively extract an embedded subdirectory from `TEMPLATES_DIR` into `dest_root`.
pub fn extract_embedded_subdir(subdir_path: &str, dest_root: &Path) -> Result<Vec<String>> {
    let dir = TEMPLATES_DIR
        .get_dir(subdir_path)
        .ok_or_else(|| anyhow!("Embedded template directory not found: {}", subdir_path))?;

    let mut written = Vec::new();
    extract_dir_recursive(dir, Path::new(subdir_path), dest_root, &mut written)?;
    Ok(written)
}

fn extract_dir_recursive(
    dir: &Dir<'_>,
    strip_prefix: &Path,
    dest_root: &Path,
    written: &mut Vec<String>,
) -> Result<()> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(sub) => {
                extract_dir_recursive(sub, strip_prefix, dest_root, written)?;
            }
            DirEntry::File(file) => {
                let rel = file
                    .path()
                    .strip_prefix(strip_prefix)
                    .unwrap_or_else(|_| file.path());
                let out_path = dest_root.join(rel);
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create parent directory: {}", parent.display())
                    })?;
                }
                fs::write(&out_path, file.contents())
                    .with_context(|| format!("Failed to write file: {}", out_path.display()))?;
                written.push(out_path.display().to_string());
            }
        }
    }
    Ok(())
}

/// Read an embedded file as UTF-8 text.
pub fn get_embedded_text(rel_path: &str) -> Result<&'static str> {
    let file = TEMPLATES_DIR
        .get_file(rel_path)
        .ok_or_else(|| anyhow!("Embedded file not found: {}", rel_path))?;
    file.contents_utf8()
        .ok_or_else(|| anyhow!("Embedded file is not valid UTF-8: {}", rel_path))
}

/// Collect all markdown files inside an embedded directory (`rules`, `agents`, `commands`).
pub fn list_embedded_markdown(subdir: &str) -> Result<Vec<(String, &'static str)>> {
    let dir = TEMPLATES_DIR
        .get_dir(subdir)
        .ok_or_else(|| anyhow!("Embedded directory not found: {}", subdir))?;
    let mut items = Vec::new();
    for file in dir.files() {
        if let Some(name) = file.path().file_name().and_then(|n| n.to_str()) {
            if let Some(text) = file.contents_utf8() {
                items.push((name.to_string(), text));
            }
        }
    }
    items.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(items)
}
