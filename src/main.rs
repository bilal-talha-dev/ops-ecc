use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use ops_ecc::cli::{Cli, Commands};
use std::io;
use std::process;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => {
            ops_ecc::init::run_init(args)?;
        }
        Commands::Doctor(args) => {
            ops_ecc::doctor::run_doctor(&args)?;
        }
        Commands::Audit(args) => {
            let summary = ops_ecc::scanner::run_audit(&args)?;
            if args.strict && !summary.findings.is_empty() {
                process::exit(1);
            }
        }
        Commands::Hook { event } => {
            let outcome = ops_ecc::hook::execute_hook(&event)?;
            if !outcome.allowed {
                process::exit(outcome.exit_code);
            }
        }
        Commands::Scaffold(args) => {
            ops_ecc::scaffold::run_scaffold(&args)?;
        }
        Commands::Update(args) => {
            ops_ecc::update::run_update(&args)?;
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, bin_name, &mut io::stdout());
        }
    }

    Ok(())
}
