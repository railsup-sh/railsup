use crate::cli::bundler::{
    self, build_full_env, check_bundler_version_mismatch, detect_bundle_context,
    format_bundle_detected_message, is_bundle_opt_out, needs_bundle_install, wrap_procfile_command,
    BundleContext,
};
use crate::cli::new::ensure_ruby_available;
use crate::paths;
use crate::util::ui;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, IsTerminal};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Timeout for graceful shutdown before force kill
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

/// Type alias for child process with output thread handles
type ChildWithHandles = (
    Child,
    Option<thread::JoinHandle<()>>,
    Option<thread::JoinHandle<()>>,
);

/// Process colors for output prefixes (only used when stdout is a TTY)
const COLORS: &[&str] = &[
    "\x1b[36m", // cyan
    "\x1b[35m", // magenta
    "\x1b[33m", // yellow
    "\x1b[32m", // green
    "\x1b[34m", // blue
];
const RESET: &str = "\x1b[0m";

/// Check if stdout is a TTY (for color output)
fn use_colors() -> bool {
    std::io::stdout().is_terminal()
}

/// Get color code for a process index, or empty string if no TTY
fn get_color(index: usize) -> &'static str {
    if use_colors() {
        COLORS[index % COLORS.len()]
    } else {
        ""
    }
}

/// Get reset code, or empty string if no TTY
fn get_reset() -> &'static str {
    if use_colors() {
        RESET
    } else {
        ""
    }
}

pub fn run(port: u16) -> Result<()> {
    // 1. Detect bundle context (finds Rails root + Gemfile)
    let current_dir = env::current_dir()?;
    let bundle_ctx = detect_bundle_context(&current_dir).ok_or_else(|| {
        anyhow::anyhow!("Not a Rails directory. Create one with: railsup new myapp")
    })?;

    // Show bundle detection message (PEP-0016, respects opt-out)
    if !is_bundle_opt_out() {
        ui::info(&format_bundle_detected_message(&bundle_ctx));
    }

    // 2. Ensure Ruby is available (auto-bootstrap if needed)
    let ruby_version = ensure_ruby_available()?;
    let ruby_bin = paths::ruby_bin_dir(&ruby_version);

    // 3. Check for bundler version mismatch (PEP-0016)
    if let Some(warning) = check_bundler_version_mismatch(&bundle_ctx, &ruby_bin) {
        ui::warn(&warning);
    }

    // 4. Check for missing Gemfile.lock and auto-install if needed
    if needs_bundle_install(&bundle_ctx) {
        ui::info("No Gemfile.lock found. Running bundle install...");
        run_bundle_install(&bundle_ctx, &ruby_version)?;
    }

    // 5. Check for Procfile.dev
    let procfile_path = bundle_ctx.rails_root.join("Procfile.dev");
    if procfile_path.exists() {
        run_with_procfile(&procfile_path, &bundle_ctx, &ruby_version, port)
    } else {
        run_server_only(&bundle_ctx, &ruby_bin, port)
    }
}

/// Run bundle install to create Gemfile.lock
fn run_bundle_install(bundle_ctx: &BundleContext, ruby_version: &str) -> Result<()> {
    let env_vars = build_full_env(ruby_version, &Some(bundle_ctx.clone()));
    let ruby_bin = paths::ruby_bin_dir(ruby_version);
    let bundle_path = ruby_bin.join("bundle");

    let status = Command::new(&bundle_path)
        .arg("install")
        .current_dir(&bundle_ctx.rails_root)
        .envs(&env_vars)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        bail!(
            "bundle install failed. Try running manually:\n  \
             cd {} && bundle install",
            bundle_ctx.rails_root.display()
        );
    }

    Ok(())
}

