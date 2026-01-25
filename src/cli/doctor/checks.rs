//! Diagnostic checks for the doctor command

use super::report::*;
use crate::{config::Config, paths};
use anyhow::Result;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Collect all diagnostics into a report
pub fn collect_diagnostics() -> Result<DiagnosticReport> {
    let ruby_versions = list_ruby_versions()?;
    let ruby_status = get_ruby_status(&ruby_versions)?;
    let shell_integration = detect_shell_integration();
    let conflicts = detect_conflicts(&shell_integration);
    let path_analysis = analyze_path(&ruby_status);

    Ok(DiagnosticReport {
        railsup_version: env!("CARGO_PKG_VERSION").to_string(),
        installation: check_installation(),
        ruby_status,
        ruby_versions,
        shell_integration,
        conflicts,
        path_analysis,
        environment: check_environment(),
        project: analyze_project(),
    })
}

/// Check railsup installation health
fn check_installation() -> InstallationHealth {
    let binary_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("railsup"));
    let config_dir = paths::railsup_dir();
    let ruby_dir = paths::ruby_dir();
    let gems_dir = paths::gems_dir();
    let cache_dir = paths::cache_dir();

    let all_healthy = config_dir.exists() || ruby_dir.exists();

    InstallationHealth {
        binary_path,
        config_dir,
        ruby_dir,
        gems_dir,
        cache_dir,
        all_healthy,
    }
}

/// List installed Ruby versions
fn list_ruby_versions() -> Result<Vec<RubyVersionInfo>> {
    let ruby_dir = paths::ruby_dir();
    if !ruby_dir.exists() {
        return Ok(vec![]);
    }

    let config = Config::load()?;
    let default_version = config.default_ruby().map(|s| s.to_string());

    let mut versions = vec![];
    for entry in fs::read_dir(ruby_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }

            let version = name.strip_prefix("ruby-").unwrap_or(&name).to_string();
            let is_default = Some(&version) == default_version.as_ref();

            versions.push(RubyVersionInfo {
                version: version.clone(),
                path: entry.path(),
                is_default,
            });
        }
    }

    // Sort by version descending
    versions.sort_by(|a, b| b.version.cmp(&a.version));
    Ok(versions)
}

/// Get Ruby installation status summary
fn get_ruby_status(versions: &[RubyVersionInfo]) -> Result<RubyStatus> {
    let default_version = Config::load()
        .ok()
        .and_then(|c| c.default_ruby().map(|s| s.to_string()));

    Ok(RubyStatus {
        any_installed: !versions.is_empty(),
        default_set: default_version.is_some(),
        default_version,
        installed_count: versions.len(),
    })
}

/// Detect shell integration status
fn detect_shell_integration() -> ShellIntegrationStatus {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            return ShellIntegrationStatus {
                configured: false,
                shell_file: None,
                line_number: None,
                placement: ShellInitPlacement::NotFound,
            }
        }
    };

    // Check common shell config files
    let shell_files = [".zshrc", ".bashrc", ".bash_profile"];

    for file in shell_files {
        let path = home.join(file);
        if let Some(status) = check_file_for_shell_init(&path) {
            return status;
        }

        // Check sourced files within the main config
        if let Some(status) = check_sourced_files(&path, &home) {
            return status;
        }
    }

    ShellIntegrationStatus {
        configured: false,
        shell_file: None,
        line_number: None,
        placement: ShellInitPlacement::NotFound,
    }
}

/// Check files that are sourced from a shell config
fn check_sourced_files(
    config_path: &PathBuf,
    home: &std::path::Path,
) -> Option<ShellIntegrationStatus> {
    let content = fs::read_to_string(config_path).ok()?;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // Look for source commands: source file, . file, or [[ -f file ]] && source file
        let sourced_file = extract_sourced_file(trimmed, home);
        if let Some(path) = sourced_file {
            if let Some(status) = check_file_for_shell_init(&path) {
                return Some(status);
            }
        }
    }

    None
}

