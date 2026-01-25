//! Data structures for the diagnostic report

use serde::Serialize;
use std::path::PathBuf;

/// Complete diagnostic report
#[derive(Debug, Serialize)]
pub struct DiagnosticReport {
    pub railsup_version: String,
    pub installation: InstallationHealth,
    pub ruby_status: RubyStatus,
    pub ruby_versions: Vec<RubyVersionInfo>,
    pub shell_integration: ShellIntegrationStatus,
    pub conflicts: Vec<Conflict>,
    pub path_analysis: PathAnalysis,
    pub environment: EnvironmentCheck,
    pub project: Option<ProjectAnalysis>,
}

/// Installation health status
#[derive(Debug, Serialize)]
pub struct InstallationHealth {
    pub binary_path: PathBuf,
    pub config_dir: PathBuf,
    pub ruby_dir: PathBuf,
    pub gems_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub all_healthy: bool,
}

/// Ruby installation status
#[derive(Debug, Serialize)]
pub struct RubyStatus {
    pub any_installed: bool,
    pub default_set: bool,
    pub default_version: Option<String>,
    pub installed_count: usize,
}

/// Information about an installed Ruby version
#[derive(Debug, Serialize)]
pub struct RubyVersionInfo {
    pub version: String,
    pub path: PathBuf,
    pub is_default: bool,
}

/// Shell integration status
#[derive(Debug, Serialize)]
pub struct ShellIntegrationStatus {
    pub configured: bool,
    pub shell_file: Option<PathBuf>,
    pub line_number: Option<usize>,
    pub placement: ShellInitPlacement,
}

/// Where shell-init is placed relative to other version managers
#[derive(Debug, Serialize, Clone)]
pub enum ShellInitPlacement {
    NotFound,
    BeforeVersionManagers, // Will be overridden - BAD
    AfterVersionManagers,  // Correct - GOOD
    NoVersionManagers,     // Only railsup - GOOD
}

/// Information about a detected version manager conflict
#[derive(Debug, Serialize)]
pub struct Conflict {
    pub tool: String,
    pub detected: bool,
    pub location: Option<PathBuf>,
    pub in_path: bool,
    pub path_position: Option<usize>,
    pub impact: ConflictImpact,
}

/// How much a conflict impacts railsup
#[derive(Debug, Serialize)]
pub enum ConflictImpact {
    None,       // Installed but not active
    Overridden, // Active but railsup takes precedence
    Blocking,   // Active and blocking railsup
}

/// PATH analysis results
#[derive(Debug, Serialize)]
pub struct PathAnalysis {
    pub entries: Vec<PathEntry>,
    pub which_ruby: Option<PathBuf>,
    pub which_gem: Option<PathBuf>,
    pub which_bundle: Option<PathBuf>,
    pub expected_ruby: PathBuf,
    pub ruby_correct: bool,
    pub gem_bin_in_path: bool,
}

/// A single PATH entry with classification
#[derive(Debug, Serialize)]
pub struct PathEntry {
    pub path: PathBuf,
    pub position: usize,
    pub source: PathSource,
}

/// Classification of a PATH entry source
#[derive(Debug, Serialize, Clone)]
pub enum PathSource {
    Railsup,
    RailsupGems,
    Rbenv,
    Asdf,
    Rvm,
    Mise,
    Homebrew,
    System,
    Unknown,
}

/// Environment variable check results
#[derive(Debug, Serialize)]
pub struct EnvironmentCheck {
    pub gem_home: Option<String>,
    pub gem_path: Option<String>,
    pub rubyopt: Option<String>,
    pub rubylib: Option<String>,
    pub bundle_path: Option<String>,
    pub issues: Vec<String>,
}

/// Project-specific analysis (when in a Rails directory)
#[derive(Debug, Serialize)]
pub struct ProjectAnalysis {
    pub path: PathBuf,
    pub is_rails: bool,
    pub ruby_version_file: Option<String>,
    pub gemfile_ruby: Option<String>,
    pub railsup_toml: Option<String>,
    pub version_match: bool,
}