/// Run all processes defined in Procfile.dev
fn run_with_procfile(
    procfile_path: &Path,
    bundle_ctx: &BundleContext,
    ruby_version: &str,
    port: u16,
) -> Result<()> {
    let processes = parse_procfile(procfile_path)?;

    if processes.is_empty() {
        bail!("Procfile.dev is empty");
    }

    ui::info("Starting development processes...");

    // Build environment with full Ruby + bundle context (PEP-0016)
    let env_vars = build_full_env(ruby_version, &Some(bundle_ctx.clone()));

    // Spawn all processes
    let mut children: Vec<(String, Child)> = vec![];
    let bundle_ctx_opt = Some(bundle_ctx.clone());
    for (i, (name, mut command)) in processes.into_iter().enumerate() {
        // Replace port in web process
        if name == "web" {
            command = replace_port_in_command(&command, port);
        }

        // Wrap Procfile commands with bundle exec if needed (PEP-0016)
        command = wrap_procfile_command(&bundle_ctx_opt, &command);

        let color = get_color(i);
        let reset = get_reset();
        ui::info(&format!("{}[{}]{} {}", color, name, reset, command));

        let child = spawn_process(&command, &bundle_ctx.rails_root, &env_vars)?;
        children.push((name, child));
    }

    // Set up signal handling for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    if let Err(e) = ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }) {
        ui::warn(&format!("Could not set signal handler: {}", e));
    }

    println!();

    // Determine if we should use colors (check once, pass to threads)
    let colors_enabled = use_colors();

    // Stream output from all processes
    let handles: Vec<_> = children
        .into_iter()
        .enumerate()
        .map(|(i, (name, mut child))| {
            let color = if colors_enabled {
                COLORS[i % COLORS.len()].to_string()
            } else {
                String::new()
            };
            let reset = if colors_enabled {
                RESET.to_string()
            } else {
                String::new()
            };

            // Take stdout and stderr
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            let name_clone = name.clone();
            let color_clone = color.clone();
            let reset_clone = reset.clone();

            // Spawn thread to read stdout
            let stdout_handle = stdout.map(|out| {
                let name = name_clone.clone();
                let color = color_clone.clone();
                let reset = reset_clone.clone();
                thread::spawn(move || {
                    let reader = BufReader::new(out);
                    for line in reader.lines().map_while(Result::ok) {
                        println!("{}[{}]{} {}", color, name, reset, line);
                    }
                })
            });

            // Spawn thread to read stderr
            let stderr_handle = stderr.map(|err| {
                let name = name.clone();
                let color = color.clone();
                let reset = reset.clone();
                thread::spawn(move || {
                    let reader = BufReader::new(err);
                    for line in reader.lines().map_while(Result::ok) {
                        eprintln!("{}[{}]{} {}", color, name, reset, line);
                    }
                })
            });

            (child, stdout_handle, stderr_handle)
        })
        .collect();

    // Wait for processes or signal - graceful shutdown on Ctrl+C
    let mut children_to_wait: Vec<_> = handles;

    loop {
        if !running.load(Ordering::SeqCst) {
            // Ctrl+C received - graceful shutdown
            graceful_shutdown(&mut children_to_wait);
            break;
        }

        // Check if all processes have exited
        let mut all_done = true;
        for (child, _, _) in &mut children_to_wait {
            match child.try_wait() {
                Ok(Some(_)) => {} // This one is done
                Ok(None) => all_done = false,
                Err(_) => {} // Treat errors as done
            }
        }

        if all_done {
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    // Wait for all output threads to finish
    for (_, stdout_handle, stderr_handle) in children_to_wait {
        if let Some(h) = stdout_handle {
            h.join().ok();
        }
        if let Some(h) = stderr_handle {
            h.join().ok();
        }
    }

    Ok(())
}

/// Gracefully shutdown all child processes
/// Sends SIGTERM first, waits for timeout, then SIGKILL if needed
fn graceful_shutdown(children: &mut [ChildWithHandles]) {
    // First, send SIGTERM to all children (Unix) or kill (Windows)
    for (child, _, _) in children.iter_mut() {
        terminate_process(child);
    }

    // Wait for processes to exit gracefully
    let start = Instant::now();
    loop {
        let mut all_done = true;
        for (child, _, _) in children.iter_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {} // Done
                Ok(None) => all_done = false,
                Err(_) => {} // Treat errors as done
            }
        }

        if all_done {
            return;
        }

        if start.elapsed() >= SHUTDOWN_TIMEOUT {
            // Timeout - force kill remaining processes
            for (child, _, _) in children.iter_mut() {
                child.kill().ok();
            }
            return;
        }

        thread::sleep(Duration::from_millis(50));
    }
}

