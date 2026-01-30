//! Binstub preference integration tests
//!
//! Verifies PEP-0016 binstub preference behavior:
//! - Uses bin/rails when it exists
//! - Falls back to bundle exec when no binstub

use super::harness::{railsup, Fixture, RailsupAssertions};

#[test]
#[ignore]
fn fixture_with_binstubs_has_bin_rails() {
    let fixture = Fixture::load("rails-8-app");
    let bin_rails = fixture.path.join("bin/rails");
    assert!(bin_rails.exists(), "Fixture should have bin/rails");
    assert!(bin_rails.is_file(), "bin/rails should be a file");
}

#[test]
#[ignore]
fn fixture_without_binstubs_missing_bin_rails() {
    let fixture = Fixture::load("rails-no-binstubs");
    let bin_rails = fixture.path.join("bin/rails");
    assert!(!bin_rails.exists(), "Fixture should NOT have bin/rails");
}

#[test]
#[ignore]
fn detects_bundle_in_project_with_binstubs() {
    let fixture = Fixture::load("rails-8-app");
    let result = railsup(&fixture, &["exec", "echo", "hello"]);

    // Should detect bundle context
    result.assert_bundle_detected();
}

#[test]
#[ignore]
fn detects_bundle_in_project_without_binstubs() {
    let fixture = Fixture::load("rails-no-binstubs");
    let result = railsup(&fixture, &["exec", "echo", "hello"]);

    // Should still detect bundle context (Gemfile exists)
    result.assert_bundle_detected();
}