/// Extract the file path from a source command
fn extract_sourced_file(line: &str, home: &std::path::Path) -> Option<PathBuf> {
    // Match patterns like:
    // source ~/.dotfiles/file
    // source "$HOME/.dotfiles/file"
    // . ~/.dotfiles/file
    // [[ -f ~/.file ]] && source ~/.file

    let patterns = ["source ", ". ", "&& source ", "&& . "];

    for pattern in patterns {
        if let Some(idx) = line.find(pattern) {
            let after = &line[idx + pattern.len()..];
            let path_str = after
                .split_whitespace()
                .next()?
                .trim_matches('"')
                .trim_matches('\'');

            return expand_path(path_str, home);
        }
    }

    None
}

/// Expand ~ and $HOME in paths
fn expand_path(path: &str, home: &std::path::Path) -> Option<PathBuf> {
    let expanded = if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else if let Some(rest) = path.strip_prefix("$HOME/") {
        home.join(rest)
    } else if let Some(rest) = path.strip_prefix("${HOME}/") {
        home.join(rest)
    } else if path.starts_with('/') {
        PathBuf::from(path)
    } else {
        // Relative path - skip
        return None;
    };

    if expanded.exists() {
        Some(expanded)
    } else {
        None
    }
}

/// Check a single file for shell-init configuration
fn check_file_for_shell_init(path: &PathBuf) -> Option<ShellIntegrationStatus> {
    let content = fs::read_to_string(path).ok()?;

    let mut railsup_line: Option<usize> = None;
    let mut last_version_manager_line: Option<usize> = None;

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // Check for railsup shell-init
        if trimmed.contains("railsup shell-init") {
            railsup_line = Some(i + 1);
        }

        // Check for version manager inits
        if trimmed.contains("rbenv init")
            || trimmed.contains("asdf.sh")
            || trimmed.contains(".asdf/asdf.sh")
            || trimmed.contains("rvm.sh")
            || trimmed.contains("rvm/scripts/rvm")
            || trimmed.contains("mise activate")
            || trimmed.contains("chruby.sh")
        {
            last_version_manager_line = Some(i + 1);
        }
    }

    match (railsup_line, last_version_manager_line) {
        (Some(r), Some(v)) if r > v => Some(ShellIntegrationStatus {
            configured: true,
            shell_file: Some(path.clone()),
            line_number: Some(r),
            placement: ShellInitPlacement::AfterVersionManagers,
        }),
        (Some(r), Some(v)) if r < v => Some(ShellIntegrationStatus {
            configured: true,
            shell_file: Some(path.clone()),
            line_number: Some(r),
            placement: ShellInitPlacement::BeforeVersionManagers,
        }),
        (Some(r), None) => Some(ShellIntegrationStatus {
            configured: true,
            shell_file: Some(path.clone()),
            line_number: Some(r),
            placement: ShellInitPlacement::NoVersionManagers,
        }),
        (None, _) => None,
        _ => None,
    }
}

