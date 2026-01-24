use crate::cli::ruby::{list_installed_versions, DEFAULT_RUBY_VERSION};
use crate::cli::which::resolve_ruby_version;
use crate::{download, paths};
use crate::util::{process, ui};
use anyhow::{bail, Result};
use std::path::Path;

/// Fallback Rails version if we can't fetch from rubygems.org
const FALLBACK_RAILS_VERSION: &str = "8.1.2";

/// Rubygems API URL for Rails gem info
const RUBYGEMS_RAILS_URL: &str = "https://rubygems.org/api/v1/gems/rails.json";

/// Fetch the latest Rails version from rubygems.org
fn fetch_latest_rails_version() -> Option<String> {
    let response = ureq::get(RUBYGEMS_RAILS_URL)
        .timeout(std::time::Duration::from_secs(5))
        .call()
        .ok()?;

    let json: serde_json::Value = response.into_json().ok()?;
    json.get("version")?.as_str().map(|s| s.to_string())
}

/// Get the Rails version to use (fetched or fallback)
fn get_rails_version() -> String {
    fetch_latest_rails_version().unwrap_or_else(|| FALLBACK_RAILS_VERSION.to_string())
}

pub fn run(name: &str, force: bool) -> Result<()> {
    // 1. Validate name - reject path separators for safety
    validate_app_name(name)?;

    // 2. Ensure Ruby is available (auto-bootstrap if needed)
    let ruby_version = ensure_ruby_available()?;
    let ruby_bin = paths::ruby_bin_dir(&ruby_version);

    // 3. Check directory doesn't exist
    let path = Path::new(name);
    if path.exists() && !force {
        bail!(
            "Directory '{}' already exists. Use --force to overwrite.",
            name
        );
    }

    // 4. Get Rails version and ensure it's installed
    let rails_version = get_rails_version();
    ensure_rails_installed(&ruby_bin, &rails_version)?;

    // 5. Run rails new
    ui::info(&format!("Creating Rails {} app...", rails_version));

    // Use rails directly from our Ruby's bin to avoid PATH conflicts with rbenv/mise
    let rails_path = ruby_bin.join("rails");
    let rails_version_arg = format!("_{}_", rails_version);
    let status = process::run_streaming(
        rails_path.to_str().unwrap(),
        &[
            rails_version_arg.as_str(),
            "new",
            name,
            "--database=sqlite3",
            "--css=tailwind",
            "--javascript=importmap",
            "--skip-jbuilder",
            "--skip-action-mailbox",
            "--skip-action-text",
        ],
        None,
    )?;

    if !status.success() {
        bail!(
            "Failed to create Rails app. Try running manually:\n  \
             gem install rails -v {} && rails new {}",
            rails_version,
            name
        );
    }

    // 6. Print success
    println!();
    ui::success(&format!("Created {}", name));
    println!();
    println!("  cd {}", name);
    println!("  railsup dev");

    Ok(())
}

fn validate_app_name(name: &str) -> Result<()> {
    // Reject empty name
    if name.is_empty() {
        bail!("App name cannot be empty.");
    }

    // Reject "." - creating in current dir is dangerous with --force
    if name == "." {
        bail!(
            "Cannot create app in current directory.\n  \
             Use: railsup new myapp"
        );
    }

    // Reject path separators - prevents accidentally nuking wrong directory
    if name.contains('/') || name.contains("..") {
        bail!(
            "App name cannot contain path separators.\n  \
             Use a simple name like: railsup new myapp"
        );
    }

    // Reject names starting with - (would be interpreted as flags)
    if name.starts_with('-') {
        bail!("App name cannot start with a dash.");
    }

    Ok(())
}

/// Ensure Ruby is available, auto-bootstrapping if needed
pub fn ensure_ruby_available() -> Result<String> {
    // First, check if any railsup-managed Ruby is available
    match resolve_ruby_version() {
        Ok(version) => {
            ui::info(&format!("Using Ruby {}", version));
            return Ok(version);
        }
        Err(_) => {
            // No Ruby installed, auto-bootstrap
        }
    }

    // Check if there are any installed versions
    let installed = list_installed_versions()?;
    if !installed.is_empty() {
        let version = installed.first().unwrap().clone();
        ui::info(&format!("Using Ruby {}", version));
        return Ok(version);
    }

    // No Ruby installed, auto-bootstrap
    println!();
    ui::info(&format!(
        "No Ruby installed. Installing Ruby {}...",
        DEFAULT_RUBY_VERSION
    ));
    download::download_ruby(DEFAULT_RUBY_VERSION, false)?;
    ui::success(&format!("Ruby {} installed", DEFAULT_RUBY_VERSION));
    println!();

    Ok(DEFAULT_RUBY_VERSION.to_string())
}

fn ensure_rails_installed(ruby_bin: &Path, rails_version: &str) -> Result<()> {
    let gem_path = ruby_bin.join("gem");
    let gem_str = gem_path.to_str().unwrap();

    // Check if rails gem at correct version exists
    let output = process::run_capture(gem_str, &["list", "rails", "-i", "-v", rails_version])?;

    if output.trim() == "true" {
        return Ok(());
    }

    // Install Rails
    ui::info(&format!("Installing Rails {}...", rails_version));
    let status = process::run_streaming(
        gem_str,
        &["install", "rails", "-v", rails_version, "--no-document"],
        None,
    )?;

    if !status.success() {
        bail!(
            "Failed to install Rails {}.\n  \
             Try running manually: {} install rails -v {}",
            rails_version,
            gem_str,
            rails_version
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty() {
        assert!(validate_app_name("").is_err());
    }

    #[test]
    fn validate_rejects_dot() {
        let err = validate_app_name(".").unwrap_err();
        assert!(err.to_string().contains("current directory"));
    }

    #[test]
    fn validate_rejects_path_separators() {
        assert!(validate_app_name("foo/bar").is_err());
        assert!(validate_app_name("../foo").is_err());
        assert!(validate_app_name("foo/..").is_err());
    }

    #[test]
    fn validate_rejects_double_dots() {
        assert!(validate_app_name("..").is_err());
        assert!(validate_app_name("foo..bar").is_err());
    }

    #[test]
    fn validate_rejects_dash_prefix() {
        assert!(validate_app_name("-myapp").is_err());
        assert!(validate_app_name("--force").is_err());
    }

    #[test]
    fn validate_accepts_valid_names() {
        assert!(validate_app_name("myapp").is_ok());
        assert!(validate_app_name("my-app").is_ok());
        assert!(validate_app_name("my_app").is_ok());
        assert!(validate_app_name("MyApp").is_ok());
        assert!(validate_app_name("app123").is_ok());
        assert!(validate_app_name("123app").is_ok());
    }

    #[test]
    fn validate_accepts_dashes_in_middle() {
        assert!(validate_app_name("my-cool-app").is_ok());
        assert!(validate_app_name("app-v2").is_ok());
    }
}
