//! HTTP download functionality with progress bar
//!
//! Uses ureq for synchronous HTTP requests

use crate::{paths, platform};
use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::Path;
use tar::Archive;

const RUBY_RELEASES_URL: &str = "https://github.com/railsup-sh/ruby/releases/download";
const GITHUB_API_RELEASES: &str = "https://api.github.com/repos/railsup-sh/ruby/releases";

/// Generate the download URL for a Ruby version
pub fn ruby_download_url(version: &str) -> String {
    let os = platform::detect_os();
    let arch = platform::detect_arch();
    format!(
        "{}/v{}/ruby-{}-{}-{}.tar.gz",
        RUBY_RELEASES_URL, version, version, os, arch
    )
}

/// Generate the checksum URL for a Ruby version
pub fn checksum_url(version: &str) -> String {
    let os = platform::detect_os();
    let arch = platform::detect_arch();
    format!(
        "{}/v{}/ruby-{}-{}-{}.tar.gz.sha256",
        RUBY_RELEASES_URL, version, version, os, arch
    )
}

/// Generate the cache filename for a Ruby version
pub fn cache_filename(version: &str) -> String {
    let os = platform::detect_os();
    let arch = platform::detect_arch();
    format!("ruby-{}-{}-{}.tar.gz", version, os, arch)
}

/// Download a file with progress bar
pub fn download_with_progress(url: &str, dest: &Path) -> Result<()> {
    let response = ureq::get(url)
        .call()
        .with_context(|| format!("Failed to download: {}", url))?;

    if response.status() != 200 {
        bail!("Failed to download: HTTP {}", response.status());
    }

    // Get content length for progress bar
    let content_length: u64 = response
        .header("Content-Length")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Create progress bar
    let pb = if content_length > 0 {
        let pb = ProgressBar::new(content_length);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .expect("Invalid progress bar template")
                .progress_chars("#>-"),
        );
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {bytes}")
                .expect("Invalid spinner template"),
        );
        pb
    };

    // Create destination file
    let mut file =
        File::create(dest).with_context(|| format!("Failed to create file: {}", dest.display()))?;

    // Read and write with progress updates
    let mut reader = response.into_reader();
    let mut buffer = [0u8; 8192];
    let mut downloaded: u64 = 0;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("Download complete");
    Ok(())
}

/// Download checksum and verify a file
pub fn verify_checksum(file_path: &Path, version: &str) -> Result<bool> {
    // Download checksum
    let url = checksum_url(version);
    let response = ureq::get(&url)
        .call()
        .with_context(|| format!("Failed to download checksum: {}", url))?;

    if response.status() != 200 {
        bail!("Failed to download checksum: HTTP {}", response.status());
    }

    let checksum_content = response.into_string()?;
    let expected = checksum_content
        .split_whitespace()
        .next()
        .context("Invalid checksum file format")?
        .to_lowercase();

    // Calculate actual checksum
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open file for checksum: {}", file_path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();

    io::copy(&mut reader, &mut hasher)?;

    let actual = format!("{:x}", hasher.finalize());

    Ok(actual == expected)
}

/// Fix shebangs in Ruby bin scripts to point to the correct ruby path
fn fix_shebangs(ruby_dir: &Path) -> Result<()> {
    let bin_dir = ruby_dir.join("bin");
    if !bin_dir.exists() {
        return Ok(());
    }

    let ruby_path = bin_dir.join("ruby");
    let new_shebang = format!("#!{}\n", ruby_path.display());

    for entry in fs::read_dir(&bin_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip directories and the ruby binary itself
        if path.is_dir() || path.file_name().map(|n| n == "ruby").unwrap_or(false) {
            continue;
        }

        // Read the file
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue, // Skip binary files
        };

        // Check if it has a shebang that needs fixing
        if content.starts_with("#!") && content.contains("/ruby") {
            // Find the end of the first line
            if let Some(newline_pos) = content.find('\n') {
                let new_content = format!("{}{}", new_shebang, &content[newline_pos + 1..]);
                fs::write(&path, new_content)?;
            }
        }
    }

    Ok(())
}

/// Extract a tarball to a destination directory
pub fn extract_tarball(tarball: &Path, dest_dir: &Path) -> Result<()> {
    let file = File::open(tarball)
        .with_context(|| format!("Failed to open tarball: {}", tarball.display()))?;

    let decoder = GzDecoder::new(BufReader::new(file));
    let mut archive = Archive::new(decoder);

    // Create destination directory
    fs::create_dir_all(dest_dir)?;

    // Extract to destination
    archive
        .unpack(dest_dir)
        .with_context(|| format!("Failed to extract tarball to: {}", dest_dir.display()))?;

    Ok(())
}

