/// Print a success message with checkmark
pub fn success(msg: &str) {
    println!("✓ {}", msg);
}

/// Print an error message to stderr
pub fn error(msg: &str) {
    eprintln!("Error: {}", msg);
}

/// Print an info message
pub fn info(msg: &str) {
    println!("{}", msg);
}
