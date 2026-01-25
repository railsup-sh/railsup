/// Print a success message with checkmark
pub fn success(msg: &str) {
    println!("  ✓ {}", msg);
}

/// Print an error message with X
pub fn error(msg: &str) {
    println!("  ✗ {}", msg);
}

/// Print a warning message
pub fn warn(msg: &str) {
    println!("  ⚠ {}", msg);
}

/// Print a dimmed/secondary message
pub fn dim(msg: &str) {
    println!("  {}", msg);
}

/// Print an info message
pub fn info(msg: &str) {
    println!("{}", msg);
}