/// Fetch available Ruby versions from GitHub releases
pub fn fetch_available_versions() -> Result<Vec<String>> {
    let response = ureq::get(GITHUB_API_RELEASES)
        .set("User-Agent", "railsup")
        .call()
        .context("Failed to fetch releases from GitHub")?;

    if response.status() != 200 {
        bail!("Failed to fetch releases: HTTP {}", response.status());
    }

    let body = response.into_string()?;
    let releases: Vec<serde_json::Value> =
        serde_json::from_str(&body).context("Failed to parse GitHub releases response")?;

    let mut versions: Vec<String> = releases
        .iter()
        .filter_map(|r| r.get("tag_name"))
        .filter_map(|t| t.as_str())
        .map(|t| t.trim_start_matches('v').to_string())
        .collect();

    // Sort by version (newest first)
    versions.sort_by(|a, b| compare_versions(b, a));
    Ok(versions)
}

/// Compare two version strings (simple semver comparison)
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<u32> = a.split('.').filter_map(|p| p.parse().ok()).collect();
    let b_parts: Vec<u32> = b.split('.').filter_map(|p| p.parse().ok()).collect();

    for (av, bv) in a_parts.iter().zip(b_parts.iter()) {
        match av.cmp(bv) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    a_parts.len().cmp(&b_parts.len())
}

/// Get the series (major.minor) from a version string
pub fn version_series(version: &str) -> String {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        version.to_string()
    }
}

/// Find the latest available version in a series
pub fn find_latest_in_series(series: &str, available: &[String]) -> Option<String> {
    available
        .iter()
        .find(|v| version_series(v) == series)
        .cloned()
}

/// Check if a version is available
pub fn is_version_available(version: &str) -> Result<bool> {
    let url = ruby_download_url(version);
    let response = ureq::head(&url).call();

    match response {
        Ok(r) => Ok(r.status() == 200),
        Err(ureq::Error::Status(404, _)) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Download and install a Ruby version
pub fn download_ruby(version: &str, force: bool) -> Result<()> {
    let dest = paths::ruby_version_dir(version);

    // Check if already installed
    if dest.exists() && !force {
        println!(
            "Ruby {} is already installed at {}",
            version,
            dest.display()
        );
        return Ok(());
    }

    // Ensure directories exist
    paths::ensure_dirs()?;

    // Cache path
    let filename = cache_filename(version);
    let cache_path = paths::cache_dir().join(&filename);

    // Download if not cached
    if !cache_path.exists() {
        let url = ruby_download_url(version);
        println!("Downloading {}...", filename);
        download_with_progress(&url, &cache_path)?;

        // Verify checksum
        println!("Verifying checksum...");
        if !verify_checksum(&cache_path, version)? {
            fs::remove_file(&cache_path)?;
            bail!("Checksum verification failed. The download may be corrupted.");
        }
    } else {
        println!("Using cached {}...", filename);
    }

    // Remove existing installation if force
    if dest.exists() && force {
        fs::remove_dir_all(&dest)?;
    }

    // Extract
    println!("Extracting to {}...", dest.display());

    // The tarball extracts to a directory named ruby-{version}
    // We need to extract to parent and then we're done
    let parent = dest.parent().expect("Ruby dir should have parent");
    extract_tarball(&cache_path, parent)?;

    // Fix shebangs to point to the installed ruby path
    fix_shebangs(&dest)?;

    // Create gems directory for this version
    let gems_dir = paths::gems_version_dir(version);
    fs::create_dir_all(&gems_dir)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruby_download_url_format() {
        let url = ruby_download_url("4.0.1");
        assert!(url.contains("github.com/railsup-sh/ruby/releases"));
        assert!(url.contains("v4.0.1"));
        assert!(url.contains("ruby-4.0.1"));
        assert!(url.ends_with(".tar.gz"));
    }

    #[test]
    fn checksum_url_format() {
        let url = checksum_url("4.0.1");
        assert!(url.contains("github.com/railsup-sh/ruby/releases"));
        assert!(url.ends_with(".sha256"));
    }

    #[test]
    fn cache_filename_format() {
        let filename = cache_filename("4.0.1");
        assert!(filename.starts_with("ruby-4.0.1"));
        assert!(filename.ends_with(".tar.gz"));
    }
}
