/// Print a success message with checkmark
pub fn success(msg: &str) {
    println!("  ✓ {}", msg);
}

/// Print an error message with X
/// Uses stderr to avoid being captured by shell eval
pub fn error(msg: &str) {
    eprintln!("  ✗ {}", msg);
}

/// Print a warning message
/// Uses stderr to avoid being captured by shell eval
pub fn warn(msg: &str) {
    eprintln!("  ⚠ {}", msg);
}

/// Print a dimmed/secondary message
pub fn dim(msg: &str) {
    println!("  {}", msg);
}

/// Print an info message
pub fn info(msg: &str) {
    println!("{}", msg);
}
