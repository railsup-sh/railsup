//! Agent context - provides AI agents with full context about railsup

use crate::cli::ruby::list_installed_versions;
use crate::config::Config;
use std::env;
use std::path::Path;

/// Output context for AI agents
pub fn run() {
    let context = build_context();
    println!("{}", context);
}

/// Detect project context from current directory
fn detect_project_context() -> Option<ProjectContext> {
    let current_dir = env::current_dir().ok()?;

    // Check if we're in a Rails project
    let is_rails = current_dir.join("Gemfile").exists()
        && (current_dir.join("config.ru").exists() || current_dir.join("bin/rails").exists());

    if !is_rails {
        return None;
    }

    // Get app name from directory
    let app_name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    // Check for Ruby version (railsup.toml, .ruby-version, or .tool-versions)
    let ruby_version = find_project_ruby(&current_dir);

    Some(ProjectContext {
        app_name,
        ruby_version,
        path: current_dir.display().to_string(),
    })
}

struct ProjectContext {
    app_name: Option<String>,
    ruby_version: Option<(String, String)>, // (version, source file)
    path: String,
}

/// Detect if shell integration is active (railsup Ruby is in PATH)
fn detect_shell_integration() -> bool {
    let path = env::var("PATH").unwrap_or_default();
    path.contains(".railsup/ruby/")
}

/// Find project Ruby version from config files
fn find_project_ruby(dir: &Path) -> Option<(String, String)> {
    // 1. railsup.toml
    let railsup_toml = dir.join("railsup.toml");
    if railsup_toml.exists() {
        if let Ok(content) = std::fs::read_to_string(&railsup_toml) {
            if let Ok(config) = toml::from_str::<toml::Table>(&content) {
                if let Some(ruby) = config.get("ruby").and_then(|v| v.as_str()) {
                    return Some((ruby.to_string(), "railsup.toml".to_string()));
                }
            }
        }
    }

    // 2. .ruby-version
    let ruby_version_file = dir.join(".ruby-version");
    if ruby_version_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&ruby_version_file) {
            let version = content.trim().trim_start_matches("ruby-").to_string();
            if !version.is_empty() {
                return Some((version, ".ruby-version".to_string()));
            }
        }
    }

    // 3. .tool-versions (asdf)
    let tool_versions = dir.join(".tool-versions");
    if tool_versions.exists() {
        if let Ok(content) = std::fs::read_to_string(&tool_versions) {
            for line in content.lines() {
                if line.starts_with("ruby ") {
                    let version = line.trim_start_matches("ruby ").trim().to_string();
                    return Some((version, ".tool-versions".to_string()));
                }
            }
        }
    }

    None
}

fn build_context() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let installed = list_installed_versions().unwrap_or_default();
    let default_ruby = Config::load()
        .ok()
        .and_then(|c| c.default_ruby().map(|s| s.to_string()));

    // Detect shell integration
    let shell_integrated = detect_shell_integration();

    // Detect project context
    let project_section = if let Some(project) = detect_project_context() {
        let app = project.app_name.unwrap_or_else(|| "unknown".to_string());
        let ruby_info = match project.ruby_version {
            Some((ver, source)) => format!("{} (from {})", ver, source),
            None => "not specified".to_string(),
        };
        format!(
            r#"
## Project Context

- Rails app: {app}
- Project Ruby: {ruby_info}
- Path: {path}
"#,
            app = app,
            ruby_info = ruby_info,
            path = project.path,
        )
    } else {
        String::new()
    };

    // Different content based on shell integration status
    let (tldr, shell_status, commands_section, agent_guidance) = if shell_integrated {
        (
            "**TL;DR: Just run Ruby/Rails commands normally. Shell integration is active.**",
            "- Shell integration: **Active** (railsup Ruby is in PATH)",
            r#"### Run commands directly
```bash
bin/rails generate scaffold Post body:text
bundle install
ruby script.rb
```

### Create a new Rails app
```bash
railsup new myapp
cd myapp
railsup dev
```"#,
            r#"## For AI Agents

**Important:** You likely run each command in a fresh shell where environment
variables don't persist. Even though the user has shell integration configured,
**always use `railsup exec`** for reliability:

```bash
railsup exec bin/rails generate scaffold Post body:text
railsup exec bundle install
railsup exec bin/dev
```

This is not a workaround - it's the correct approach for automation and AI agents.

Shell integration (`eval "$(railsup shell-init)"`) is for humans in interactive
terminals where the environment persists across commands."#,
        )
    } else {
        (
            "**TL;DR: Prefix Ruby/Rails commands with `railsup exec`**",
            "- Shell integration: **Not active** (recommend setting up for easier workflow)",
            r#"### Run commands with railsup Ruby (bypasses rbenv/asdf)
```bash
railsup exec bin/rails generate scaffold Post body:text
railsup exec bundle install
railsup exec ruby script.rb
```

### Create a new Rails app
```bash
railsup new myapp
cd myapp
railsup dev
```"#,
            r#"## For AI Agents

**Always use `railsup exec`** for Ruby/Rails commands:

```bash
railsup exec bin/rails generate scaffold Post body:text
railsup exec bundle install
railsup exec bin/dev
```

This is not a workaround - it's the correct approach for automation and AI agents.
Each command runs in a fresh shell, so `railsup exec` ensures the right Ruby is used.

### For the human user

Suggest they enable shell integration for their interactive terminal:

```bash
# Add to ~/.zshrc or ~/.bashrc
eval "$(railsup shell-init)"
```

Then *they* can run `bin/rails`, `bundle`, etc. directly. But you (the agent)
should still use `railsup exec` for reliability."#,
        )
    };

    format!(
        r#"# Railsup - AI Agent Context

{tldr}

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
{shell_status}
- Ruby installations: ~/.railsup/ruby/
- Gem installations: ~/.railsup/gems/
{project_section}
## Key Commands

{commands_section}

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

## Directory Structure

```
~/.railsup/
├── ruby/           # Ruby installations (e.g., ruby-4.0.1/)
├── gems/           # Per-version gems
├── cache/          # Downloaded tarballs
└── config.toml     # Global config (default Ruby version)
```

{agent_guidance}
"#,
        tldr = tldr,
        version = version,
        installed = if installed.is_empty() {
            "none".to_string()
        } else {
            installed.join(", ")
        },
        default = default_ruby.unwrap_or_else(|| "not set".to_string()),
        shell_status = shell_status,
        project_section = project_section,
        commands_section = commands_section,
        agent_guidance = agent_guidance,
    )
}
