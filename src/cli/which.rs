//! Which command - show path to ruby/gem/bundle executables
//!
//! railsup which <command>

use crate::cli::ruby::list_installed_versions;
use crate::{config::Config, paths};
use anyhow::{bail, Result};
use std::env;
use std::path::Path;

/// Resolve which Ruby version to use
/// Priority: project config -> global default -> latest installed
pub fn resolve_ruby_version() -> Result<String> {
    // 1. Check current directory and parents for railsup.toml
    let current_dir = env::current_dir()?;
    if let Some(version) = find_project_ruby_version(&current_dir)? {
        let version_dir = paths::ruby_version_dir(&version);
        if version_dir.exists() {
            return Ok(version);
        }
        // Project specifies a version that isn't installed
        bail!(
            "Project requires Ruby {} but it's not installed.\nRun: railsup ruby install {}",
            version,
            version
        );
    }

    // 2. Check global default
    let config = Config::load()?;
    if let Some(default) = config.default_ruby() {
        let version_dir = paths::ruby_version_dir(default);
        if version_dir.exists() {
            return Ok(default.to_string());
        }
    }

    // 3. Use latest installed
    let installed = list_installed_versions()?;
    if let Some(version) = installed.first() {
        return Ok(version.clone());
    }

    // 4. No Ruby installed
    bail!("No Ruby version installed.\nRun: railsup ruby install 4.0.1")
}

/// Search up the directory tree for a railsup.toml with ruby version
fn find_project_ruby_version(start: &Path) -> Result<Option<String>> {
    let mut current = start.to_path_buf();

    loop {
        let config_path = current.join("railsup.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            if let Ok(config) = toml::from_str::<toml::Table>(&content) {
                if let Some(ruby) = config.get("ruby") {
                    if let Some(version) = ruby.as_str() {
                        return Ok(Some(version.to_string()));
                    }
                }
            }
        }

        if !current.pop() {
            return Ok(None);
        }
    }
}

/// Run the which command
pub fn run(command: &str) -> Result<()> {
    let version = resolve_ruby_version()?;
    let ruby_bin = paths::ruby_bin_dir(&version);
    let gems_bin = paths::gems_version_dir(&version).join("bin");

    let path = match command {
        "ruby" => ruby_bin.join("ruby"),
        "gem" => ruby_bin.join("gem"),
        "bundle" | "bundler" => ruby_bin.join("bundle"),
        "rake" => ruby_bin.join("rake"),
        "irb" => ruby_bin.join("irb"),
        "erb" => ruby_bin.join("erb"),
        "rdoc" => ruby_bin.join("rdoc"),
        "ri" => ruby_bin.join("ri"),
        // For rails and other gems, check gems bin first, then ruby bin
        "rails" => {
            let gem_path = gems_bin.join("rails");
            if gem_path.exists() {
                gem_path
            } else {
                ruby_bin.join("rails")
            }
        }
        other => {
            // Check gems bin first, then ruby bin
            let gem_path = gems_bin.join(other);
            if gem_path.exists() {
                gem_path
            } else {
                ruby_bin.join(other)
            }
        }
    };

    if path.exists() {
        println!("{}", path.display());
    } else {
        bail!(
            "{} not found for Ruby {}\nPath checked: {}",
            command,
            version,
            path.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_project_ruby_version_returns_none_for_empty_dir() {
        let temp = tempfile::tempdir().unwrap();
        let result = find_project_ruby_version(temp.path()).unwrap();
        assert!(result.is_none());
    }
}
