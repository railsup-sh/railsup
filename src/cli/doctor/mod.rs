//! Doctor command - diagnose environment and troubleshoot issues
//!
//! railsup doctor [--json] [--fix] [--verbose]

mod ai;
mod checks;
mod report;

use crate::util::ui;
use anyhow::Result;

/// Run the doctor command
pub fn run(json: bool, fix: bool, verbose: bool) -> Result<()> {
    // 1. Collect all diagnostics
    let report = checks::collect_diagnostics()?;

    // 2. Output report
    if json {
        // JSON mode: print and exit (no AI)
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    // Human-readable output
    print_report(&report, verbose);

    // 3. Auto-invoke AI if available (not in JSON mode)
    if ai::is_claude_available() {
        ai::stream_analysis(&report)?;
    }

    // 4. Handle --fix
    if fix {
        apply_fixes(&report)?;
    }

    Ok(())
}

/// Print the diagnostic report in human-readable format
fn print_report(report: &report::DiagnosticReport, verbose: bool) {
    println!();
    println!("Railsup Doctor");
    println!("{}", "═".repeat(50));
    println!();

    // Environment section
    println!("Environment");

    // Railsup version
    ui::success(&format!(
        "Railsup v{} at {}",
        report.railsup_version,
        report.installation.binary_path.display()
    ));

    // Ruby status
    if !report.ruby_status.any_installed {
        ui::error("No Ruby installed");
        println!("    Run: railsup ruby install 4.0.1");
    } else {
        ui::success(&format!(
            "Ruby versions installed: {}",
            report
                .ruby_versions
                .iter()
                .map(|v| v.version.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));

        if !report.ruby_status.default_set {
            ui::warn("No default Ruby set");
            if let Some(v) = report.ruby_versions.first() {
                println!("    Run: railsup ruby default {}", v.version);
            }
        } else if let Some(ref default) = report.ruby_status.default_version {
            ui::success(&format!("Default Ruby: {}", default));
        }
    }

    println!();

    // Shell Integration section
    println!("Shell Integration");
    match &report.shell_integration.placement {
        report::ShellInitPlacement::NotFound => {
            ui::error("shell-init not configured");
            if let Some(shell_file) = get_shell_config_file() {
                println!("    Add to {}: eval \"$(railsup shell-init)\"", shell_file);
            } else {
                println!("    Add to your shell config: eval \"$(railsup shell-init)\"");
            }
        }
        report::ShellInitPlacement::BeforeVersionManagers => {
            ui::warn("shell-init placed BEFORE version managers");
            if let Some(ref file) = report.shell_integration.shell_file {
                println!(
                    "    In {} line {} - move to END of file",
                    file.display(),
                    report.shell_integration.line_number.unwrap_or(0)
                );
            }
            println!("    Other version managers will override railsup");
        }
        report::ShellInitPlacement::AfterVersionManagers => {
            if let Some(ref file) = report.shell_integration.shell_file {
                ui::success(&format!(
                    "shell-init in {} (line {})",
                    file.display(),
                    report.shell_integration.line_number.unwrap_or(0)
                ));
            }
            ui::success("Placed AFTER version managers (correct)");
        }
        report::ShellInitPlacement::NoVersionManagers => {
            if let Some(ref file) = report.shell_integration.shell_file {
                ui::success(&format!(
                    "shell-init in {} (line {})",
                    file.display(),
                    report.shell_integration.line_number.unwrap_or(0)
                ));
            }
        }
    }

    println!();

    // Conflicts section
    let active_conflicts: Vec<_> = report.conflicts.iter().filter(|c| c.detected).collect();

    if !active_conflicts.is_empty() || verbose {
        println!("Conflicts");

        if active_conflicts.is_empty() {
            ui::success("No version managers detected");
        } else {
            for conflict in &active_conflicts {
                match conflict.impact {
                    report::ConflictImpact::None => {
                        if verbose {
                            ui::dim(&format!(
                                "{} installed at {} (not active)",
                                conflict.tool,
                                conflict
                                    .location
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_default()
                            ));
                        }
                    }
                    report::ConflictImpact::Overridden => {
                        ui::warn(&format!("{} detected", conflict.tool));
                        if let Some(ref loc) = conflict.location {
                            println!("    Installed at {}", loc.display());
                        }
                        if conflict.in_path {
                            println!(
                                "    In PATH at position {}",
                                conflict.path_position.unwrap_or(0)
                            );
                        }
                        println!("    railsup shell-init overrides (OK)");
                    }
                    report::ConflictImpact::Blocking => {
                        ui::error(&format!("{} is blocking railsup", conflict.tool));
                        if let Some(ref loc) = conflict.location {
                            println!("    Installed at {}", loc.display());
                        }
                        println!("    Use `railsup exec` or configure shell-init");
                    }
                }
            }
        }

        println!();
    }

    // PATH Analysis section
    if verbose || !report.path_analysis.ruby_correct {
        println!("PATH Analysis");

        // Show relevant PATH entries
        for (i, entry) in report.path_analysis.entries.iter().take(6).enumerate() {
            let annotation = match entry.source {
                report::PathSource::Railsup => " <- railsup (active)",
                report::PathSource::RailsupGems => " <- gem binaries",
                report::PathSource::Rbenv => " <- rbenv",
                report::PathSource::Asdf => " <- asdf",
                report::PathSource::Rvm => " <- rvm",
                report::PathSource::Mise => " <- mise",
                report::PathSource::Homebrew => " <- homebrew",
                report::PathSource::System => "",
                report::PathSource::Unknown => "",
            };
            println!("  {}. {}{}", i + 1, entry.path.display(), annotation);
        }

        if report.path_analysis.entries.len() > 6 {
            println!("  ...");
        }

        println!();

        // which ruby comparison
        if let Some(ref which_ruby) = report.path_analysis.which_ruby {
            if report.path_analysis.ruby_correct {
                ui::success(&format!("which ruby -> {}", which_ruby.display()));
            } else {
                ui::error(&format!("which ruby -> {}", which_ruby.display()));
                println!(
                    "    Expected: {}",
                    report.path_analysis.expected_ruby.display()
                );
            }
        } else if report.ruby_status.any_installed {
            ui::warn("which ruby -> not found");
            println!("    Shell integration may not be active");
        }

        println!();
    }

    // Environment Variables section (only show issues or in verbose)
    if !report.environment.issues.is_empty() || verbose {
        println!("Environment Variables");

        if report.environment.issues.is_empty() {
            ui::success("No problematic variables detected");
        } else {
            for issue in &report.environment.issues {
                ui::warn(issue);
            }
        }

        println!();
    }

    // Project section (if in a Rails project)
    if let Some(ref project) = report.project {
        println!("Project");
        ui::dim(&format!("Path: {}", project.path.display()));

        if let Some(ref ruby_ver) = project.ruby_version_file {
            println!("  .ruby-version: {}", ruby_ver);
        }
        if let Some(ref gemfile_ver) = project.gemfile_ruby {
            println!("  Gemfile ruby: {}", gemfile_ver);
        }
        if let Some(ref toml_ver) = project.railsup_toml {
            println!("  railsup.toml: {}", toml_ver);
        }

        if !project.version_match {
            ui::warn("Project Ruby version may not match installed version");
        }

        println!();
    }
}

/// Apply automatic fixes
fn apply_fixes(report: &report::DiagnosticReport) -> Result<()> {
    let mut fixes_available = false;

    // Check for fixable issues
    if !report.shell_integration.configured {
        fixes_available = true;
        println!();
        println!("Fixable Issues Found:");
        println!();
        println!("1. Shell integration not configured");

        if let Some(shell_file) = get_shell_config_file() {
            println!(
                "   Fix: Add `eval \"$(railsup shell-init)\"` to {}",
                shell_file
            );
            print!("   [Apply? y/n] ");

            use std::io::{self, Write};
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() == "y" {
                // Append shell-init to config file
                let home = dirs::home_dir().expect("Could not get home directory");
                let config_path = home.join(&shell_file);

                use std::fs::OpenOptions;
                let mut file = OpenOptions::new().append(true).open(&config_path)?;

                use std::io::Write as _;
                writeln!(file)?;
                writeln!(file, "# Railsup shell integration")?;
                writeln!(file, "eval \"$(railsup shell-init)\"")?;

                ui::success(&format!("Added to {}", shell_file));
                println!();
                println!("Restart your shell or run: source ~/{}", shell_file);
            } else {
                println!("   Skipped.");
            }
        }
    }

    if !fixes_available {
        println!();
        ui::success("No fixable issues found");
    }

    Ok(())
}

/// Get the appropriate shell config file for the current shell
fn get_shell_config_file() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.contains("zsh") {
        Some(".zshrc".to_string())
    } else if shell.contains("bash") {
        // Prefer .bashrc on Linux, .bash_profile on macOS
        #[cfg(target_os = "macos")]
        return Some(".bash_profile".to_string());
        #[cfg(not(target_os = "macos"))]
        return Some(".bashrc".to_string());
    } else {
        None
    }
}
