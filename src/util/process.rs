use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

/// Run a command with output streamed to the terminal.
/// Uses current_dir to set working directory (doesn't change process cwd).
pub fn run_streaming<S: AsRef<OsStr>>(
    program: &str,
    args: &[S],
    working_dir: Option<&Path>,
) -> Result<ExitStatus> {
    run_streaming_with_env(program, args, working_dir, None)
}

/// Run a command with output streamed to the terminal, with custom PATH prepended.
/// This ensures subprocesses also use the correct Ruby.
pub fn run_streaming_with_env<S: AsRef<OsStr>>(
    program: &str,
    args: &[S],
    working_dir: Option<&Path>,
    prepend_path: Option<&Path>,
) -> Result<ExitStatus> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    // Prepend to PATH so subprocesses find the correct Ruby
    if let Some(bin_dir) = prepend_path {
        let current_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", bin_dir.display(), current_path);
        cmd.env("PATH", new_path);
    }

    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute: {} {}", program, format_args(args)))?;

    Ok(status)
}

/// Run a command and capture stdout.
pub fn run_capture<S: AsRef<OsStr>>(program: &str, args: &[S]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute: {} {}", program, format_args(args)))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Format args for error messages
fn format_args<S: AsRef<OsStr>>(args: &[S]) -> String {
    args.iter()
        .map(|s| s.as_ref().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_args_works() {
        let args = vec!["--version", "-v"];
        assert_eq!(format_args(&args), "--version -v");
    }

    #[test]
    fn format_args_empty() {
        let args: Vec<&str> = vec![];
        assert_eq!(format_args(&args), "");
    }
}
