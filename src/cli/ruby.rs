//! Ruby version management commands
//!
//! railsup ruby install <version>
//! railsup ruby list [--available]
//! railsup ruby default <version>
//! railsup ruby remove <version>

use crate::{config::Config, download, paths, util::ui};
use anyhow::{bail, Result};
use clap::Subcommand;
use std::fs;

/// Default Ruby version for auto-bootstrap
pub const DEFAULT_RUBY_VERSION: &str = "4.0.1";

/// Available Ruby versions (built and hosted on GitHub Releases)
pub const AVAILABLE_VERSIONS: &[&str] = &["4.0.1", "3.4.8"];

#[derive(Subcommand)]
pub enum RubyCommands {
    /// Install a Ruby version
    Install {
        /// Ruby version to install (e.g., 4.0.1)
        version: String,

        /// Force reinstall even if already installed
        #[arg(short, long)]
        force: bool,
    },

    /// List installed Ruby versions
    List {
        /// Show available versions for download
        #[arg(long)]
        available: bool,
    },

    /// Set the default Ruby version
    Default {
        /// Ruby version to set as default
        version: String,
    },

    /// Remove an installed Ruby version
    Remove {
        /// Ruby version to remove
        version: String,
    },

    /// Clear the download cache
    ClearCache,
}

/// Handle Ruby subcommands
pub fn run(cmd: RubyCommands) -> Result<()> {
    match cmd {
        RubyCommands::Install { version, force } => install(&version, force),
        RubyCommands::List { available } => list(available),
        RubyCommands::Default { version } => set_default(&version),
        RubyCommands::Remove { version } => remove(&version),
        RubyCommands::ClearCache => clear_cache(),
    }
}

/// Install a Ruby version
fn install(version: &str, force: bool) -> Result<()> {
    let version = if version == "latest" {
        DEFAULT_RUBY_VERSION
    } else {
        version
    };

    ui::info(&format!("Installing Ruby {}...", version));

    // Download and extract
    download::download_ruby(version, force)?;

    ui::success(&format!("Ruby {} installed successfully", version));

    // Set as default if it's the first/only version
    let installed = list_installed_versions()?;
    if installed.len() == 1 {
        let mut config = Config::load()?;
        config.set_default_ruby(version);
        config.save()?;
        println!("  Set as default Ruby version");
    }

    Ok(())
}

/// List installed or available Ruby versions
fn list(show_available: bool) -> Result<()> {
    if show_available {
        println!("Available Ruby versions:");
        for version in AVAILABLE_VERSIONS {
            println!("  {}", version);
        }
        return Ok(());
    }

    let installed = list_installed_versions()?;

    if installed.is_empty() {
        println!("No Ruby versions installed.");
        println!("Run: railsup ruby install {}", DEFAULT_RUBY_VERSION);
        return Ok(());
    }

    let config = Config::load()?;
    let default_version = config.default_ruby();

    println!("Installed Ruby versions:");
    for version in &installed {
        if Some(version.as_str()) == default_version {
            println!("  {} (default)", version);
        } else {
            println!("  {}", version);
        }
    }

    Ok(())
}

/// Set the default Ruby version
fn set_default(version: &str) -> Result<()> {
    // Check if version is installed
    let version_dir = paths::ruby_version_dir(version);
    if !version_dir.exists() {
        bail!(
            "Ruby {} is not installed.\nRun: railsup ruby install {}",
            version,
            version
        );
    }

    let mut config = Config::load()?;
    config.set_default_ruby(version);
    config.save()?;

    ui::success(&format!("Default Ruby version set to {}", version));
    Ok(())
}

/// Remove an installed Ruby version
fn remove(version: &str) -> Result<()> {
    let version_dir = paths::ruby_version_dir(version);
    if !version_dir.exists() {
        bail!("Ruby {} is not installed", version);
    }

    // Remove Ruby directory
    ui::info(&format!("Removing Ruby {}...", version));
    fs::remove_dir_all(&version_dir)?;

    // Remove gems directory if it exists
    let gems_dir = paths::gems_version_dir(version);
    if gems_dir.exists() {
        fs::remove_dir_all(&gems_dir)?;
    }

    // Check if this was the default and warn user
    let config = Config::load()?;
    if config.default_ruby() == Some(version) {
        println!("  Note: This was the default version. Set a new default with:");
        println!("    railsup ruby default <version>");
    }

    ui::success(&format!("Ruby {} removed", version));
    Ok(())
}

/// Clear the download cache
fn clear_cache() -> Result<()> {
    let cache_dir = paths::cache_dir();

    if !cache_dir.exists() {
        println!("Cache is already empty.");
        return Ok(());
    }

    let mut count = 0;
    let mut total_size: u64 = 0;

    for entry in fs::read_dir(&cache_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Ok(metadata) = path.metadata() {
                total_size += metadata.len();
            }
            fs::remove_file(&path)?;
            count += 1;
        }
    }

    if count == 0 {
        println!("Cache is already empty.");
    } else {
        let size_mb = total_size as f64 / 1024.0 / 1024.0;
        ui::success(&format!("Cleared {} cached file(s) ({:.1} MB)", count, size_mb));
    }

    Ok(())
}

/// List all installed Ruby versions, sorted by version (newest first)
/// Returns version numbers without the "ruby-" prefix (e.g., "3.3.5" not "ruby-3.3.5")
pub fn list_installed_versions() -> Result<Vec<String>> {
    let ruby_dir = paths::ruby_dir();
    if !ruby_dir.exists() {
        return Ok(vec![]);
    }

    let mut versions = vec![];
    for entry in fs::read_dir(ruby_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden directories
            if name.starts_with('.') {
                continue;
            }
            // Strip "ruby-" prefix if present to normalize version names
            let version = if let Some(stripped) = name.strip_prefix("ruby-") {
                stripped.to_string()
            } else {
                name
            };
            versions.push(version);
        }
    }

    // Sort by version (descending)
    versions.sort_by(|a, b| compare_versions(b, a));
    Ok(versions)
}

/// Compare two version strings (simple semver comparison)
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<u32> = a.split('.').filter_map(|p| p.parse().ok()).collect();
    let b_parts: Vec<u32> = b.split('.').filter_map(|p| p.parse().ok()).collect();

    for (av, bv) in a_parts.iter().zip(b_parts.iter()) {
        match av.cmp(bv) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    a_parts.len().cmp(&b_parts.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_versions_works() {
        use std::cmp::Ordering;
        assert_eq!(compare_versions("4.0.1", "4.0.0"), Ordering::Greater);
        assert_eq!(compare_versions("4.0.0", "4.0.1"), Ordering::Less);
        assert_eq!(compare_versions("4.0.1", "4.0.1"), Ordering::Equal);
        assert_eq!(compare_versions("4.1.0", "4.0.9"), Ordering::Greater);
        assert_eq!(compare_versions("5.0.0", "4.9.9"), Ordering::Greater);
    }
}
