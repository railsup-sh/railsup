mod cli;
mod config;
mod download;
mod paths;
mod platform;
mod ruby;
mod util;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    if let Err(e) = run() {
        util::ui::error(&format!("{:#}", e));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Handle --agent flag
    if cli.agent {
        cli::agent::run();
        return Ok(());
    }

    // Handle subcommands
    match cli.command {
        Some(Commands::New {
            name,
            force,
            rails_args,
        }) => cli::new::run(&name, force, &rails_args),
        Some(Commands::Dev { port }) => cli::dev::run(port),
        Some(Commands::Ruby(cmd)) => cli::ruby::run(cmd),
        Some(Commands::Which { command }) => cli::which::run(&command),
        Some(Commands::Exec { ruby, command }) => cli::exec::run(ruby, command),
        Some(Commands::ShellInit { shell }) => cli::shell_init::run(shell),
        Some(Commands::Doctor { json, fix, verbose }) => cli::doctor::run(json, fix, verbose),
        None => {
            // No command provided, show help
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
    }
}