/// Send SIGTERM to a process (Unix) or kill it (Windows)
#[cfg(unix)]
fn terminate_process(child: &Child) {
    // Send SIGTERM to the process for graceful shutdown
    unsafe {
        libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
    }
}

#[cfg(not(unix))]
fn terminate_process(child: &mut Child) {
    // On non-Unix, just kill immediately
    child.kill().ok();
}

/// Run Rails server only (fallback when no Procfile.dev)
fn run_server_only(bundle_ctx: &BundleContext, ruby_bin: &Path, port: u16) -> Result<()> {
    ui::info(&format!("Starting Rails on http://localhost:{}", port));

    let port_str = port.to_string();

    // Use binstub if available, otherwise bundle exec (PEP-0016)
    let bundle_ctx_opt = Some(bundle_ctx.clone());
    let (cmd, args) = bundler::wrap_command(
        &bundle_ctx_opt,
        "rails",
        &["server".to_string(), "-p".to_string(), port_str],
    );

    // Build full path to command
    let cmd_path = if cmd == "bundle" || cmd == "rails" {
        ruby_bin.join(&cmd)
    } else {
        // It's a binstub path like "bin/rails"
        bundle_ctx.rails_root.join(&cmd)
    };

    // Build environment with full Ruby + bundle context
    let ruby_version = ruby_bin
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|s| s.trim_start_matches("ruby-"))
        .unwrap_or("unknown");
    let env_vars = build_full_env(ruby_version, &bundle_ctx_opt);

    let status = Command::new(&cmd_path)
        .args(&args)
        .current_dir(&bundle_ctx.rails_root)
        .envs(&env_vars)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        bail!(
            "Server exited with error.\n  \
             Try running manually: cd {} && bundle exec rails server",
            bundle_ctx.rails_root.display()
        );
    }

    Ok(())
}

/// Parse Procfile.dev into process name -> command pairs
fn parse_procfile(path: &Path) -> Result<Vec<(String, String)>> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_procfile_content(&content))
}

/// Parse Procfile content from a string (used by parse_procfile and tests)
fn parse_procfile_content(content: &str) -> Vec<(String, String)> {
    let mut processes = vec![];

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse "name: command"
        if let Some((name, command)) = line.split_once(':') {
            let name = name.trim().to_string();
            let command = command.trim().to_string();
            if !name.is_empty() && !command.is_empty() && is_valid_process_name(&name) {
                processes.push((name, command));
            }
        }
    }

    processes
}

/// Validate process name (alphanumeric, underscore, hyphen only)
fn is_valid_process_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Replace port in a command string
fn replace_port_in_command(command: &str, port: u16) -> String {
    let port_str = port.to_string();

    // Try each port pattern in order of specificity
    let patterns = [
        ("--port=", 7),
        ("--port ", 7),
        ("-p=", 3),
        ("-p ", 3),
        ("-p", 2), // -p3000 (no space) - must be last to avoid matching -p= or -p<space>
    ];

    for (pattern, prefix_len) in patterns {
        if let Some(result) = try_replace_port(command, pattern, prefix_len, &port_str) {
            return result;
        }
    }

    // No port pattern found, return unchanged
    command.to_string()
}

/// Try to replace port after a given pattern, returns None if pattern not found
fn try_replace_port(
    command: &str,
    pattern: &str,
    prefix_len: usize,
    port_str: &str,
) -> Option<String> {
    let idx = command.find(pattern)?;
    let start = idx + prefix_len;

    // For -p without space/equals, verify next char is a digit
    if pattern == "-p" {
        let next_char = command[start..].chars().next()?;
        if !next_char.is_ascii_digit() {
            return None;
        }
    }

    // Find end of port number
    let end = command[start..]
        .find(|c: char| !c.is_ascii_digit())
        .map(|i| start + i)
        .unwrap_or(command.len());

    // Only replace if there's actually a port number
    if start == end {
        return None;
    }

    let mut result = command.to_string();
    result.replace_range(start..end, port_str);
    Some(result)
}

