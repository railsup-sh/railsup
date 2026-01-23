use crate::ruby;
use crate::util::{process, ui};
use anyhow::{bail, Result};
use std::env;
use std::path::{Path, PathBuf};

pub fn run(port: u16) -> Result<()> {
    // 1. Find Rails root
    let current_dir = env::current_dir()?;
    let rails_root = find_rails_root(&current_dir).ok_or_else(|| {
        anyhow::anyhow!("Not a Rails directory. Create one with: railsup new myapp")
    })?;

    // 2. Detect Ruby
    let _ruby_info = ruby::detect()?;

    // 3. Print startup message
    ui::info(&format!("Starting Rails on http://localhost:{}", port));

    // 4. Run rails server using current_dir (not env::set_current_dir)
    let port_str = port.to_string();
    let status = process::run_streaming(
        "ruby",
        &["-S", "bundle", "exec", "rails", "server", "-p", &port_str],
        Some(&rails_root),
    )?;

    if !status.success() {
        bail!(
            "Server exited with error.\n  \
             Try running manually: cd {} && bundle exec rails server",
            rails_root.display()
        );
    }

    Ok(())
}

/// Search upward from start directory to find Rails root.
/// Returns the directory containing config/application.rb, or None if not found.
fn find_rails_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        let marker = current.join("config/application.rb");
        if marker.exists() {
            return Some(current);
        }

        if !current.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn find_rails_root_in_project_dir() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();

        let result = find_rails_root(dir.path());
        assert_eq!(result, Some(dir.path().to_path_buf()));
    }

    #[test]
    fn find_rails_root_from_subdirectory() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::create_dir_all(dir.path().join("app/models")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();

        let subdir = dir.path().join("app/models");
        let result = find_rails_root(&subdir);
        assert_eq!(result, Some(dir.path().to_path_buf()));
    }

    #[test]
    fn find_rails_root_from_deep_subdirectory() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::create_dir_all(dir.path().join("app/controllers/concerns")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();

        let subdir = dir.path().join("app/controllers/concerns");
        let result = find_rails_root(&subdir);
        assert_eq!(result, Some(dir.path().to_path_buf()));
    }

    #[test]
    fn find_rails_root_not_found() {
        let dir = tempdir().unwrap();
        let result = find_rails_root(dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn find_rails_root_with_config_but_no_application_rb() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        // No application.rb file

        let result = find_rails_root(dir.path());
        assert_eq!(result, None);
    }
}
