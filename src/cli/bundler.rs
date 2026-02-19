//! Bundler integration for railsup
//!
//! This module implements PEP-0016 (Gem Isolation Strategy):
//! - Bundle context detection (Gemfile within Rails root)
//! - Command wrapping (binstubs preferred, then bundle exec)
//! - Consistent Ruby environment setup
//!
//! All CLI commands (dev, exec, new, etc.) use these shared functions
//! to ensure consistent behavior.

use crate::paths;
use crate::util::tls;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Bundle context for a project
#[derive(Debug, Clone)]
pub struct BundleContext {
    /// The Rails root directory (contains config/application.rb)
    pub rails_root: PathBuf,
    /// Path to the Gemfile
    pub gemfile: PathBuf,
    /// Path to the Gemfile.lock (if exists)
    pub lockfile: Option<PathBuf>,
}

impl BundleContext {
    /// Check if a binstub exists for the given command
    pub fn has_binstub(&self, command: &str) -> bool {
        self.rails_root.join("bin").join(command).is_file()
    }

    /// Get the binstub path for a command
    #[allow(dead_code)]
    pub fn binstub_path(&self, command: &str) -> PathBuf {
        self.rails_root.join("bin").join(command)
    }

    /// Parse BUNDLED WITH version from Gemfile.lock
    #[allow(dead_code)]
    pub fn bundled_with_version(&self) -> Option<String> {
        let lockfile = self.lockfile.as_ref()?;
        let content = std::fs::read_to_string(lockfile).ok()?;

        // BUNDLED WITH is the last section in Gemfile.lock
        // Format:
        // BUNDLED WITH
        //    2.5.6
        let lines: Vec<&str> = content.lines().collect();
        let mut found_bundled_with = false;

        for line in &lines {
            let trimmed = line.trim();
            if found_bundled_with && !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
            if trimmed == "BUNDLED WITH" {
                found_bundled_with = true;
            }
        }
        None
    }
}

/// Detect bundle context starting from a directory
///
/// Algorithm (per PEP-0016):
/// 1. Find Rails root by walking up looking for config/application.rb
/// 2. Check for Gemfile within the Rails root (not above it - monorepo safety)
/// 3. Return BundleContext if both exist
pub fn detect_bundle_context(start_dir: &Path) -> Option<BundleContext> {
    // Step 1: Find Rails root
    let rails_root = find_rails_root(start_dir)?;

    // Step 2: Check for Gemfile within Rails root (not above)
    let gemfile = rails_root.join("Gemfile");
    if !gemfile.exists() {
        return None;
    }

    // Step 3: Check for lockfile
    let lockfile_path = rails_root.join("Gemfile.lock");
    let lockfile = if lockfile_path.exists() {
        Some(lockfile_path)
    } else {
        None
    };

    Some(BundleContext {
        rails_root,
        gemfile,
        lockfile,
    })
}