/// Detect version manager conflicts
fn detect_conflicts(shell_integration: &ShellIntegrationStatus) -> Vec<Conflict> {
    let home = dirs::home_dir().unwrap_or_default();
    let path_env = env::var("PATH").unwrap_or_default();
    let path_entries: Vec<&str> = path_env.split(':').collect();

    let railsup_active = matches!(
        shell_integration.placement,
        ShellInitPlacement::AfterVersionManagers | ShellInitPlacement::NoVersionManagers
    );

    let mut conflicts = vec![];

    // Check rbenv
    let rbenv_dir = home.join(".rbenv");
    let rbenv_exists = rbenv_dir.exists();
    let rbenv_in_path = path_entries.iter().position(|p| p.contains(".rbenv/shims"));
    conflicts.push(Conflict {
        tool: "rbenv".to_string(),
        detected: rbenv_exists,
        location: if rbenv_exists { Some(rbenv_dir) } else { None },
        in_path: rbenv_in_path.is_some(),
        path_position: rbenv_in_path,
        impact: if !rbenv_exists {
            ConflictImpact::None
        } else if railsup_active {
            ConflictImpact::Overridden
        } else if rbenv_in_path.is_some() {
            ConflictImpact::Blocking
        } else {
            ConflictImpact::None
        },
    });

    // Check asdf
    let asdf_dir = home.join(".asdf");
    let asdf_exists = asdf_dir.exists();
    let asdf_in_path = path_entries.iter().position(|p| p.contains(".asdf/shims"));
    conflicts.push(Conflict {
        tool: "asdf".to_string(),
        detected: asdf_exists,
        location: if asdf_exists { Some(asdf_dir) } else { None },
        in_path: asdf_in_path.is_some(),
        path_position: asdf_in_path,
        impact: if !asdf_exists {
            ConflictImpact::None
        } else if railsup_active {
            ConflictImpact::Overridden
        } else if asdf_in_path.is_some() {
            ConflictImpact::Blocking
        } else {
            ConflictImpact::None
        },
    });

    // Check rvm
    let rvm_dir = home.join(".rvm");
    let rvm_exists = rvm_dir.exists();
    let rvm_in_path = path_entries.iter().position(|p| p.contains(".rvm"));
    conflicts.push(Conflict {
        tool: "rvm".to_string(),
        detected: rvm_exists,
        location: if rvm_exists { Some(rvm_dir) } else { None },
        in_path: rvm_in_path.is_some(),
        path_position: rvm_in_path,
        impact: if !rvm_exists {
            ConflictImpact::None
        } else if railsup_active {
            ConflictImpact::Overridden
        } else if rvm_in_path.is_some() {
            ConflictImpact::Blocking
        } else {
            ConflictImpact::None
        },
    });

    // Check mise
    let mise_dir = home.join(".local/share/mise");
    let mise_exists = mise_dir.exists();
    let mise_in_path = path_entries.iter().position(|p| p.contains("mise/shims"));
    conflicts.push(Conflict {
        tool: "mise".to_string(),
        detected: mise_exists,
        location: if mise_exists { Some(mise_dir) } else { None },
        in_path: mise_in_path.is_some(),
        path_position: mise_in_path,
        impact: if !mise_exists {
            ConflictImpact::None
        } else if railsup_active {
            ConflictImpact::Overridden
        } else if mise_in_path.is_some() {
            ConflictImpact::Blocking
        } else {
            ConflictImpact::None
        },
    });

    // Check Homebrew Ruby (macOS)
    #[cfg(target_os = "macos")]
    {
        let homebrew_ruby = PathBuf::from("/opt/homebrew/opt/ruby");
        let homebrew_in_path = path_entries
            .iter()
            .position(|p| p.contains("/opt/homebrew/opt/ruby"));
        if homebrew_ruby.exists() || homebrew_in_path.is_some() {
            conflicts.push(Conflict {
                tool: "Homebrew Ruby".to_string(),
                detected: homebrew_ruby.exists(),
                location: if homebrew_ruby.exists() {
                    Some(homebrew_ruby)
                } else {
                    None
                },
                in_path: homebrew_in_path.is_some(),
                path_position: homebrew_in_path,
                impact: if railsup_active {
                    ConflictImpact::Overridden
                } else if homebrew_in_path.is_some() {
                    ConflictImpact::Blocking
                } else {
                    ConflictImpact::None
                },
            });
        }
    }

    conflicts
}

/// Analyze PATH for Ruby-related entries
fn analyze_path(ruby_status: &RubyStatus) -> PathAnalysis {
    let path_env = env::var("PATH").unwrap_or_default();
    let path_entries: Vec<&str> = path_env.split(':').collect();

    let mut entries = vec![];
    for (i, path_str) in path_entries.iter().enumerate() {
        let path = PathBuf::from(path_str);
        let source = classify_path_source(path_str);
        entries.push(PathEntry {
            path,
            position: i,
            source,
        });
    }

    // Find which ruby
    let which_ruby = which::which("ruby").ok();
    let which_gem = which::which("gem").ok();
    let which_bundle = which::which("bundle").ok();

    // Expected ruby path
    let expected_ruby = if let Some(ref version) = ruby_status.default_version {
        paths::ruby_bin_dir(version).join("ruby")
    } else {
        PathBuf::from("~/.railsup/ruby/ruby-VERSION/bin/ruby")
    };

    // Check if ruby is correct
    let ruby_correct = which_ruby
        .as_ref()
        .is_some_and(|p| p.to_string_lossy().contains(".railsup/ruby/"));

    // Check if gem_bin is in PATH
    let gem_bin_in_path = entries
        .iter()
        .any(|e| e.path.to_string_lossy().contains(".railsup/gems/"));

    PathAnalysis {
        entries,
        which_ruby,
        which_gem,
        which_bundle,
        expected_ruby,
        ruby_correct,
        gem_bin_in_path,
    }
}

