//! Configuration management for railsup
//!
//! Handles reading/writing ~/.railsup/config.toml

use crate::paths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Global railsup configuration
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub ruby: RubyConfig,
}

/// Ruby-specific configuration
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RubyConfig {
    /// Default Ruby version
    pub default: Option<String>,
}

impl Config {
    /// Load configuration from ~/.railsup/config.toml
    /// Returns default config if file doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = paths::config_file();

        if !config_path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))
    }

    /// Save configuration to ~/.railsup/config.toml
    pub fn save(&self) -> Result<()> {
        let config_path = paths::config_file();

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))
    }

    /// Get the default Ruby version
    pub fn default_ruby(&self) -> Option<&str> {
        self.ruby.default.as_deref()
    }

    /// Set the default Ruby version
    pub fn set_default_ruby(&mut self, version: &str) {
        self.ruby.default = Some(version.to_string());
    }
}

/// Project-level configuration from railsup.toml
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Ruby version for this project
    pub ruby: Option<String>,
}

impl ProjectConfig {
    /// Load project config from a directory
    #[allow(dead_code)] // Will be used in auto-bootstrap
    pub fn load_from_dir(dir: &Path) -> Result<Option<Self>> {
        let config_path = dir.join("railsup.toml");

        if !config_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read project config: {}", config_path.display()))?;

        let config: ProjectConfig = toml::from_str(&content).with_context(|| {
            format!("Failed to parse project config: {}", config_path.display())
        })?;

        Ok(Some(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_ruby() {
        let config = Config::default();
        assert!(config.default_ruby().is_none());
    }

    #[test]
    fn set_default_ruby_works() {
        let mut config = Config::default();
        config.set_default_ruby("4.0.1");
        assert_eq!(config.default_ruby(), Some("4.0.1"));
    }

    #[test]
    fn config_serialization_roundtrip() {
        let mut config = Config::default();
        config.set_default_ruby("4.0.1");

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let loaded: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(loaded.default_ruby(), Some("4.0.1"));
    }
}