/// Find Rails root by walking up from start directory
/// Returns the directory containing config/application.rb
pub fn find_rails_root(start: &Path) -> Option<PathBuf> {
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

/// Commands that should be wrapped with bundle exec when bare
const WRAPPABLE_COMMANDS: &[&str] = &[
    "rails", "rake", "ruby", "rspec", "sidekiq", "puma", "spring", "foreman",
];

/// Check if bundle wrapping is disabled via RAILSUP_NO_BUNDLE=1
pub fn is_bundle_opt_out() -> bool {
    std::env::var("RAILSUP_NO_BUNDLE").ok().as_deref() == Some("1")
}

/// Wrap a command according to PEP-0016 rules
///
/// Rules:
/// 1. Check RAILSUP_NO_BUNDLE=1 env var - skip wrapping if set
/// 2. If no bundle context, no wrapping
/// 3. If command is already "bundle" or starts with "bin/", don't wrap
/// 4. If binstub exists for command, use binstub
/// 5. Otherwise, wrap with bundle exec
pub fn wrap_command(
    bundle_ctx: &Option<BundleContext>,
    command: &str,
    args: &[String],
) -> (String, Vec<String>) {
    // RULE 0: Check opt-out env var
    if is_bundle_opt_out() {
        return (command.to_string(), args.to_vec());
    }

    // If no bundle context, no wrapping
    let ctx = match bundle_ctx {
        Some(ctx) => ctx,
        None => return (command.to_string(), args.to_vec()),
    };

    // RULE 1: Already wrapped — don't wrap again
    if command == "bundle" {
        return (command.to_string(), args.to_vec());
    }
    if command.starts_with("bin/") {
        return (command.to_string(), args.to_vec());
    }

    // RULE 2: Use binstub if it exists (for rails and rake)
    if (command == "rails" || command == "rake") && ctx.has_binstub(command) {
        let binstub = format!("bin/{}", command);
        return (binstub, args.to_vec());
    }

    // RULE 3: Wrap with bundle exec
    let mut new_args = vec!["exec".to_string(), command.to_string()];
    new_args.extend(args.iter().cloned());
    ("bundle".to_string(), new_args)
}

/// Wrap a Procfile command string according to PEP-0016 rules
///
/// Special handling for Procfile commands:
/// - Skip if already starts with "bundle" or "bin/"
/// - Wrap bare commands (rails, rake, ruby, etc.) with bundle exec
/// - Don't wrap unknown commands (might be system commands)
/// - Handle common patterns: KEY=VAL prefixes, exec prefix
pub fn wrap_procfile_command(bundle_ctx: &Option<BundleContext>, command_string: &str) -> String {
    // Check opt-out
    if is_bundle_opt_out() {
        return command_string.to_string();
    }

    // If no bundle context, no wrapping
    if bundle_ctx.is_none() {
        return command_string.to_string();
    }

    // Find the actual command by skipping:
    // 1. Environment variable assignments (KEY=VAL)
    // 2. Optional "exec" prefix
    let tokens: Vec<&str> = command_string.split_whitespace().collect();
    let mut cmd_index = 0;

    // Skip KEY=VAL assignments
    while cmd_index < tokens.len() && tokens[cmd_index].contains('=') {
        cmd_index += 1;
    }

    // Skip optional "exec" prefix
    if cmd_index < tokens.len() && tokens[cmd_index] == "exec" {
        cmd_index += 1;
    }

    // No command found after prefixes
    if cmd_index >= tokens.len() {
        return command_string.to_string();
    }

    let actual_command = tokens[cmd_index];

    // Skip if already bundled or uses binstub
    if actual_command == "bundle" {
        return command_string.to_string();
    }
    if actual_command.starts_with("bin/") {
        return command_string.to_string();
    }

    // Wrap bare commands that are known Ruby/Rails commands
    if WRAPPABLE_COMMANDS.contains(&actual_command) {
        // Insert "bundle exec" after any env vars but before exec/command
        let env_prefix: Vec<&str> = tokens[..cmd_index].to_vec();
        let cmd_suffix: Vec<&str> = tokens[cmd_index..].to_vec();

        if env_prefix.is_empty() {
            return format!("bundle exec {}", cmd_suffix.join(" "));
        } else {
            return format!(
                "{} bundle exec {}",
                env_prefix.join(" "),
                cmd_suffix.join(" ")
            );
        }
    }

    // Unknown command — don't wrap (might be system command like nginx, postgres)
    command_string.to_string()
}

/// Path separator for the current platform (`:` on Unix, `;` on Windows)
#[cfg(unix)]
const PATH_SEPARATOR: char = ':';
#[cfg(windows)]
const PATH_SEPARATOR: char = ';';

/// Build environment with railsup Ruby paths
///
/// This is the canonical way to set up Ruby environment.
/// All CLI commands should use this for consistency.
pub fn build_ruby_env(version: &str) -> HashMap<String, String> {
    let ruby_bin = paths::ruby_bin_dir(version);
    let gem_home = paths::gems_version_dir(version);
    let gem_bin = gem_home.join("bin");

    // Start with current environment
    let mut env: HashMap<String, String> = std::env::vars().collect();

    // Prepend our Ruby bin AND gem bin to PATH
    let current_path = env.get("PATH").cloned().unwrap_or_default();
    let new_path = format!(
        "{}{}{}{}{}",
        ruby_bin.display(),
        PATH_SEPARATOR,
        gem_bin.display(),
        PATH_SEPARATOR,
        current_path
    );
    env.insert("PATH".into(), new_path);

    // Set GEM_HOME and GEM_PATH to our directories
    env.insert("GEM_HOME".into(), gem_home.display().to_string());
    env.insert("GEM_PATH".into(), gem_home.display().to_string());

    // Clear problematic variables that could interfere
    env.remove("RUBYOPT");
    env.remove("RUBYLIB");

    // Ensure TLS cert paths are valid so HTTPS calls (Ruby/OpenSSL) work reliably.
    let (cert_file, cert_dir) = tls::recommended_cert_env(
        env.get("SSL_CERT_FILE").map(String::as_str),
        env.get("SSL_CERT_DIR").map(String::as_str),
    );
    if let Some(path) = cert_file {
        env.insert("SSL_CERT_FILE".into(), path);
    }
    if let Some(path) = cert_dir {
        env.insert("SSL_CERT_DIR".into(), path);
    }

    env
}

/// Build full environment including bundle context
///
/// Combines Ruby env setup with Bundler-specific variables.
/// Respects RAILSUP_NO_BUNDLE=1 opt-out - won't set BUNDLE_GEMFILE if opt-out is active.
pub fn build_full_env(
    ruby_version: &str,
    bundle_ctx: &Option<BundleContext>,
) -> HashMap<String, String> {
    let mut env = build_ruby_env(ruby_version);

    // If we have bundle context and opt-out is not active, set BUNDLE_GEMFILE
    if !is_bundle_opt_out() {
        if let Some(ctx) = bundle_ctx {
            env.insert("BUNDLE_GEMFILE".into(), ctx.gemfile.display().to_string());
        }
    }

    env
}

/// Check if bundle install is needed (missing Gemfile.lock)
pub fn needs_bundle_install(bundle_ctx: &BundleContext) -> bool {
    bundle_ctx.lockfile.is_none()
}

/// Get installed bundler version from gem list
#[allow(dead_code)]
pub fn get_installed_bundler_version(ruby_bin: &Path) -> Option<String> {
    let gem_path = ruby_bin.join("gem");
    let output = std::process::Command::new(&gem_path)
        .args(["list", "bundler", "--exact"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output format: "bundler (2.5.6)"
    // Extract version between parentheses
    let start = stdout.find('(')? + 1;
    let end = stdout.find(')')?;
    Some(stdout[start..end].to_string())
}

/// Check for bundler version mismatch and return warning message if any
pub fn check_bundler_version_mismatch(
    bundle_ctx: &BundleContext,
    ruby_bin: &Path,
) -> Option<String> {
    let required = bundle_ctx.bundled_with_version()?;
    let installed = get_installed_bundler_version(ruby_bin)?;

    // Compare major.minor.patch - Bundler is usually compatible across patches
    let required_parts: Vec<&str> = required.split('.').collect();
    let installed_parts: Vec<&str> = installed.split('.').collect();

    // Only warn if major or minor version differs
    if required_parts.len() >= 2
        && installed_parts.len() >= 2
        && (required_parts[0] != installed_parts[0] || required_parts[1] != installed_parts[1])
    {
        return Some(format!(
            "Bundler version mismatch detected.\n\
             Gemfile.lock requires bundler {}, but {} is installed.\n\
             To fix: railsup exec gem install bundler:{}",
            required, installed, required
        ));
    }

    None
}

/// Format the "bundle detected" message for user output
pub fn format_bundle_detected_message(bundle_ctx: &BundleContext) -> String {
    format!(
        "Detected Gemfile → using project bundle ({})",
        bundle_ctx.rails_root.display()
    )
}

/// Check if an error message indicates missing gems
/// Returns a helpful hint message if so
#[allow(dead_code)]
pub fn check_missing_gems_error(stderr: &str) -> Option<String> {
    // Common Bundler error patterns for missing gems
    let missing_patterns = [
        "Could not find gem",
        "could not find gem",
        "Bundler could not find compatible versions",
        "Your bundle is locked to",
        "Run `bundle install`",
        "Make sure the gem is installed",
    ];

    for pattern in &missing_patterns {
        if stderr.contains(pattern) {
            return Some("Gems may be missing. Run: railsup exec bundle install".to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::tempdir;

    /// Mutex to serialize tests that modify environment variables
    /// This prevents race conditions when tests run in parallel
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    // ==================== detect_bundle_context tests ====================

    #[test]
    fn detect_bundle_context_in_rails_project() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile.lock"), "").unwrap();

        let result = detect_bundle_context(dir.path());
        assert!(result.is_some());

        let ctx = result.unwrap();
        assert_eq!(ctx.rails_root, dir.path());
        assert!(ctx.lockfile.is_some());
    }

    #[test]
    fn detect_bundle_context_no_lockfile() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let result = detect_bundle_context(dir.path());
        assert!(result.is_some());

        let ctx = result.unwrap();
        assert!(ctx.lockfile.is_none());
    }

    #[test]
    fn detect_bundle_context_no_gemfile() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        // No Gemfile

        let result = detect_bundle_context(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn detect_bundle_context_not_rails() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();
        // No config/application.rb

        let result = detect_bundle_context(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn detect_bundle_context_from_subdirectory() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::create_dir_all(dir.path().join("app/models")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let subdir = dir.path().join("app/models");
        let result = detect_bundle_context(&subdir);
        assert!(result.is_some());
        assert_eq!(result.unwrap().rails_root, dir.path());
    }

    #[test]
    fn monorepo_uses_app_gemfile_not_parent() {
        let dir = tempdir().unwrap();

        // Parent with Gemfile (should be ignored)
        std::fs::write(dir.path().join("Gemfile"), "# parent").unwrap();

        // Nested Rails app
        let app_path = dir.path().join("apps/myapp");
        std::fs::create_dir_all(app_path.join("config")).unwrap();
        std::fs::write(app_path.join("config/application.rb"), "").unwrap();
        std::fs::write(app_path.join("Gemfile"), "# app").unwrap();

        let result = detect_bundle_context(&app_path);
        assert!(result.is_some());

        let ctx = result.unwrap();
        assert_eq!(ctx.rails_root, app_path);
        assert_eq!(ctx.gemfile, app_path.join("Gemfile"));
    }

    // ==================== wrap_command tests ====================

    #[test]
    fn wrap_command_no_context() {
        let result = wrap_command(&None, "rails", &["server".to_string()]);
        assert_eq!(result.0, "rails");
        assert_eq!(result.1, vec!["server"]);
    }

    #[test]
    fn wrap_command_already_bundle() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        let result = wrap_command(&ctx, "bundle", &["exec".to_string(), "rails".to_string()]);
        assert_eq!(result.0, "bundle");
    }

    #[test]
    fn wrap_command_already_binstub_path() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        let result = wrap_command(&ctx, "bin/rails", &["server".to_string()]);
        assert_eq!(result.0, "bin/rails");
    }

    #[test]
    fn wrap_command_uses_binstub_when_exists() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::create_dir_all(dir.path().join("bin")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();
        std::fs::write(dir.path().join("bin/rails"), "#!/bin/sh").unwrap();

        let ctx = detect_bundle_context(dir.path());
        let result = wrap_command(&ctx, "rails", &["server".to_string()]);
        assert_eq!(result.0, "bin/rails");
        assert_eq!(result.1, vec!["server"]);
    }

    #[test]
    fn wrap_command_bundle_exec_fallback() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();
        // No binstubs

        let ctx = detect_bundle_context(dir.path());
        let result = wrap_command(&ctx, "rails", &["server".to_string()]);
        assert_eq!(result.0, "bundle");
        assert_eq!(result.1, vec!["exec", "rails", "server"]);
    }

    // ==================== wrap_procfile_command tests ====================

    #[test]
    fn wrap_procfile_already_bundle() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        let result = wrap_procfile_command(&ctx, "bundle exec rails server");
        assert_eq!(result, "bundle exec rails server");
    }

    #[test]
    fn wrap_procfile_already_binstub() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        let result = wrap_procfile_command(&ctx, "bin/rails server -p 3000");
        assert_eq!(result, "bin/rails server -p 3000");
    }

    #[test]
    fn wrap_procfile_bare_rails() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        let result = wrap_procfile_command(&ctx, "rails server -p 3000");
        assert_eq!(result, "bundle exec rails server -p 3000");
    }

    #[test]
    fn wrap_procfile_unknown_command() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        let result = wrap_procfile_command(&ctx, "nginx -c /etc/nginx.conf");
        // Unknown command should not be wrapped
        assert_eq!(result, "nginx -c /etc/nginx.conf");
    }

    // ==================== bundled_with_version tests ====================

    #[test]
    fn parse_bundled_with_version() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();
        std::fs::write(
            dir.path().join("Gemfile.lock"),
            "GEM\n  remote: https://rubygems.org/\n  specs:\n\nBUNDLED WITH\n   2.5.6\n",
        )
        .unwrap();

        let ctx = detect_bundle_context(dir.path()).unwrap();
        assert_eq!(ctx.bundled_with_version(), Some("2.5.6".to_string()));
    }

    // ==================== build_ruby_env tests ====================

    #[test]
    fn build_ruby_env_sets_gem_home() {
        let env = build_ruby_env("4.0.1");
        let gem_home = env.get("GEM_HOME").unwrap();
        assert!(gem_home.contains(".railsup/gems/4.0.1"));
    }

    #[test]
    fn build_ruby_env_prepends_path() {
        let env = build_ruby_env("4.0.1");
        let path = env.get("PATH").unwrap();
        assert!(path.contains(".railsup/ruby/ruby-4.0.1/bin"));
        assert!(path.contains(".railsup/gems/4.0.1/bin"));
    }

    #[test]
    fn build_ruby_env_clears_rubyopt() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("RUBYOPT", "-rbundler/setup");
        let env = build_ruby_env("4.0.1");
        assert!(!env.contains_key("RUBYOPT"));
        std::env::remove_var("RUBYOPT");
    }

    // ==================== build_full_env tests ====================

    #[test]
    fn build_full_env_sets_bundle_gemfile() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        let env = build_full_env("4.0.1", &ctx);

        assert!(env.get("BUNDLE_GEMFILE").is_some());
        assert!(env.get("BUNDLE_GEMFILE").unwrap().ends_with("Gemfile"));
    }

    // ==================== needs_bundle_install tests ====================

    #[test]
    fn needs_bundle_install_when_no_lockfile() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path()).unwrap();
        assert!(needs_bundle_install(&ctx));
    }

    #[test]
    fn no_bundle_install_needed_with_lockfile() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile.lock"), "").unwrap();

        let ctx = detect_bundle_context(dir.path()).unwrap();
        assert!(!needs_bundle_install(&ctx));
    }

    // ==================== Procfile realistic patterns tests ====================

    #[test]
    fn wrap_procfile_with_env_var_prefix() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        let result = wrap_procfile_command(&ctx, "PORT=3000 rails server");
        assert_eq!(result, "PORT=3000 bundle exec rails server");
    }

    #[test]
    fn wrap_procfile_with_exec_prefix() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        // exec prefix is preserved - it's used to replace shell process
        let result = wrap_procfile_command(&ctx, "exec rails server");
        assert_eq!(result, "exec bundle exec rails server");
    }

    #[test]
    fn wrap_procfile_with_env_and_exec_prefix() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        // exec prefix is preserved along with env vars
        let result =
            wrap_procfile_command(&ctx, "PORT=3000 RAILS_ENV=production exec rails server");
        assert_eq!(
            result,
            "PORT=3000 RAILS_ENV=production exec bundle exec rails server"
        );
    }

    #[test]
    fn wrap_procfile_with_multiple_env_vars() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        let result = wrap_procfile_command(
            &ctx,
            "PORT=3000 RAILS_ENV=development rails server -b 0.0.0.0",
        );
        assert_eq!(
            result,
            "PORT=3000 RAILS_ENV=development bundle exec rails server -b 0.0.0.0"
        );
    }

    #[test]
    fn wrap_procfile_exec_with_binstub_unchanged() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        // exec with binstub should not be wrapped
        let result = wrap_procfile_command(&ctx, "exec bin/rails server");
        assert_eq!(result, "exec bin/rails server");
    }

    #[test]
    fn wrap_procfile_sidekiq() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());
        let result = wrap_procfile_command(&ctx, "sidekiq -C config/sidekiq.yml");
        assert_eq!(result, "bundle exec sidekiq -C config/sidekiq.yml");
    }

    // ==================== opt-out tests ====================

    #[test]
    fn wrap_command_respects_opt_out() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());

        // Set opt-out
        std::env::set_var("RAILSUP_NO_BUNDLE", "1");

        // Should not wrap even with bundle context
        let result = wrap_command(&ctx, "rails", &["server".to_string()]);
        assert_eq!(result.0, "rails");
        assert_eq!(result.1, vec!["server"]);

        // Clean up
        std::env::remove_var("RAILSUP_NO_BUNDLE");
    }

    #[test]
    fn wrap_procfile_respects_opt_out() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());

        // Set opt-out
        std::env::set_var("RAILSUP_NO_BUNDLE", "1");

        // Should not wrap even with bundle context
        let result = wrap_procfile_command(&ctx, "rails server -p 3000");
        assert_eq!(result, "rails server -p 3000");

        // Clean up
        std::env::remove_var("RAILSUP_NO_BUNDLE");
    }

    #[test]
    fn build_full_env_respects_opt_out() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("config/application.rb"), "").unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();

        let ctx = detect_bundle_context(dir.path());

        // Set opt-out
        std::env::set_var("RAILSUP_NO_BUNDLE", "1");

        // BUNDLE_GEMFILE should NOT be set when opt-out is active
        let env = build_full_env("4.0.1", &ctx);
        assert!(env.get("BUNDLE_GEMFILE").is_none());

        // Clean up
        std::env::remove_var("RAILSUP_NO_BUNDLE");
    }

    // ==================== check_missing_gems_error tests ====================

    #[test]
    fn check_missing_gems_detects_could_not_find() {
        let stderr = "Could not find gem 'rails' in locally installed gems.";
        let hint = check_missing_gems_error(stderr);
        assert!(hint.is_some());
        assert!(hint.unwrap().contains("bundle install"));
    }

    #[test]
    fn check_missing_gems_detects_run_bundle_install() {
        let stderr = "Run `bundle install` to install missing gems.";
        let hint = check_missing_gems_error(stderr);
        assert!(hint.is_some());
    }

    #[test]
    fn check_missing_gems_returns_none_for_other_errors() {
        let stderr = "SyntaxError: unexpected end of input";
        let hint = check_missing_gems_error(stderr);
        assert!(hint.is_none());
    }
}
