//! Custom assertions for railsup integration tests

use super::RunResult;

/// Extension trait for railsup-specific assertions
pub trait RailsupAssertions {
    /// Assert that bundle context was detected
    fn assert_bundle_detected(&self);

    /// Assert that no bundle context was detected
    fn assert_no_bundle_detected(&self);

    /// Assert that a binstub was used
    fn assert_used_binstub(&self, name: &str);

    /// Assert that bundle exec was used
    fn assert_used_bundle_exec(&self);

    /// Assert that error output contains a message
    fn assert_error_contains(&self, message: &str);

    /// Assert command succeeded
    fn assert_success(&self);

    /// Assert command failed
    fn assert_failure(&self);
}

impl RailsupAssertions for RunResult {
    fn assert_bundle_detected(&self) {
        assert!(
            self.output_contains("Detected Gemfile")
                || self.output_contains("using project bundle")
                || self.output_contains("bundle context"),
            "Expected bundle detection message in output:\nstdout: {}\nstderr: {}",
            self.stdout,
            self.stderr
        );
    }

    fn assert_no_bundle_detected(&self) {
        assert!(
            !self.output_contains("Detected Gemfile")
                && !self.output_contains("using project bundle"),
            "Did not expect bundle detection message, but found it:\nstdout: {}\nstderr: {}",
            self.stdout,
            self.stderr
        );
    }

    fn assert_used_binstub(&self, name: &str) {
        let binstub_pattern = format!("bin/{}", name);
        assert!(
            self.output_contains(&binstub_pattern),
            "Expected binstub bin/{} to be used in output:\nstdout: {}\nstderr: {}",
            name,
            self.stdout,
            self.stderr
        );
    }

    fn assert_used_bundle_exec(&self) {
        assert!(
            self.output_contains("bundle exec"),
            "Expected 'bundle exec' in output:\nstdout: {}\nstderr: {}",
            self.stdout,
            self.stderr
        );
    }

    fn assert_error_contains(&self, message: &str) {
        assert!(
            !self.success() && self.output_contains(message),
            "Expected error containing '{}', got:\nexit: {}\nstdout: {}\nstderr: {}",
            message,
            self.exit_code,
            self.stdout,
            self.stderr
        );
    }

    fn assert_success(&self) {
        assert!(
            self.success(),
            "Expected success (exit 0), got exit {}:\nstdout: {}\nstderr: {}",
            self.exit_code,
            self.stdout,
            self.stderr
        );
    }

    fn assert_failure(&self) {
        assert!(
            !self.success(),
            "Expected failure (non-zero exit), got exit 0:\nstdout: {}\nstderr: {}",
            self.stdout,
            self.stderr
        );
    }
}
