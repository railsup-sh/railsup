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
    detect_shell_from_env(env::var("SHELL").ok())
}

/// Extract shell name from SHELL env value (testable helper)
fn detect_shell_from_env(shell_var: Option<String>) -> String {
    shell_var
        .and_then(|s| s.rsplit('/').next().map(String::from))
        .filter(|s| !s.is_empty())
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
    use std::path::PathBuf;

    // ==================== detect_shell_from_env tests ====================

    #[test]
    fn detect_shell_extracts_zsh() {
        assert_eq!(detect_shell_from_env(Some("/bin/zsh".to_string())), "zsh");
    }

    #[test]
    fn detect_shell_extracts_fish() {
        assert_eq!(
            detect_shell_from_env(Some("/usr/local/bin/fish".to_string())),
            "fish"
        );
    }

    #[test]
    fn detect_shell_extracts_bash() {
        assert_eq!(detect_shell_from_env(Some("/bin/bash".to_string())), "bash");
    }

    #[test]
    fn detect_shell_handles_homebrew_path() {
        assert_eq!(
            detect_shell_from_env(Some("/opt/homebrew/bin/zsh".to_string())),
            "zsh"
        );
    }

    #[test]
    fn detect_shell_defaults_to_bash_when_none() {
        assert_eq!(detect_shell_from_env(None), "bash");
    }

    #[test]
    fn detect_shell_defaults_to_bash_when_empty() {
        assert_eq!(detect_shell_from_env(Some("".to_string())), "bash");
    }

    // ==================== generate_posix tests ====================

    #[test]
    fn generate_posix_includes_version_comment() {
        let output = generate_posix(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("Ruby 4.0.1"));
    }

    #[test]
    fn generate_posix_includes_important_warning() {
        let output = generate_posix(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("IMPORTANT"));
        assert!(output.contains("rbenv/asdf/rvm"));
    }

    #[test]
    fn generate_posix_exports_path() {
        let output = generate_posix(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("export PATH="));
        assert!(output.contains("/home/user/.railsup/ruby/4.0.1/bin"));
        assert!(output.contains("/home/user/.railsup/gems/4.0.1/bin"));
    }

    #[test]
    fn generate_posix_exports_gem_home() {
        let output = generate_posix(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("export GEM_HOME="));
        assert!(output.contains("/home/user/.railsup/gems/4.0.1"));
    }

    #[test]
    fn generate_posix_exports_gem_path() {
        let output = generate_posix(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("export GEM_PATH="));
    }

    #[test]
    fn generate_posix_includes_eval_instruction() {
        let output = generate_posix(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("eval \"$(railsup shell-init)\""));
    }

    // ==================== generate_fish tests ====================

    #[test]
    fn generate_fish_includes_version_comment() {
        let output = generate_fish(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("Ruby 4.0.1"));
    }

    #[test]
    fn generate_fish_includes_important_warning() {
        let output = generate_fish(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("IMPORTANT"));
        assert!(output.contains("rbenv/asdf/rvm"));
    }

    #[test]
    fn generate_fish_sets_path_with_correct_syntax() {
        let output = generate_fish(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("set -gx PATH"));
        assert!(output.contains("/home/user/.railsup/ruby/4.0.1/bin"));
    }

    #[test]
    fn generate_fish_sets_gem_home() {
        let output = generate_fish(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("set -gx GEM_HOME"));
    }

    #[test]
    fn generate_fish_sets_gem_path() {
        let output = generate_fish(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("set -gx GEM_PATH"));
    }

    #[test]
    fn generate_fish_includes_source_instruction() {
        let output = generate_fish(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        assert!(output.contains("railsup shell-init | source"));
    }

    #[test]
    fn generate_fish_does_not_use_export_keyword() {
        let output = generate_fish(
            "4.0.1",
            &PathBuf::from("/home/user/.railsup/ruby/4.0.1/bin"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1"),
            &PathBuf::from("/home/user/.railsup/gems/4.0.1/bin"),
        );
        // Fish uses 'set -gx', not 'export'
        assert!(!output.contains("export "));
    }
}
