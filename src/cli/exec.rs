//! Exec command - run commands with railsup Ruby environment
//!
//! railsup exec <command> [args...]
//!
//! This bypasses other version managers (rbenv, asdf, rvm) by:
//! 1. Prepending railsup Ruby bin to PATH
//! 2. Setting GEM_HOME/GEM_PATH to railsup directories
//! 3. Clearing problematic env vars (RUBYOPT, RUBYLIB)

use crate::cli::which::resolve_ruby_version;
use crate::paths;
use anyhow::{bail, Result};
use std::collections::HashMap;

/// Run a command with railsup Ruby environment
pub fn run(ruby_version: Option<String>, command: Vec<String>) -> Result<()> {
    if command.is_empty() {
        bail!("No command specified.\nUsage: railsup exec <command> [args...]");
    }

    // 1. Resolve Ruby version
    let version = match ruby_version {
        Some(v) => v,
        None => resolve_ruby_version()?,
    };

    // 2. Verify Ruby is installed
    let ruby_bin = paths::ruby_bin_dir(&version);
    if !ruby_bin.exists() {
        bail!(
            "Ruby {} is not installed.\nRun: railsup ruby install {}",
            version,
            version
        );
    }

    // 3. Build environment
    let env = build_ruby_env(&version);

    // 4. Execute (replaces current process)
    let program = &command[0];
    let args = &command[1..];

    // Set environment variables before exec
    for (key, value) in &env {
        std::env::set_var(key, value);
    }

    // Clear removed variables
    std::env::remove_var("RUBYOPT");
    std::env::remove_var("RUBYLIB");

    let err = exec::Command::new(program).args(args).exec();

    // exec() only returns on error
    bail!("Failed to execute '{}': {}", program, err)
}

/// Build environment with railsup Ruby paths
fn build_ruby_env(version: &str) -> HashMap<String, String> {
    let ruby_bin = paths::ruby_bin_dir(version);
    let gem_home = paths::gems_version_dir(version);
    let gem_bin = gem_home.join("bin");

    // Start with current environment
    let mut env: HashMap<String, String> = std::env::vars().collect();

    // Prepend our Ruby bin AND gem bin to PATH
    let current_path = env.get("PATH").cloned().unwrap_or_default();
    let new_path = format!("{}:{}:{}", ruby_bin.display(), gem_bin.display(), current_path);
    env.insert("PATH".into(), new_path);

    // Set GEM_HOME and GEM_PATH to our directories
    env.insert("GEM_HOME".into(), gem_home.display().to_string());
    env.insert("GEM_PATH".into(), gem_home.display().to_string());

    // Clear problematic variables that could interfere
    env.remove("RUBYOPT");
    env.remove("RUBYLIB");

    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_ruby_env_prepends_path() {
        let env = build_ruby_env("4.0.1");
        let path = env.get("PATH").unwrap();
        // Should start with our Ruby bin
        assert!(path.contains(".railsup/ruby/ruby-4.0.1/bin"));
    }

    #[test]
    fn build_ruby_env_sets_gem_home() {
        let env = build_ruby_env("4.0.1");
        let gem_home = env.get("GEM_HOME").unwrap();
        assert!(gem_home.contains(".railsup/gems/4.0.1"));
    }

    #[test]
    fn build_ruby_env_clears_rubyopt() {
        std::env::set_var("RUBYOPT", "-rbundler/setup");
        let env = build_ruby_env("4.0.1");
        assert!(!env.contains_key("RUBYOPT"));
        std::env::remove_var("RUBYOPT");
    }

    #[test]
    fn build_ruby_env_clears_rubylib() {
        std::env::set_var("RUBYLIB", "/some/path");
        let env = build_ruby_env("4.0.1");
        assert!(!env.contains_key("RUBYLIB"));
        std::env::remove_var("RUBYLIB");
    }
}
