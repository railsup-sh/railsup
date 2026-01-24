//! Platform detection for Ruby binary downloads

/// Detect the operating system for download URL construction
pub fn detect_os() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "darwin"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        panic!("Unsupported operating system. railsup only supports macOS and Linux.");
    }
}

/// Detect the CPU architecture for download URL construction
pub fn detect_arch() -> &'static str {
    #[cfg(target_arch = "aarch64")]
    {
        "arm64"
    }
    #[cfg(target_arch = "x86_64")]
    {
        "x86_64"
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        panic!("Unsupported architecture. railsup only supports arm64 and x86_64.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_os_returns_valid_value() {
        let os = detect_os();
        assert!(os == "darwin" || os == "linux");
    }

    #[test]
    fn detect_arch_returns_valid_value() {
        let arch = detect_arch();
        assert!(arch == "arm64" || arch == "x86_64");
    }
}
