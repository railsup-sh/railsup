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

# Build release binary to build/ directory for local testing
build-local:
    cargo build --release
    mkdir -p build
    cp target/release/{{binary_name}} build/{{binary_name}}
    @echo "Built build/{{binary_name}}"
    @./build/{{binary_name}} --version

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

    # Normalize input (allow "v0.3.17" or "0.3.17")
    RAW_VER="{{ver}}"
    NORMALIZED_VER="${RAW_VER#v}"

    # Validate semver (X.Y.Z)
    if [[ ! "$NORMALIZED_VER" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "Error: invalid version '$RAW_VER'. Use X.Y.Z (or vX.Y.Z)."
        exit 1
    fi

    # Check for any working tree changes (including untracked), excluding Cargo files
    STATUS_OUTPUT=$(git status --porcelain --untracked-files=all -- ':!Cargo.toml' ':!Cargo.lock')
    if [ -n "$STATUS_OUTPUT" ]; then
        echo "Error: You have uncommitted changes. Commit or stash them first."
        echo "$STATUS_OUTPUT"
        exit 1
    fi

    # Get current version from Cargo.toml
    CURRENT_VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)

    if [ "$CURRENT_VERSION" = "$NORMALIZED_VER" ]; then
        echo "Cargo.toml already at version $NORMALIZED_VER"
    else
        echo "Updating Cargo.toml: $CURRENT_VERSION -> $NORMALIZED_VER"
        # Portable in-place edit (works on macOS + Linux)
        perl -0777 -i -pe "s/^version = \".*\"/version = \"$NORMALIZED_VER\"/m" Cargo.toml

        # Update Cargo.lock
        cargo check --quiet 2>/dev/null || true

        git add Cargo.toml Cargo.lock
        git commit -m "Bump version to $NORMALIZED_VER"
    fi

    if git rev-parse -q --verify "refs/tags/v$NORMALIZED_VER" >/dev/null; then
        echo "Error: tag v$NORMALIZED_VER already exists locally."
        exit 1
    fi

    if git ls-remote --exit-code --tags origin "refs/tags/v$NORMALIZED_VER" >/dev/null 2>&1; then
        echo "Error: tag v$NORMALIZED_VER already exists on origin."
        exit 1
    fi

    echo "Creating tag v$NORMALIZED_VER..."
    git tag -a "v$NORMALIZED_VER" -m "Release $NORMALIZED_VER"
    echo "Pushing commit and tag to origin..."
    git push origin HEAD "v$NORMALIZED_VER"
    echo ""
    echo "Tagged and pushed v$NORMALIZED_VER"
    echo "GitHub Actions will build and publish to bkt.sh"

# Preview what tag would do without making changes
tag-dry-run ver:
    #!/usr/bin/env bash
    set -euo pipefail
    RAW_VER="{{ver}}"
    NORMALIZED_VER="${RAW_VER#v}"
    echo "=== DRY RUN: tag $RAW_VER (normalized: $NORMALIZED_VER) ==="
    echo ""

    # Validate semver (X.Y.Z)
    if [[ ! "$NORMALIZED_VER" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "✗ Invalid version '$RAW_VER'. Expected X.Y.Z (or vX.Y.Z)."
        exit 1
    fi

    # Check for any working tree changes (including untracked), excluding Cargo files
    STATUS_OUTPUT=$(git status --porcelain --untracked-files=all -- ':!Cargo.toml' ':!Cargo.lock')
    if [ -n "$STATUS_OUTPUT" ]; then
        echo "⚠ Warning: You have uncommitted changes"
        echo "$STATUS_OUTPUT"
        echo ""
    fi

    # Get current version from Cargo.toml
    CURRENT_VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)

    if [ "$CURRENT_VERSION" = "$NORMALIZED_VER" ]; then
        echo "✓ Cargo.toml already at version $NORMALIZED_VER"
    else
        echo "-> Would update Cargo.toml: $CURRENT_VERSION -> $NORMALIZED_VER"
        echo "-> Would commit: \"Bump version to $NORMALIZED_VER\""
    fi

    if git rev-parse -q --verify "refs/tags/v$NORMALIZED_VER" >/dev/null; then
        echo "✗ Tag already exists locally: v$NORMALIZED_VER"
    fi
    if git ls-remote --exit-code --tags origin "refs/tags/v$NORMALIZED_VER" >/dev/null 2>&1; then
        echo "✗ Tag already exists on origin: v$NORMALIZED_VER"
    fi

    echo "-> Would create tag: v$NORMALIZED_VER"
    echo "-> Would push commit and tag in one command: git push origin HEAD v$NORMALIZED_VER"
    echo ""
    echo "Run 'just tag $RAW_VER' to execute"

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
