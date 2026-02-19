use std::path::Path;

#[cfg(target_os = "macos")]
const CERT_FILE_CANDIDATES: &[&str] = &[
    "/opt/homebrew/etc/openssl@3/cert.pem",
    "/usr/local/etc/openssl@3/cert.pem",
    "/etc/ssl/cert.pem",
];

#[cfg(not(target_os = "macos"))]
const CERT_FILE_CANDIDATES: &[&str] = &[
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/pki/tls/certs/ca-bundle.crt",
    "/etc/ssl/cert.pem",
    "/etc/ssl/ca-bundle.pem",
];

#[cfg(target_os = "macos")]
const CERT_DIR_CANDIDATES: &[&str] = &[
    "/opt/homebrew/etc/openssl@3/certs",
    "/usr/local/etc/openssl@3/certs",
    "/etc/ssl/certs",
];

#[cfg(not(target_os = "macos"))]
const CERT_DIR_CANDIDATES: &[&str] = &["/etc/ssl/certs", "/etc/pki/tls/certs"];

/// Resolve TLS cert environment overrides.
///
/// If `SSL_CERT_FILE` or `SSL_CERT_DIR` are missing/invalid, returns best-known
/// existing system paths. Existing valid values are preserved.
pub fn recommended_cert_env(
    current_cert_file: Option<&str>,
    current_cert_dir: Option<&str>,
) -> (Option<String>, Option<String>) {
    let cert_file = if current_cert_file.is_some_and(path_is_file) {
        None
    } else {
        first_existing_file(CERT_FILE_CANDIDATES)
    };

    let cert_dir = if current_cert_dir.is_some_and(path_is_dir) {
        None
    } else {
        first_existing_dir(CERT_DIR_CANDIDATES)
    };

    (cert_file, cert_dir)
}

fn first_existing_file(candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|p| path_is_file(p))
        .map(|p| (*p).to_string())
}

fn first_existing_dir(candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|p| path_is_dir(p))
        .map(|p| (*p).to_string())
}

fn path_is_file(path: &str) -> bool {
    Path::new(path).is_file()
}

fn path_is_dir(path: &str) -> bool {
    Path::new(path).is_dir()
}