/// Spawn a process with the given command
fn spawn_process(
    command: &str,
    working_dir: &Path,
    env_vars: &HashMap<String, String>,
) -> Result<Child> {
    // Use shell to handle command parsing
    let child = Command::new("sh")
        .args(["-c", command])
        .current_dir(working_dir)
        .envs(env_vars)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    Ok(child)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::bundler::find_rails_root;
    use tempfile::tempdir;

    // ==================== find_rails_root tests ====================
    // (Tests now use bundler::find_rails_root)

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

    // ==================== parse_procfile_content tests ====================

    #[test]
    fn parse_procfile_basic() {
        let content = "web: bin/rails server -p 3000\ncss: bin/rails tailwindcss:watch";
        let result = parse_procfile_content(content);
        assert_eq!(
            result,
            vec![
                ("web".to_string(), "bin/rails server -p 3000".to_string()),
                ("css".to_string(), "bin/rails tailwindcss:watch".to_string()),
            ]
        );
    }

    #[test]
    fn parse_procfile_with_comments() {
        let content = "# This is a comment\nweb: bin/rails server\n# Another comment\ncss: bin/rails tailwindcss:watch";
        let result = parse_procfile_content(content);
        assert_eq!(
            result,
            vec![
                ("web".to_string(), "bin/rails server".to_string()),
                ("css".to_string(), "bin/rails tailwindcss:watch".to_string()),
            ]
        );
    }

    #[test]
    fn parse_procfile_with_empty_lines() {
        let content = "web: bin/rails server\n\n\ncss: bin/rails tailwindcss:watch\n";
        let result = parse_procfile_content(content);
        assert_eq!(
            result,
            vec![
                ("web".to_string(), "bin/rails server".to_string()),
                ("css".to_string(), "bin/rails tailwindcss:watch".to_string()),
            ]
        );
    }

    #[test]
    fn parse_procfile_with_whitespace() {
        let content = "  web  :  bin/rails server  \n  css:bin/rails tailwindcss:watch";
        let result = parse_procfile_content(content);
        assert_eq!(
            result,
            vec![
                ("web".to_string(), "bin/rails server".to_string()),
                ("css".to_string(), "bin/rails tailwindcss:watch".to_string()),
            ]
        );
    }

    #[test]
    fn parse_procfile_empty() {
        let content = "";
        let result = parse_procfile_content(content);
        assert!(result.is_empty());
    }

    #[test]
    fn parse_procfile_only_comments() {
        let content = "# Comment 1\n# Comment 2\n";
        let result = parse_procfile_content(content);
        assert!(result.is_empty());
    }

    #[test]
    fn parse_procfile_malformed_lines() {
        let content = "web: bin/rails server\nno_colon_here\n:empty_name\nempty_command:\ncss: bin/rails tailwindcss:watch";
        let result = parse_procfile_content(content);
        // Only valid lines should be parsed
        assert_eq!(
            result,
            vec![
                ("web".to_string(), "bin/rails server".to_string()),
                ("css".to_string(), "bin/rails tailwindcss:watch".to_string()),
            ]
        );
    }

    #[test]
    fn parse_procfile_command_with_colons() {
        // Commands can contain colons (like URLs or Ruby namespaced tasks)
        let content = "web: bin/rails server -b http://localhost:3000";
        let result = parse_procfile_content(content);
        assert_eq!(
            result,
            vec![(
                "web".to_string(),
                "bin/rails server -b http://localhost:3000".to_string()
            ),]
        );
    }

    // ==================== replace_port_in_command tests ====================

    #[test]
    fn replace_port_short_flag_with_space() {
        let cmd = "bin/rails server -p 3000";
        let result = replace_port_in_command(cmd, 4000);
        assert_eq!(result, "bin/rails server -p 4000");
    }

    #[test]
    fn replace_port_short_flag_with_equals() {
        let cmd = "bin/rails server -p=3000";
        let result = replace_port_in_command(cmd, 4000);
        assert_eq!(result, "bin/rails server -p=4000");
    }

    #[test]
    fn replace_port_long_flag_with_space() {
        let cmd = "bin/rails server --port 3000";
        let result = replace_port_in_command(cmd, 4000);
        assert_eq!(result, "bin/rails server --port 4000");
    }

    #[test]
    fn replace_port_long_flag_with_equals() {
        let cmd = "bin/rails server --port=3000";
        let result = replace_port_in_command(cmd, 4000);
        assert_eq!(result, "bin/rails server --port=4000");
    }

    #[test]
    fn replace_port_no_port_in_command() {
        let cmd = "bin/rails server";
        let result = replace_port_in_command(cmd, 4000);
        // No port to replace, command unchanged
        assert_eq!(result, "bin/rails server");
    }

    #[test]
    fn replace_port_with_trailing_args() {
        let cmd = "bin/rails server -p 3000 -b 0.0.0.0";
        let result = replace_port_in_command(cmd, 4000);
        assert_eq!(result, "bin/rails server -p 4000 -b 0.0.0.0");
    }

    #[test]
    fn replace_port_at_end_of_command() {
        let cmd = "bin/rails server -b 0.0.0.0 -p 3000";
        let result = replace_port_in_command(cmd, 4000);
        assert_eq!(result, "bin/rails server -b 0.0.0.0 -p 4000");
    }

    #[test]
    fn replace_port_different_port_number() {
        let cmd = "bin/rails server -p 8080";
        let result = replace_port_in_command(cmd, 9000);
        assert_eq!(result, "bin/rails server -p 9000");
    }

    #[test]
    fn replace_port_short_flag_no_space() {
        let cmd = "bin/rails server -p3000";
        let result = replace_port_in_command(cmd, 4000);
        assert_eq!(result, "bin/rails server -p4000");
    }

    #[test]
    fn replace_port_short_flag_no_space_with_trailing() {
        let cmd = "bin/rails server -p3000 -b 0.0.0.0";
        let result = replace_port_in_command(cmd, 4000);
        assert_eq!(result, "bin/rails server -p4000 -b 0.0.0.0");
    }

    // ==================== is_valid_process_name tests ====================

    #[test]
    fn valid_process_name_simple() {
        assert!(is_valid_process_name("web"));
        assert!(is_valid_process_name("css"));
        assert!(is_valid_process_name("worker"));
    }

    #[test]
    fn valid_process_name_with_numbers() {
        assert!(is_valid_process_name("web1"));
        assert!(is_valid_process_name("worker2"));
    }

    #[test]
    fn valid_process_name_with_underscore() {
        assert!(is_valid_process_name("web_server"));
        assert!(is_valid_process_name("css_watcher"));
    }

    #[test]
    fn valid_process_name_with_hyphen() {
        assert!(is_valid_process_name("web-server"));
        assert!(is_valid_process_name("css-watcher"));
    }

    #[test]
    fn invalid_process_name_with_spaces() {
        assert!(!is_valid_process_name("web server"));
        assert!(!is_valid_process_name(" web"));
    }

    #[test]
    fn invalid_process_name_with_special_chars() {
        assert!(!is_valid_process_name("web@server"));
        assert!(!is_valid_process_name("css!watcher"));
        assert!(!is_valid_process_name("worker#1"));
    }

    #[test]
    fn invalid_process_name_empty() {
        assert!(!is_valid_process_name(""));
    }

    #[test]
    fn parse_procfile_rejects_invalid_names() {
        let content =
            "web: bin/rails server\ninvalid name: some command\ncss: bin/rails tailwindcss:watch";
        let result = parse_procfile_content(content);
        // "invalid name" should be rejected due to space
        assert_eq!(
            result,
            vec![
                ("web".to_string(), "bin/rails server".to_string()),
                ("css".to_string(), "bin/rails tailwindcss:watch".to_string()),
            ]
        );
    }
}
