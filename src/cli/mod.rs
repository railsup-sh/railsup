use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "railsup")]
#[command(about = "The better way to install and run Ruby on Rails")]
#[command(version)]
pub struct Cli {
    /// Output context for AI agents (what railsup is, how to use it)
    #[arg(long)]
    pub agent: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new Rails application
    New {
        /// Name of the application
        name: String,

        /// Overwrite existing directory
        #[arg(short, long)]
        force: bool,

        /// Additional arguments forwarded to `rails new`
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        rails_args: Vec<String>,
    },

    /// Start the development server
    Dev {
        /// Port to run on
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },

    /// Manage Ruby versions
    #[command(subcommand)]
    Ruby(ruby::RubyCommands),

    /// Show path to a command (ruby, gem, bundle, rails, etc.)
    Which {
        /// Command to find (ruby, gem, bundle, rails, rake, irb)
        command: String,
    },

    /// Run a command with railsup Ruby environment
    Exec {
        /// Ruby version to use (default: auto-detect)
        #[arg(long)]
        ruby: Option<String>,

        /// Command and arguments to run
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },

    /// Output shell integration script for PATH setup
    ShellInit {
        /// Shell type (zsh, bash, fish). Auto-detected if not specified.
        #[arg(long)]
        shell: Option<String>,
    },

    /// Diagnose environment and troubleshoot issues
    Doctor {
        /// Output as JSON (suppresses AI analysis)
        #[arg(long)]
        json: bool,

        /// Attempt automatic fixes with confirmation
        #[arg(long)]
        fix: bool,

        /// Show all checks, not just issues
        #[arg(long, short)]
        verbose: bool,
    },
}

pub mod agent;
pub mod bundler;
pub mod dev;
pub mod doctor;
pub mod exec;
pub mod new;
pub mod ruby;
pub mod shell_init;
pub mod which;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_accepts_forwarded_rails_args() {
        let cli = Cli::try_parse_from([
            "railsup",
            "new",
            "myapp",
            "--database=postgresql",
            "--skip-git",
        ])
        .expect("should parse forwarded rails args");

        match cli.command {
            Some(Commands::New {
                name,
                force,
                rails_args,
            }) => {
                assert_eq!(name, "myapp");
                assert!(!force);
                assert_eq!(rails_args, vec!["--database=postgresql", "--skip-git"]);
            }
            _ => panic!("expected new command"),
        }
    }
}
