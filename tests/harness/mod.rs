//! Test harness for railsup integration tests
//!
//! Provides fixture loading, binary execution, and custom assertions
//! for testing end-to-end CLI behavior.

mod assertions;
mod fixture;
mod runner;

pub use assertions::RailsupAssertions;
pub use fixture::Fixture;
pub use runner::{railsup, railsup_with_env, RunResult};