/// Classify a PATH entry by its source
fn classify_path_source(path: &str) -> PathSource {
    if path.contains(".railsup/ruby/") {
        PathSource::Railsup
    } else if path.contains(".railsup/gems/") {
        PathSource::RailsupGems
    } else if path.contains(".rbenv") {
        PathSource::Rbenv
    } else if path.contains(".asdf") {
        PathSource::Asdf
    } else if path.contains(".rvm") {
        PathSource::Rvm
    } else if path.contains("mise") {
        PathSource::Mise
    } else if path.contains("/opt/homebrew") || path.contains("/usr/local/Cellar") {
        PathSource::Homebrew
    } else if path.starts_with("/usr/") || path.starts_with("/bin") {
        PathSource::System
    } else {
        PathSource::Unknown
    }
}

/// Check environment variables for issues
fn check_environment() -> EnvironmentCheck {
    let gem_home = env::var("GEM_HOME").ok();
    let gem_path = env::var("GEM_PATH").ok();
    let rubyopt = env::var("RUBYOPT").ok();
    let rubylib = env::var("RUBYLIB").ok();
    let bundle_path = env::var("BUNDLE_PATH").ok();

    let mut issues = vec![];

    // Check if GEM_HOME is set but not to railsup
    if let Some(ref gh) = gem_home {
        if !gh.contains(".railsup/gems/") {
            issues.push(format!("GEM_HOME={} (not railsup's)", gh));
        }
    }

    // Check if RUBYOPT is set (can cause issues)
    if let Some(ref ro) = rubyopt {
        issues.push(format!("RUBYOPT={} (may cause conflicts)", ro));
    }

    // Check if RUBYLIB is set
    if let Some(ref rl) = rubylib {
        issues.push(format!("RUBYLIB={} (may cause conflicts)", rl));
    }

    EnvironmentCheck {
        gem_home,
        gem_path,
        rubyopt,
        rubylib,
        bundle_path,
        issues,
    }
}

/// Analyze the current project (if in a Rails directory)
fn analyze_project() -> Option<ProjectAnalysis> {
    let current_dir = env::current_dir().ok()?;

    // Check if this looks like a Rails project
    let is_rails = current_dir.join("Gemfile").exists()
        && (current_dir.join("config.ru").exists() || current_dir.join("bin/rails").exists());

    if !is_rails {
        return None;
    }

    // Read .ruby-version
    let ruby_version_file = fs::read_to_string(current_dir.join(".ruby-version"))
        .ok()
        .map(|s| s.trim().trim_start_matches("ruby-").to_string());

    // Read railsup.toml
    let railsup_toml = fs::read_to_string(current_dir.join("railsup.toml"))
        .ok()
        .and_then(|content| {
            toml::from_str::<toml::Table>(&content)
                .ok()
                .and_then(|t| t.get("ruby").and_then(|v| v.as_str().map(String::from)))
        });

    // Read Gemfile ruby version (simple regex)
    let gemfile_ruby = fs::read_to_string(current_dir.join("Gemfile"))
        .ok()
        .and_then(|content| {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("ruby ") || trimmed.starts_with("ruby(") {
                    // Extract version from ruby "3.3.0" or ruby("3.3.0")
                    if let Some(start) = trimmed.find('"') {
                        if let Some(end) = trimmed[start + 1..].find('"') {
                            return Some(trimmed[start + 1..start + 1 + end].to_string());
                        }
                    }
                }
            }
            None
        });

    // Check if versions match
    let config = Config::load().ok();
    let default_version = config.and_then(|c| c.default_ruby().map(|s| s.to_string()));
    let project_version = railsup_toml
        .as_ref()
        .or(ruby_version_file.as_ref())
        .or(gemfile_ruby.as_ref());

    let version_match = match (project_version, &default_version) {
        (Some(pv), Some(dv)) => pv == dv,
        (None, _) => true, // No project version specified is OK
        (Some(_), None) => false,
    };

    Some(ProjectAnalysis {
        path: current_dir,
        is_rails,
        ruby_version_file,
        gemfile_ruby,
        railsup_toml,
        version_match,
    })
}
