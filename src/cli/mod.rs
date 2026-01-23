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
}

pub mod dev;
pub mod new;
