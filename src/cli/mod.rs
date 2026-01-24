use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "railsup")]
#[command(about = "The better way to install and run Ruby on Rails")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
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
}

pub mod dev;
pub mod new;
pub mod ruby;
pub mod which;
