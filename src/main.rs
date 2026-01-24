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

    match cli.command {
        Commands::New { name, force } => cli::new::run(&name, force),
        Commands::Dev { port } => cli::dev::run(port),
        Commands::Ruby(cmd) => cli::ruby::run(cmd),
        Commands::Which { command } => cli::which::run(&command),
    }
}
