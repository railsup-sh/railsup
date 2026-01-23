mod detect;

pub use detect::detect;

// Re-export for potential future use
#[allow(unused_imports)]
pub use detect::{RubyError, RubyInfo};
