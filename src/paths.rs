//! Directory structure helpers for railsup
//!
//! ~/.railsup/
//! ├── ruby/           # Ruby installations
//! │   └── 4.0.1/
//! ├── gems/           # Per-version gems
//! │   └── 4.0.1/
//! ├── cache/          # Downloaded tarballs
//! └── config.toml     # Global config

use std::path::PathBuf;

/// Get the railsup home directory (~/.railsup)
pub fn railsup_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".railsup")
}

/// Get the Ruby installations directory (~/.railsup/ruby)
pub fn ruby_dir() -> PathBuf {
    railsup_dir().join("ruby")
}

/// Get the gems directory (~/.railsup/gems)
pub fn gems_dir() -> PathBuf {
    railsup_dir().join("gems")
}

/// Get the cache directory (~/.railsup/cache)
pub fn cache_dir() -> PathBuf {
    railsup_dir().join("cache")
}

/// Get the config file path (~/.railsup/config.toml)
pub fn config_file() -> PathBuf {
    railsup_dir().join("config.toml")
}

/// Get the directory for a specific Ruby version
/// The directory is named `ruby-{version}` (e.g., ruby-4.0.1)
pub fn ruby_version_dir(version: &str) -> PathBuf {
    // Handle both forms: "4.0.1" or "ruby-4.0.1"
    let dir_name = if version.starts_with("ruby-") {
        version.to_string()
    } else {
        format!("ruby-{}", version)
    };
    ruby_dir().join(dir_name)
}

/// Get the gems directory for a specific Ruby version
pub fn gems_version_dir(version: &str) -> PathBuf {
    gems_dir().join(version)
}

/// Get the bin directory for a specific Ruby version
pub fn ruby_bin_dir(version: &str) -> PathBuf {
    ruby_version_dir(version).join("bin")
}

/// Get the gems bin directory for a specific Ruby version
pub fn gems_bin_dir(version: &str) -> PathBuf {
    gems_version_dir(version).join("bin")
}

/// Ensure all railsup directories exist
pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(ruby_dir())?;
    std::fs::create_dir_all(gems_dir())?;
    std::fs::create_dir_all(cache_dir())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn railsup_dir_ends_with_railsup() {
        let path = railsup_dir();
        assert!(path.ends_with(".railsup"));
    }

    #[test]
    fn ruby_version_dir_includes_version() {
        let path = ruby_version_dir("4.0.1");
        // Directory should be named "ruby-4.0.1"
        assert!(path.ends_with("ruby-4.0.1"));
        assert!(path.to_string_lossy().contains(".railsup/ruby/"));
    }

    #[test]
    fn ruby_version_dir_handles_prefixed_input() {
        // If someone passes "ruby-4.0.1", don't double-prefix it
        let path = ruby_version_dir("ruby-4.0.1");
        assert!(path.ends_with("ruby-4.0.1"));
        assert!(!path.to_string_lossy().contains("ruby-ruby-"));
    }

    #[test]
    fn ruby_bin_dir_path() {
        let path = ruby_bin_dir("4.0.1");
        assert!(path.ends_with("bin"));
        assert!(path.to_string_lossy().contains("ruby-4.0.1"));
    }
}
