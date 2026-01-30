//! Integration test entry point
//!
//! Run with: cargo test --test integration
//!
//! These tests run against the compiled railsup binary using real
//! project fixtures, verifying end-to-end CLI behavior per PEP-0016
//! (Gem Isolation Strategy) and PEP-0018 (Integration Test Infrastructure).

mod harness;

// Include integration test modules directly
#[path = "integration/bundle_detection.rs"]
mod bundle_detection;

#[path = "integration/binstub_preference.rs"]
mod binstub_preference;

#[path = "integration/command_wrapping.rs"]
mod command_wrapping;

#[path = "integration/opt_out.rs"]
mod opt_out;

#[path = "integration/error_messages.rs"]
mod error_messages;
