use crate::ruby;
use crate::util::{process, ui};
use anyhow::{bail, Result};
use std::path::Path;

const RAILS_VERSION: &str = "8.1.2";

pub fn run(name: &str, force: bool) -> Result<()> {
    // 1. Validate name - reject path separators for safety
    validate_app_name(name)?;

    // 2. Detect Ruby
    let _ruby_info = ruby::detect()?;

    // 3. Check directory doesn't exist
    let path = Path::new(name);
    if path.exists() && !force {
        bail!(
            "Directory '{}' already exists. Use --force to overwrite.",
            name
        );
    }

    // 4. Ensure Rails gem is installed
    ensure_rails_installed()?;

    // 5. Run rails new using ruby -S for reliable shim resolution
    ui::info(&format!("Creating Rails {} app...", RAILS_VERSION));

    let rails_version_arg = format!("_{}_", RAILS_VERSION);
    let status = process::run_streaming(
        "ruby",
        &[
            "-S",
            "rails",
            &rails_version_arg,
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
            RAILS_VERSION,
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

fn ensure_rails_installed() -> Result<()> {
    // Check if rails gem at correct version exists
    let output = process::run_capture("gem", &["list", "rails", "-i", "-v", RAILS_VERSION])?;

    if output.trim() == "true" {
        return Ok(());
    }

    // Install Rails
    ui::info(&format!("Installing Rails {}...", RAILS_VERSION));
    let status = process::run_streaming(
        "gem",
        &["install", "rails", "-v", RAILS_VERSION, "--no-document"],
        None,
    )?;

    if !status.success() {
        bail!(
            "Failed to install Rails {}.\n  \
             Try running manually: gem install rails -v {}",
            RAILS_VERSION,
            RAILS_VERSION
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
