# Variables
binary_name := "railsup"
user_bin := if os() == "macos" { "~/bin" } else { "~/.local/bin" }
version := `git describe --tags --always --dirty 2>/dev/null || echo "dev"`

# List available commands
_default:
    @just --list

# Build debug binary
build:
    cargo build

# Build release binary
release:
    cargo build --release

# Run tests
test:
    cargo test

# Run tests including ignored (requires Ruby + Rails)
test-all:
    cargo test -- --ignored

# Check formatting and lints
check:
    cargo fmt --check
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Install to user bin directory
install: release
    mkdir -p {{user_bin}}
    cp target/release/{{binary_name}} {{user_bin}}/{{binary_name}}
    @echo "Installed {{user_bin}}/{{binary_name}}"

# Create symlink in user bin to development build
link: release
    mkdir -p {{user_bin}}
    ln -sf {{justfile_directory()}}/target/release/{{binary_name}} {{user_bin}}/{{binary_name}}
    @echo "Linked {{user_bin}}/{{binary_name}} -> {{justfile_directory()}}/target/release/{{binary_name}}"

# Remove symlink from user bin
unlink:
    #!/usr/bin/env bash
    if [ -L {{user_bin}}/{{binary_name}} ]; then
        rm -f {{user_bin}}/{{binary_name}}
        echo "Unlinked {{user_bin}}/{{binary_name}}"
    else
        echo "No symlink found at {{user_bin}}/{{binary_name}}"
    fi

# Show version info
version:
    @echo "Version: {{version}}"
    ./target/release/{{binary_name}} --version 2>/dev/null || cargo run -- --version

# Create and push a version tag (triggers GitHub Actions release)
tag ver:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Creating tag v{{ver}}..."
    git tag -a "v{{ver}}" -m "Release {{ver}}"
    echo "Pushing tag to origin..."
    git push origin "v{{ver}}"
    echo ""
    echo "Tagged and pushed v{{ver}}"
    echo "GitHub Actions will build and publish to bkt.sh"

# Delete a tag locally and remotely
tag-delete ver:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Deleting tag v{{ver}} locally..."
    git tag -d "v{{ver}}" || true
    echo "Deleting tag v{{ver}} from origin..."
    git push origin --delete "v{{ver}}" || true
    echo "Deleted v{{ver}}"

# Quick dev cycle: build and test new app creation
demo name="testapp":
    #!/usr/bin/env bash
    set -e
    cargo build --release
    rm -rf /tmp/{{name}}
    ./target/release/railsup new {{name}}
    cd /tmp/{{name}} && ../{{justfile_directory()}}/target/release/railsup dev
