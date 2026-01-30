//! Fixture loading for integration tests

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// A test fixture representing a project structure
pub struct Fixture {
    /// Name of the fixture
    pub name: String,
    /// Path to the fixture directory
    pub path: PathBuf,
    /// Temp directory (if mutable fixture)
    _temp_dir: Option<TempDir>,
}

impl Fixture {
    /// Load a read-only fixture from tests/fixtures/
    pub fn load(name: &str) -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);

        assert!(path.exists(), "Fixture not found: {}", name);

        Self {
            name: name.to_string(),
            path,
            _temp_dir: None,
        }
    }

    /// Load a fixture into a temp directory (for mutation)
    pub fn load_mutable(name: &str) -> Self {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);

        assert!(source.exists(), "Fixture not found: {}", name);

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        copy_dir_recursive(&source, temp_dir.path()).expect("Failed to copy fixture");

        Self {
            name: name.to_string(),
            path: temp_dir.path().to_path_buf(),
            _temp_dir: Some(temp_dir),
        }
    }

    /// Create a fixture pointing to a subdirectory
    pub fn subdir(&self, subpath: &str) -> Self {
        let new_path = self.path.join(subpath);
        assert!(
            new_path.exists(),
            "Subdirectory not found: {}",
            new_path.display()
        );

        Self {
            name: format!("{}/{}", self.name, subpath),
            path: new_path,
            _temp_dir: None,
        }
    }
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_fixture_exists() {
        // This test will pass once we create the rails-8-app fixture
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join("rails-8-app");

        if path.exists() {
            let fixture = Fixture::load("rails-8-app");
            assert_eq!(fixture.name, "rails-8-app");
            assert!(fixture.path.exists());
        }
    }
}
