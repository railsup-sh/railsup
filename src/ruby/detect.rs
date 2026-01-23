use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RubyError {
    #[error("Ruby not found. Install with: {suggestion}")]
    NotFound { suggestion: String },

    #[error("Ruby 3.3+ required (found {found}). Upgrade with: {suggestion}")]
    VersionTooOld { found: String, suggestion: String },

    #[error("Failed to execute ruby: {0}")]
    ExecutionFailed(#[from] std::io::Error),

    #[error("Could not parse ruby version from: {0}")]
    ParseFailed(String),
}

#[allow(dead_code)]
pub struct RubyInfo {
    pub path: PathBuf,
    pub version: String,
}

/// Detect Ruby installation, validate version >= 3.3
pub fn detect() -> Result<RubyInfo, RubyError> {
    // 1. Find ruby in PATH
    let ruby_path = which::which("ruby").map_err(|_| RubyError::NotFound {
        suggestion: suggest_install(),
    })?;

    // 2. Get version
    let output = Command::new(&ruby_path).arg("--version").output()?;

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = parse_version(&version_str)?;

    // 3. Check minimum version
    if !meets_minimum(&version, "3.3") {
        return Err(RubyError::VersionTooOld {
            found: version,
            suggestion: suggest_install(),
        });
    }

    Ok(RubyInfo {
        path: ruby_path,
        version,
    })
}

/// Parse "ruby 3.3.0 (2023-12-25)..." -> "3.3.0"
fn parse_version(output: &str) -> Result<String, RubyError> {
    output
        .split_whitespace()
        .nth(1)
        .map(|v| {
            // Handle versions like "3.3.1p55" -> "3.3.1"
            v.split('p')
                .next()
                .unwrap_or(v)
                .trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.')
                .to_string()
        })
        .filter(|v| !v.is_empty())
        .ok_or_else(|| RubyError::ParseFailed(output.to_string()))
}

/// Check if version meets minimum requirement
fn meets_minimum(version: &str, minimum: &str) -> bool {
    let parts: Vec<u32> = version.split('.').filter_map(|p| p.parse().ok()).collect();
    let min_parts: Vec<u32> = minimum.split('.').filter_map(|p| p.parse().ok()).collect();

    for (v, m) in parts.iter().zip(min_parts.iter()) {
        if v > m {
            return true;
        }
        if v < m {
            return false;
        }
    }
    parts.len() >= min_parts.len()
}

/// Suggest install command based on detected version manager
fn suggest_install() -> String {
    if which::which("mise").is_ok() {
        "mise install ruby@3.3".to_string()
    } else if which::which("rbenv").is_ok() {
        "rbenv install 3.3.0".to_string()
    } else if which::which("asdf").is_ok() {
        "asdf install ruby 3.3.0".to_string()
    } else {
        // Default to mise as recommended option
        "mise install ruby@3.3".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_standard() {
        let v = parse_version("ruby 3.3.0 (2023-12-25 revision abc123)").unwrap();
        assert_eq!(v, "3.3.0");
    }

    #[test]
    fn parse_version_with_p() {
        let v = parse_version("ruby 3.3.1p55 (2024-01-15 revision def456)").unwrap();
        assert_eq!(v, "3.3.1");
    }

    #[test]
    fn parse_version_short() {
        let v = parse_version("ruby 3.4.0").unwrap();
        assert_eq!(v, "3.4.0");
    }

    #[test]
    fn parse_version_invalid() {
        let result = parse_version("not ruby output");
        assert!(result.is_err());
    }

    #[test]
    fn parse_version_empty() {
        let result = parse_version("");
        assert!(result.is_err());
    }

    #[test]
    fn meets_minimum_exact() {
        assert!(meets_minimum("3.3.0", "3.3"));
        assert!(meets_minimum("3.3", "3.3"));
    }

    #[test]
    fn meets_minimum_higher() {
        assert!(meets_minimum("3.4.0", "3.3"));
        assert!(meets_minimum("4.0.0", "3.3"));
        assert!(meets_minimum("3.3.5", "3.3"));
    }

    #[test]
    fn meets_minimum_lower() {
        assert!(!meets_minimum("3.2.0", "3.3"));
        assert!(!meets_minimum("3.1.4", "3.3"));
        assert!(!meets_minimum("2.7.0", "3.3"));
    }

    #[test]
    fn meets_minimum_edge_cases() {
        assert!(meets_minimum("3.3.0", "3.3.0"));
        assert!(!meets_minimum("3.2.9", "3.3.0"));
        assert!(meets_minimum("3.3.1", "3.3.0"));
    }
}
