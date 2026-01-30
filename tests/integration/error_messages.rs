//! Error message integration tests
//!
//! Verifies helpful error messages for common problems:
//! - No Rails project found
//! - Missing Gemfile.lock
//! - Helpful hints for fixes

use super::harness::{railsup, Fixture, RailsupAssertions};

#[test]
#[ignore]
fn dev_in_empty_dir_shows_helpful_error() {
    let fixture = Fixture::load("empty-dir");
    let result = railsup(&fixture, &["dev"]);

    result.assert_failure();
    // Should mention it's not a Rails directory
    assert!(
        result.stderr_contains("Rails")
            || result.stderr_contains("rails")
            || result.output_contains("Not a Rails directory"),
        "Expected Rails-related error message, got:\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );
}

#[test]
#[ignore]
fn dev_in_non_rails_ruby_shows_clear_error() {
    let fixture = Fixture::load("non-rails-ruby");
    let result = railsup(&fixture, &["dev"]);

    result.assert_failure();
    result.assert_error_contains("Not a Rails directory");
}

#[test]
#[ignore]
fn missing_lockfile_detected() {
    let fixture = Fixture::load("rails-no-lockfile");
    // This will likely fail because no Ruby installed, but we can check detection
    let result = railsup(&fixture, &["dev"]);

    // Should either mention bundle install or fail with Ruby not installed
    // Both are acceptable - the key is it detected the Rails project
    // and tried to proceed
    let output = result.output();
    assert!(
        output.contains("bundle install")
            || output.contains("Ruby")
            || output.contains("Detected Gemfile"),
        "Expected bundle install mention or Ruby requirement, got: {}",
        output
    );
}

#[test]
#[ignore]
fn help_command_always_works() {
    let fixture = Fixture::load("empty-dir");
    let result = railsup(&fixture, &["--help"]);

    result.assert_success();
    assert!(result.stdout_contains("railsup") || result.stdout_contains("Usage"));
}

#[test]
#[ignore]
fn version_command_works() {
    let fixture = Fixture::load("empty-dir");
    let result = railsup(&fixture, &["--version"]);

    result.assert_success();
    // Version should contain a number
    assert!(
        result.stdout.chars().any(|c| c.is_ascii_digit()),
        "Version output should contain version number: {}",
        result.stdout
    );
}
