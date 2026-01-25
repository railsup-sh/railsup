use crate::cli::new::ensure_ruby_available;
use crate::paths;
use crate::util::ui;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

/// Process colors for output prefixes
const COLORS: &[&str] = &[
    "\x1b[36m", // cyan
    "\x1b[35m", // magenta
    "\x1b[33m", // yellow
    "\x1b[32m", // green
    "\x1b[34m", // blue
];
const RESET: &str = "\x1b[0m";

pub fn run(port: u16) -> Result<()> {
    // 1. Find Rails root
    let current_dir = env::current_dir()?;
    let rails_root = find_rails_root(&current_dir).ok_or_else(|| {
        anyhow::anyhow!("Not a Rails directory. Create one with: railsup new myapp")
    })?;

    // 2. Ensure Ruby is available (auto-bootstrap if needed)
    let ruby_version = ensure_ruby_available()?;
    let ruby_bin = paths::ruby_bin_dir(&ruby_version);

    // 3. Check for Procfile.dev
    let procfile_path = rails_root.join("Procfile.dev");
    if procfile_path.exists() {
        run_with_procfile(&procfile_path, &rails_root, &ruby_bin, port)
    } else {
        run_server_only(&rails_root, &ruby_bin, port)
    }
}

/// Run all processes defined in Procfile.dev
fn run_with_procfile(
    procfile_path: &Path,
    rails_root: &Path,
    ruby_bin: &Path,
    port: u16,
) -> Result<()> {
    let processes = parse_procfile(procfile_path)?;

    if processes.is_empty() {
        bail!("Procfile.dev is empty");
    }

    ui::info("Starting development processes...");

    // Build environment with railsup Ruby in PATH
    let mut env_vars: HashMap<String, String> = std::env::vars().collect();
    let current_path = env_vars.get("PATH").cloned().unwrap_or_default();
    env_vars.insert(
        "PATH".to_string(),
        format!("{}:{}", ruby_bin.display(), current_path),
    );

    // Spawn all processes
    let mut children: Vec<(String, Child)> = vec![];
    for (i, (name, mut command)) in processes.into_iter().enumerate() {
        // Replace port in web process
        if name == "web" {
            command = replace_port_in_command(&command, port);
        }

        let color = COLORS[i % COLORS.len()];
        ui::info(&format!("{}[{}]{} {}", color, name, RESET, command));

        let child = spawn_process(&command, rails_root, &env_vars)?;
        children.push((name, child));
    }

    // Set up signal handling for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .ok();

    println!();

    // Stream output from all processes
    let handles: Vec<_> = children
        .into_iter()
        .enumerate()
        .map(|(i, (name, mut child))| {
            let color = COLORS[i % COLORS.len()].to_string();
            let running = running.clone();

            // Take stdout and stderr
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            let name_clone = name.clone();
            let color_clone = color.clone();

            // Spawn thread to read stdout
            let stdout_handle = stdout.map(|out| {
                let name = name_clone.clone();
                let color = color_clone.clone();
                thread::spawn(move || {
                    let reader = BufReader::new(out);
                    for line in reader.lines().map_while(Result::ok) {
                        println!("{}[{}]{} {}", color, name, RESET, line);
                    }
                })
            });

            // Spawn thread to read stderr
            let stderr_handle = stderr.map(|err| {
                let name = name.clone();
                let color = color.clone();
                thread::spawn(move || {
                    let reader = BufReader::new(err);
                    for line in reader.lines().map_while(Result::ok) {
                        eprintln!("{}[{}]{} {}", color, name, RESET, line);
                    }
                })
            });

            (child, stdout_handle, stderr_handle, running)
        })
        .collect();

    // Wait for processes or signal
    for (mut child, stdout_handle, stderr_handle, running) in handles {
        loop {
            if !running.load(Ordering::SeqCst) {
                // Ctrl+C received, kill child
                child.kill().ok();
                break;
            }

            match child.try_wait() {
                Ok(Some(_status)) => break,
                Ok(None) => {
                    thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(_) => break,
            }
        }

        // Wait for output threads to finish
        if let Some(h) = stdout_handle {
            h.join().ok();
        }
        if let Some(h) = stderr_handle {
            h.join().ok();
        }
    }

    Ok(())
}

/// Run Rails server only (fallback when no Procfile.dev)
fn run_server_only(rails_root: &Path, ruby_bin: &Path, port: u16) -> Result<()> {
    ui::info(&format!("Starting Rails on http://localhost:{}", port));

    let bundle_path = ruby_bin.join("bundle");
    let port_str = port.to_string();

    let status = Command::new(&bundle_path)
        .args(["exec", "rails", "server", "-p", &port_str])
        .current_dir(rails_root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                ruby_bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        bail!(
            "Server exited with error.\n  \
             Try running manually: cd {} && bundle exec rails server",
            rails_root.display()
        );
    }

    Ok(())
}

/// Parse Procfile.dev into process name -> command pairs
fn parse_procfile(path: &Path) -> Result<Vec<(String, String)>> {
    let content = std::fs::read_to_string(path)?;
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
            if !name.is_empty() && !command.is_empty() {
                processes.push((name, command));
            }
        }
    }

    Ok(processes)
}

/// Replace port in a command string
fn replace_port_in_command(command: &str, port: u16) -> String {
    // Replace -p XXXX or --port XXXX or -p=XXXX
    let mut result = command.to_string();
    let port_str = port.to_string();

    // Pattern: -p 3000 or -p3000
    if let Some(idx) = result.find("-p ") {
        let start = idx + 3;
        let end = result[start..]
            .find(|c: char| !c.is_ascii_digit())
            .map(|i| start + i)
            .unwrap_or(result.len());
        result.replace_range(start..end, &port_str);
    } else if let Some(idx) = result.find("-p=") {
        let start = idx + 3;
        let end = result[start..]
            .find(|c: char| !c.is_ascii_digit())
            .map(|i| start + i)
            .unwrap_or(result.len());
        result.replace_range(start..end, &port_str);
    } else if let Some(idx) = result.find("--port ") {
        let start = idx + 7;
        let end = result[start..]
            .find(|c: char| !c.is_ascii_digit())
            .map(|i| start + i)
            .unwrap_or(result.len());
        result.replace_range(start..end, &port_str);
    } else if let Some(idx) = result.find("--port=") {
        let start = idx + 7;
        let end = result[start..]
            .find(|c: char| !c.is_ascii_digit())
            .map(|i| start + i)
            .unwrap_or(result.len());
        result.replace_range(start..end, &port_str);
    }

    result
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
