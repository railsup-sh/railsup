//! Shell initialization - outputs shell config for PATH integration
//!
//! railsup shell-init [--shell zsh|bash|fish]
//!
//! Outputs shell configuration that adds railsup's Ruby to PATH.
//! Users add `eval "$(railsup shell-init)"` to their shell profile.

use crate::cli::ruby::list_installed_versions;
use crate::config::Config;
use crate::paths;
use anyhow::{bail, Result};
use std::env;
use std::path::Path;

/// Run the shell-init command
pub fn run(shell: Option<String>) -> Result<()> {
    let shell_type = shell.unwrap_or_else(detect_shell);
    let output = generate_init(&shell_type)?;
    println!("{}", output);
    Ok(())
}

/// Detect shell type from $SHELL environment variable
fn detect_shell() -> String {
    env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(String::from))
        .unwrap_or_else(|| "bash".to_string())
}

/// Resolve the default Ruby version to use
fn resolve_default_version() -> Result<String> {
    // 1. Check global default
    if let Ok(config) = Config::load() {
        if let Some(default) = config.default_ruby() {
            let version_dir = paths::ruby_version_dir(default);
            if version_dir.exists() {
                return Ok(default.to_string());
            }
        }
    }

    // 2. Use latest installed
    let installed = list_installed_versions()?;
    if let Some(version) = installed.first() {
        return Ok(version.clone());
    }

    // 3. No Ruby installed
    bail!(
        "No Ruby version installed.\n\n\
         Install Ruby first:\n  \
         railsup ruby install 4.0.1\n\n\
         Then add shell integration:\n  \
         eval \"$(railsup shell-init)\""
    )
}

/// Generate shell initialization script
fn generate_init(shell: &str) -> Result<String> {
    let version = resolve_default_version()?;
    let ruby_bin = paths::ruby_bin_dir(&version);
    let gem_home = paths::gems_version_dir(&version);
    let gem_bin = gem_home.join("bin");

    match shell {
        "fish" => Ok(generate_fish(&version, &ruby_bin, &gem_home, &gem_bin)),
        _ => Ok(generate_posix(&version, &ruby_bin, &gem_home, &gem_bin)),
    }
}

/// Generate POSIX-compatible shell script (bash, zsh)
fn generate_posix(version: &str, ruby_bin: &Path, gem_home: &Path, gem_bin: &Path) -> String {
    format!(
        r#"# Railsup shell integration (Ruby {version})
# Add to your ~/.zshrc or ~/.bashrc:
#   eval "$(railsup shell-init)"
#
# IMPORTANT: Place this AFTER any rbenv/asdf/rvm initialization
# to ensure railsup takes precedence.

export PATH="{ruby_bin}:{gem_bin}:$PATH"
export GEM_HOME="{gem_home}"
export GEM_PATH="{gem_home}"
"#,
        version = version,
        ruby_bin = ruby_bin.display(),
        gem_bin = gem_bin.display(),
        gem_home = gem_home.display(),
    )
}

/// Generate fish shell script
fn generate_fish(version: &str, ruby_bin: &Path, gem_home: &Path, gem_bin: &Path) -> String {
    format!(
        r#"# Railsup shell integration (Ruby {version})
# Add to your ~/.config/fish/config.fish:
#   railsup shell-init | source
#
# IMPORTANT: Place this AFTER any rbenv/asdf/rvm initialization
# to ensure railsup takes precedence.

set -gx PATH {ruby_bin} {gem_bin} $PATH
set -gx GEM_HOME {gem_home}
set -gx GEM_PATH {gem_home}
"#,
        version = version,
        ruby_bin = ruby_bin.display(),
        gem_bin = gem_bin.display(),
        gem_home = gem_home.display(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_shell_extracts_shell_name() {
        std::env::set_var("SHELL", "/bin/zsh");
        assert_eq!(detect_shell(), "zsh");

        std::env::set_var("SHELL", "/usr/local/bin/fish");
        assert_eq!(detect_shell(), "fish");

        std::env::set_var("SHELL", "/bin/bash");
        assert_eq!(detect_shell(), "bash");
    }
}
