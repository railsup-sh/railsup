//! Opt-out mechanism integration tests
//!
//! Verifies PEP-0016 opt-out behavior:
//! - RAILSUP_NO_BUNDLE=1 disables bundle wrapping
//! - Opt-out also prevents BUNDLE_GEMFILE from being set

use super::harness::{railsup_with_env, Fixture, RailsupAssertions};
use std::collections::HashMap;

#[test]
fn opt_out_disables_bundle_detection_message() {
    let fixture = Fixture::load("rails-8-app");

    let mut env = HashMap::new();
    env.insert("RAILSUP_NO_BUNDLE".to_string(), "1".to_string());

    let result = railsup_with_env(&fixture, &["exec", "echo", "test"], env);

    // Should NOT show bundle detection when opt-out is set
    result.assert_no_bundle_detected();
    result.assert_success();
}

#[test]
fn opt_out_works_with_dev_command() {
    let fixture = Fixture::load("rails-8-app");

    let mut env = HashMap::new();
    env.insert("RAILSUP_NO_BUNDLE".to_string(), "1".to_string());

    // dev command will still fail (no Ruby installed typically) but
    // should not show the bundle detection message
    let result = railsup_with_env(&fixture, &["dev"], env);

    // The command might fail for other reasons, but we check opt-out worked
    // by verifying no bundle detection message
    result.assert_no_bundle_detected();
}

#[test]
fn without_opt_out_shows_bundle_detection() {
    let fixture = Fixture::load("rails-8-app");
    let result = railsup_with_env(&fixture, &["exec", "echo", "test"], HashMap::new());

    // Without opt-out, should show bundle detection
    result.assert_bundle_detected();
    result.assert_success();
}
