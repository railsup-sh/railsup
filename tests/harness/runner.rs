//! Binary execution for integration tests

use super::Fixture;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Result of running the railsup binary
#[derive(Debug)]
pub struct RunResult {
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Standard output as string
    pub stdout: String,
    /// Standard error as string
    pub stderr: String,
}

impl RunResult {
    /// Check if command succeeded (exit code 0)
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Check if stdout contains a substring
    pub fn stdout_contains(&self, needle: &str) -> bool {
        self.stdout.contains(needle)
    }

    /// Check if stderr contains a substring
    pub fn stderr_contains(&self, needle: &str) -> bool {
        self.stderr.contains(needle)
    }

    /// Combined output (stdout + stderr)
    pub fn output(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }

    /// Check if combined output contains a substring
    pub fn output_contains(&self, needle: &str) -> bool {
        self.output().contains(needle)
    }
}

impl From<Output> for RunResult {
    fn from(output: Output) -> Self {
        Self {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }
}

/// Run railsup binary in fixture directory
pub fn railsup(fixture: &Fixture, args: &[&str]) -> RunResult {
    railsup_with_env(fixture, args, HashMap::new())
}

/// Run railsup with custom environment variables
pub fn railsup_with_env(
    fixture: &Fixture,
    args: &[&str],
    env: HashMap<String, String>,
) -> RunResult {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_railsup"));

    let mut cmd = Command::new(&binary);
    cmd.current_dir(&fixture.path);
    cmd.args(args);

    // Clear potentially interfering env vars
    cmd.env_remove("BUNDLE_GEMFILE");
    cmd.env_remove("RUBYOPT");
    cmd.env_remove("RUBYLIB");
    cmd.env_remove("GEM_HOME");
    cmd.env_remove("GEM_PATH");

    // Apply custom env vars
    for (key, value) in env {
        cmd.env(&key, &value);
    }

    let output = cmd.output().expect("Failed to execute railsup");
    RunResult::from(output)
}

/// Run railsup in a specific directory (not necessarily a fixture)
pub fn railsup_in_dir(dir: &std::path::Path, args: &[&str]) -> RunResult {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_railsup"));

    let mut cmd = Command::new(&binary);
    cmd.current_dir(dir);
    cmd.args(args);

    // Clear potentially interfering env vars
    cmd.env_remove("BUNDLE_GEMFILE");
    cmd.env_remove("RUBYOPT");
    cmd.env_remove("RUBYLIB");
    cmd.env_remove("GEM_HOME");
    cmd.env_remove("GEM_PATH");

    let output = cmd.output().expect("Failed to execute railsup");
    RunResult::from(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_result_success() {
        let result = RunResult {
            exit_code: 0,
            stdout: "output".to_string(),
            stderr: "".to_string(),
        };
        assert!(result.success());
    }

    #[test]
    fn run_result_failure() {
        let result = RunResult {
            exit_code: 1,
            stdout: "".to_string(),
            stderr: "error".to_string(),
        };
        assert!(!result.success());
    }

    #[test]
    fn run_result_contains() {
        let result = RunResult {
            exit_code: 0,
            stdout: "hello world".to_string(),
            stderr: "warning message".to_string(),
        };
        assert!(result.stdout_contains("hello"));
        assert!(result.stderr_contains("warning"));
        assert!(result.output_contains("hello"));
        assert!(result.output_contains("warning"));
    }
}
