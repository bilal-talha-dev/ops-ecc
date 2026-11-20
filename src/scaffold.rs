use crate::assets::extract_embedded_subdir;
use crate::cli::ScaffoldArgs;
use anyhow::Result;
use owo_colors::OwoColorize;

pub fn run_scaffold(args: &ScaffoldArgs) -> Result<Vec<String>> {
    let subdir = format!("scaffolds/{}", args.template.dir_name());
    let written = extract_embedded_subdir(&subdir, &args.out)?;

    println!(
        "{} Scaffolded template `{}` into `{}` ({} files):",
        "[ops-ecc]".bold().green(),
        args.template.dir_name().cyan(),
        args.out.display(),
        written.len()
    );
    for path in &written {
        println!("  + {}", path);
    }
    Ok(written)
}
