# PEP-0001: Shell Integration for Automatic PATH Management

## Metadata
- **PEP**: 0001
- **Title**: Shell Integration for Automatic PATH Management
- **Status**: Draft
- **Type**: Feature
- **Created**: 2026-01-24
- **Author**: Development Team

## Abstract

Add `railsup shell-init` command that outputs shell configuration to automatically add railsup's Ruby to PATH. Users add `eval "$(railsup shell-init)"` to their shell profile, eliminating the need for `railsup exec` in most cases.

## Motivation

### The Problem

Today, users must prefix every Ruby/Rails command with `railsup exec`:

```bash
railsup exec bin/rails generate scaffold Post body:text
railsup exec bundle install
railsup exec ruby script.rb
```

This is required because:

1. **Shell doesn't know about railsup's Ruby** - The user's PATH doesn't include `~/.railsup/ruby/ruby-4.0.1/bin`
2. **Shebang lookup fails** - When `bin/rails` has `#!/usr/bin/env ruby`, the shell looks for `ruby` in PATH
3. **Version manager conflicts** - rbenv/asdf shims intercept Ruby commands

### Why This Matters

- **Friction for new users** - Extra prefix on every command
- **Muscle memory** - Experienced Rails devs type `bin/rails`, not `railsup exec bin/rails`
- **AI agents** - Need special instructions to use `railsup exec`
- **Scripts and tooling** - External tools expect `ruby` to just work

### The Vision

Railsup is THE way to build with Rails. That means:
- `bin/rails` should just work
- `bundle install` should just work
- No prefixes, no friction

## Proposed Solution

### New Command: `railsup shell-init`

```bash
$ railsup shell-init
# Railsup shell integration
# Add to your shell profile: eval "$(railsup shell-init)"

export PATH="$HOME/.railsup/ruby/ruby-4.0.1/bin:$HOME/.railsup/gems/4.0.1/bin:$PATH"
export GEM_HOME="$HOME/.railsup/gems/4.0.1"
export GEM_PATH="$HOME/.railsup/gems/4.0.1"
```

### User Setup

One-time addition to `~/.zshrc` or `~/.bashrc`:

```bash
eval "$(railsup shell-init)"
```

### Dynamic Version Resolution

The output should respect:
1. Project `.ruby-version` or `railsup.toml` (if in a project directory)
2. Global default (`~/.railsup/config.toml`)
3. Latest installed version (fallback)

### Shell Support

- **zsh** (default on macOS)
- **bash** (common on Linux)
- **fish** (optional, different syntax)

### Directory-Aware Switching (Future)

Like rbenv/asdf, could hook into `cd` to auto-switch Ruby versions per project. This is a future enhancement, not required for initial implementation.

## Trade-offs

### Advantages

1. **Zero friction** - Commands just work after one-time setup
2. **Familiar pattern** - Same as rbenv, nvm, asdf shell integration
3. **Composable** - Works with existing shell configs
4. **AI-friendly** - Agents don't need special `railsup exec` knowledge

### Disadvantages

1. **Setup required** - User must edit shell profile (one time)
2. **Shell reload** - Changes require new terminal or `source ~/.zshrc`
3. **Complexity** - Another moving part in the system
4. **Conflict potential** - Could conflict with existing rbenv/asdf shell init

### Mitigation

- Clear setup instructions in `railsup --help` and docs
- Detect existing version manager shell integration and warn
- `railsup doctor` can diagnose PATH issues

## Success Criteria

1. After `eval "$(railsup shell-init)"`, user can run `ruby --version` and see railsup's Ruby
2. `bin/rails generate scaffold` works without `railsup exec` prefix
3. `bundle install` works without prefix
4. Works on macOS (zsh) and Linux (bash)
5. `railsup doctor` can verify shell integration is working

## Related Documents

- **PEP-0008**: Exec Command (current workaround)
- **PEP-0009**: Doctor Command (will diagnose shell integration)
- **KB-0009**: Environment Isolation for Ruby Subprocesses

## Open Questions

1. Should we auto-detect shell type or require `--shell zsh` flag?
2. How to handle project-specific Ruby versions (cd hook vs manual)?
3. Should `railsup install` prompt to add shell integration?
