//! Exec command - run commands with railsup Ruby environment
//!
//! railsup exec <command> [args...]
//!
//! This bypasses other version managers (rbenv, asdf, rvm) by:
//! 1. Prepending railsup Ruby bin to PATH
//! 2. Setting GEM_HOME/GEM_PATH to railsup directories
//! 3. Clearing problematic env vars (RUBYOPT, RUBYLIB)
//!
//! Implements PEP-0016 (Gem Isolation Strategy):
//! - Detects bundle context (Gemfile within Rails root)
//! - Wraps commands with bundle exec or uses binstubs automatically

use crate::cli::bundler::{
    build_full_env, detect_bundle_context, format_bundle_detected_message, is_bundle_opt_out,
    wrap_command,
};
use crate::cli::which::resolve_ruby_version;
use crate::paths;
use crate::util::ui;
use anyhow::{bail, Result};

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

    // 3. Detect bundle context (PEP-0016)
    let current_dir = std::env::current_dir()?;
    let bundle_ctx = detect_bundle_context(&current_dir);

    // Show bundle detection message if in a Rails project (respects opt-out)
    if let Some(ref ctx) = bundle_ctx {
        if !is_bundle_opt_out() {
            ui::info(&format_bundle_detected_message(ctx));
        }
    }

    // 4. Apply command wrapping (PEP-0016)
    let program = &command[0];
    let args: Vec<String> = command[1..].to_vec();
    let (wrapped_program, wrapped_args) = wrap_command(&bundle_ctx, program, &args);

    // 5. Build environment with bundle context
    let env = build_full_env(&version, &bundle_ctx);

    // Set environment variables before exec
    for (key, value) in &env {
        std::env::set_var(key, value);
    }

    // Clear removed variables
    std::env::remove_var("RUBYOPT");
    std::env::remove_var("RUBYLIB");

    // 6. Resolve command path
    let cmd_path = if wrapped_program.starts_with("bin/") {
        // Binstub - resolve relative to Rails root
        if let Some(ref ctx) = bundle_ctx {
            ctx.rails_root.join(&wrapped_program).display().to_string()
        } else {
            wrapped_program.clone()
        }
    } else {
        wrapped_program.clone()
    };

    let err = exec::Command::new(&cmd_path).args(&wrapped_args).exec();

    // exec() only returns on error
    bail!("Failed to execute '{}': {}", cmd_path, err)
}

#[cfg(test)]
mod tests {
    use crate::cli::bundler::build_ruby_env;
    use std::sync::Mutex;

    /// Mutex to serialize tests that modify environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

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
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("RUBYOPT", "-rbundler/setup");
        let env = build_ruby_env("4.0.1");
        assert!(!env.contains_key("RUBYOPT"));
        std::env::remove_var("RUBYOPT");
    }

    #[test]
    fn build_ruby_env_clears_rubylib() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("RUBYLIB", "/some/path");
        let env = build_ruby_env("4.0.1");
        assert!(!env.contains_key("RUBYLIB"));
        std::env::remove_var("RUBYLIB");
    }
}
