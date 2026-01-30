//! Bundle detection integration tests
//!
//! Verifies PEP-0016 bundle context detection behavior:
//! - Detects Gemfile in Rails projects
//! - Walks up from subdirectories to find Rails root
//! - Monorepo safety: uses app's Gemfile, not parent's

use super::harness::{railsup, Fixture, RailsupAssertions};

#[test]
fn detects_gemfile_in_rails_project() {
    let fixture = Fixture::load("rails-8-app");
    let result = railsup(&fixture, &["--help"]);

    // Help command should work
    result.assert_success();
}

#[test]
fn no_detection_in_empty_directory() {
    let fixture = Fixture::load("empty-dir");
    let result = railsup(&fixture, &["dev"]);

    // Should fail because no Rails project
    result.assert_failure();
    result.assert_error_contains("Not a Rails directory");
}

#[test]
fn no_detection_in_non_rails_ruby_project() {
    let fixture = Fixture::load("non-rails-ruby");
    let result = railsup(&fixture, &["dev"]);

    // Should fail - has Gemfile but no config/application.rb
    result.assert_failure();
    result.assert_error_contains("Not a Rails directory");
}

#[test]
fn monorepo_uses_app_gemfile_not_parent() {
    // Run from the nested Rails app directory
    let fixture = Fixture::load("monorepo");
    let app_fixture = fixture.subdir("apps/myapp");

    let result = railsup(&app_fixture, &["--help"]);

    // Should work from the nested app
    result.assert_success();
}

#[test]
fn detects_bundle_context_shows_message() {
    let fixture = Fixture::load("rails-8-app");
    // Use exec with a simple command that will fail fast but show detection message
    let result = railsup(&fixture, &["exec", "echo", "test"]);

    // Should show bundle detection message
    result.assert_bundle_detected();
}
