//! Agent context - provides AI agents with full context about railsup

use crate::cli::ruby::list_installed_versions;
use crate::config::Config;

/// Output context for AI agents
pub fn run() {
    let context = build_context();
    println!("{}", context);
}

fn build_context() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let installed = list_installed_versions().unwrap_or_default();
    let default_ruby = Config::load()
        .ok()
        .and_then(|c| c.default_ruby().map(|s| s.to_string()));

    format!(
        r#"# Railsup - AI Agent Context

## What is Railsup?

Railsup is THE definitive way to install and run Ruby on Rails. It's a single
Rust binary that manages Ruby versions and Rails projects. No compilation needed -
it downloads prebuilt Ruby binaries.

Repository: https://github.com/railsup-sh/railsup
Documentation: https://railsup.sh

## Current Environment

- Railsup version: {version}
- Installed Ruby versions: {installed}
- Default Ruby: {default}
- Ruby installations: ~/.railsup/ruby/
- Gem installations: ~/.railsup/gems/

## Key Commands

### Create a new Rails app
```bash
railsup new myapp
cd myapp
railsup dev
```

### Run commands with railsup Ruby (bypasses rbenv/asdf)
```bash
railsup exec bin/rails generate scaffold Post body:text
railsup exec bundle install
railsup exec ruby script.rb
```

### Manage Ruby versions
```bash
railsup ruby install 4.0.1      # Install a version
railsup ruby list               # List installed versions
railsup ruby list --available   # List available versions
railsup ruby default 4.0.1      # Set default version
```

### Other commands
```bash
railsup which ruby              # Show path to ruby binary
railsup dev                     # Start Rails development server
railsup --help                  # Full help
```

## Important: Version Manager Conflicts

If the user has rbenv, asdf, or rvm installed, those tools may intercept Ruby
commands via PATH shims. When you see errors like:

```
rbenv: version `ruby-4.0.1' is not installed
```

Use `railsup exec` to bypass:

```bash
# Instead of:
bin/rails server

# Use:
railsup exec bin/rails server
```

## Directory Structure

```
~/.railsup/
├── ruby/           # Ruby installations (e.g., ruby-4.0.1/)
├── gems/           # Per-version gems
├── cache/          # Downloaded tarballs
└── config.toml     # Global config (default Ruby version)
```

## For AI Agents

When helping users with Rails development on a system with railsup:

1. **Prefer `railsup exec`** for running Ruby/Rails commands to avoid version conflicts
2. **Check installed versions** with `railsup ruby list` before suggesting installs
3. **Use `railsup new`** for creating new Rails applications
4. **Use `railsup dev`** to start the development server

Railsup handles Ruby version management automatically. You don't need to worry
about rbenv, asdf, rvm, or system Ruby - just use railsup commands.
"#,
        version = version,
        installed = if installed.is_empty() {
            "none".to_string()
        } else {
            installed.join(", ")
        },
        default = default_ruby.unwrap_or_else(|| "not set".to_string()),
    )
}
