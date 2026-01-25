# Implementation Mandate: Shell Integration

This document specifies HOW to implement PEP-0001 (Shell Integration for Automatic PATH Management).

## Command Structure

### CLI Addition

```rust
// src/cli/mod.rs
#[derive(Subcommand)]
pub enum Commands {
    // ... existing commands ...

    /// Output shell integration script
    ShellInit {
        /// Shell type (zsh, bash, fish). Auto-detected if not specified.
        #[arg(long)]
        shell: Option<String>,
    },
}
```

### New Module

Create `src/cli/shell_init.rs`:

```rust
//! Shell initialization - outputs shell config for PATH integration

use crate::cli::which::resolve_ruby_version;
use crate::paths;
use anyhow::Result;
use std::env;

pub fn run(shell: Option<String>) -> Result<()> {
    let shell_type = shell.unwrap_or_else(detect_shell);
    let output = generate_init(&shell_type)?;
    println!("{}", output);
    Ok(())
}

fn detect_shell() -> String {
    env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(String::from))
        .unwrap_or_else(|| "bash".to_string())
}

fn generate_init(shell: &str) -> Result<String> {
    let version = resolve_ruby_version()?;
    let ruby_bin = paths::ruby_bin_dir(&version);
    let gem_home = paths::gems_version_dir(&version);
    let gem_bin = gem_home.join("bin");

    match shell {
        "fish" => Ok(generate_fish(&ruby_bin, &gem_home, &gem_bin)),
        _ => Ok(generate_posix(&ruby_bin, &gem_home, &gem_bin)), // bash, zsh
    }
}
```

## Shell Output Formats

### POSIX (bash/zsh)

```bash
# Railsup shell integration
# Add to your ~/.zshrc or ~/.bashrc:
#   eval "$(railsup shell-init)"

export PATH="$HOME/.railsup/ruby/ruby-4.0.1/bin:$HOME/.railsup/gems/4.0.1/bin:$PATH"
export GEM_HOME="$HOME/.railsup/gems/4.0.1"
export GEM_PATH="$HOME/.railsup/gems/4.0.1"
```

### Fish

```fish
# Railsup shell integration
# Add to your ~/.config/fish/config.fish:
#   railsup shell-init | source

set -gx PATH $HOME/.railsup/ruby/ruby-4.0.1/bin $HOME/.railsup/gems/4.0.1/bin $PATH
set -gx GEM_HOME $HOME/.railsup/gems/4.0.1
set -gx GEM_PATH $HOME/.railsup/gems/4.0.1
```

## Implementation Steps

### Phase 1: Basic Implementation

1. **Create `src/cli/shell_init.rs`**
   - `run(shell: Option<String>)` entry point
   - `detect_shell()` from $SHELL env var
   - `generate_posix()` for bash/zsh
   - `generate_fish()` for fish

2. **Update `src/cli/mod.rs`**
   - Add `ShellInit` variant to Commands enum
   - Add `pub mod shell_init;`

3. **Update `src/main.rs`**
   - Handle `Commands::ShellInit { shell }`

4. **Test manually**
   ```bash
   cargo build
   ./target/debug/railsup shell-init
   eval "$(./target/debug/railsup shell-init)"
   which ruby  # Should show ~/.railsup/ruby/...
   ```

### Phase 2: Polish

5. **Add helpful comments in output**
   - Instructions for adding to shell profile
   - Note about restarting terminal

6. **Handle edge cases**
   - No Ruby installed → helpful error message
   - No default set → use latest installed

7. **Update README**
   - Add shell integration section
   - Show one-liner setup

### Phase 3: Future Enhancements (Not Required Now)

- Directory-aware version switching (cd hook)
- `railsup doctor` integration to verify shell init
- Auto-prompt after `railsup ruby install`

## File Structure

```
src/
├── cli/
│   ├── mod.rs          # Add ShellInit command
│   ├── shell_init.rs   # NEW - shell integration output
│   └── ...
└── main.rs             # Handle ShellInit
```

## Testing

### Manual Testing

```bash
# Build
cargo build

# Test output
./target/debug/railsup shell-init
./target/debug/railsup shell-init --shell bash
./target/debug/railsup shell-init --shell fish

# Test integration
eval "$(./target/debug/railsup shell-init)"
ruby --version        # Should show railsup Ruby
which ruby            # Should show ~/.railsup/ruby/...
gem --version         # Should work
bundle --version      # Should work (if installed)
```

### Automated Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_shell_from_env() {
        std::env::set_var("SHELL", "/bin/zsh");
        assert_eq!(detect_shell(), "zsh");
    }

    #[test]
    fn generate_posix_contains_path() {
        let output = generate_posix(...);
        assert!(output.contains("export PATH="));
        assert!(output.contains("GEM_HOME="));
    }

    #[test]
    fn generate_fish_uses_set() {
        let output = generate_fish(...);
        assert!(output.contains("set -gx PATH"));
    }
}
```

## Dependencies

No new dependencies required. Uses existing:
- `crate::paths` for Ruby/gem directories
- `crate::cli::which::resolve_ruby_version` for version detection
- `std::env` for shell detection

## Acceptance Criteria

1. `railsup shell-init` outputs valid shell script
2. Auto-detects zsh/bash/fish from $SHELL
3. `--shell` flag overrides detection
4. After `eval "$(railsup shell-init)"`:
   - `ruby --version` shows railsup Ruby
   - `which ruby` shows `~/.railsup/ruby/...`
   - `bin/rails` works without `railsup exec`
