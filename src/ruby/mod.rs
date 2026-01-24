// System Ruby detection (kept for potential fallback use)
#[allow(dead_code)]
mod detect;

#[allow(unused_imports)]
pub use detect::{detect, RubyError, RubyInfo};
