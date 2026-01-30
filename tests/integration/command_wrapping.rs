//! Command wrapping integration tests
//!
//! Verifies PEP-0016 command wrapping behavior:
//! - Bare commands get wrapped with bundle exec
//! - Already-wrapped commands are not double-wrapped
//! - Unknown commands are not wrapped

use super::harness::{railsup, Fixture, RailsupAssertions};

#[test]
#[ignore]
fn exec_command_works_in_rails_project() {
    let fixture = Fixture::load("rails-8-app");
    let result = railsup(&fixture, &["exec", "echo", "integration-test-marker"]);

    result.assert_success();
    assert!(result.stdout_contains("integration-test-marker"));
}

#[test]
#[ignore]
fn exec_command_shows_bundle_detected() {
    let fixture = Fixture::load("rails-8-app");
    let result = railsup(&fixture, &["exec", "echo", "test"]);

    result.assert_bundle_detected();
}

#[test]
#[ignore]
fn exec_in_non_rails_project_no_bundle_detection() {
    let fixture = Fixture::load("non-rails-ruby");
    let result = railsup(&fixture, &["exec", "echo", "test"]);

    // Should not show bundle detection (no config/application.rb)
    result.assert_no_bundle_detected();
    result.assert_success();
}

#[test]
#[ignore]
fn exec_in_empty_dir_no_bundle_detection() {
    let fixture = Fixture::load("empty-dir");
    let result = railsup(&fixture, &["exec", "echo", "test"]);

    result.assert_no_bundle_detected();
    result.assert_success();
}
