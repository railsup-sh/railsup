# RailsUp

The better way to install and run Ruby on Rails. Bring your agents. They're welcome.

## What

A single Rust binary that simplifies Ruby on Rails development. RailsUp manages Ruby versions and Rails projects so you can focus on building.

## Quick Start

```bash
# Install railsup via bkt
curl -fsSL https://bkt.sh/adamstac/railsup/install.sh | sh
bkt install adamstac/railsup

# Create and run a new Rails app
railsup new myapp
cd myapp
railsup dev
```

That's it. You're building.

## Shell Integration

Add railsup's Ruby to your PATH permanently:

```bash
# Add to ~/.zshrc or ~/.bashrc (place AFTER any rbenv/asdf/rvm init)
eval "$(railsup shell-init)"
```

Then just use Ruby commands directly:

```bash
bin/rails generate scaffold Post body:text
bundle install
ruby script.rb
```

No `railsup exec` prefix needed.

## Commands

```
railsup new <name>              Create a new Rails application
railsup dev                     Start the development server
railsup shell-init              Output shell integration script
railsup ruby install <version>  Install a Ruby version
railsup ruby list [--available] List installed/available Ruby versions
railsup ruby default <version>  Set default Ruby version
railsup ruby remove <version>   Remove a Ruby version
railsup which <command>         Show path to command (ruby, gem, bundle)
railsup exec <command>          Run command with railsup Ruby environment
railsup --help                  Show help
railsup --version               Show version
```

### Options

```
railsup new <name> [--force]     Overwrite existing directory
railsup dev [-p, --port PORT]    Use custom port (default: 3000)
```

## How It Works

RailsUp downloads prebuilt Ruby binaries from [railsup-sh/ruby](https://github.com/railsup-sh/ruby) and manages them in `~/.railsup/ruby/`. No compilation needed.

When you run `railsup new` or `railsup dev` without Ruby installed, RailsUp automatically bootstraps the recommended version.

## Platforms

| Platform | Status |
|----------|--------|
| macOS ARM64 (Apple Silicon) | Supported |
| macOS x86_64 (Intel) | Supported |
| Linux x86_64 | Supported |
| Linux ARM64 | Supported |

## Installation

### Via bkt (recommended)

```bash
curl -fsSL https://bkt.sh/install | sh
bkt install railsup-sh/railsup
```

### Manual Download

Download from [GitHub Releases](https://github.com/railsup-sh/railsup/releases):

```bash
# macOS ARM64
curl -LO https://github.com/railsup-sh/railsup/releases/download/v0.3.5/railsup-aarch64-apple-darwin.tar.gz
tar xzf railsup-aarch64-apple-darwin.tar.gz
sudo mv railsup /usr/local/bin/

# macOS x86_64
curl -LO https://github.com/railsup-sh/railsup/releases/download/v0.3.5/railsup-x86_64-apple-darwin.tar.gz

# Linux x86_64
curl -LO https://github.com/railsup-sh/railsup/releases/download/v0.3.5/railsup-x86_64-unknown-linux-gnu.tar.gz

# Linux ARM64
curl -LO https://github.com/railsup-sh/railsup/releases/download/v0.3.5/railsup-aarch64-unknown-linux-gnu.tar.gz
```

## Directory Structure

```
~/.railsup/
├── ruby/           # Ruby installations
│   └── 4.0.1/
├── gems/           # Per-version gems
│   └── 4.0.1/
├── cache/          # Downloaded tarballs
└── config.toml     # Global config (default Ruby version)
```

## Troubleshooting

### Ruby download fails

Check your network connection and try again:

```bash
railsup ruby install 4.0.1
```

### Bundle install fails

If `railsup new` fails during gem installation:

1. Check your Ruby version: `railsup which ruby && ruby --version`
2. Try running manually: `cd myapp && bundle install`

### Server won't start

If `railsup dev` fails:

1. Ensure you're in a Rails directory (or subdirectory)
2. Try running manually: `bundle exec rails server`

### Directory already exists

Use `--force` to overwrite:

```bash
railsup new myapp --force
```

### rbenv/asdf conflict

If you see errors like `rbenv: version 'ruby-4.0.1' is not installed`, your system has another Ruby version manager that's intercepting commands.

Use `railsup exec` to bypass:

```bash
# Instead of:
bin/rails generate scaffold Post body:text

# Use:
railsup exec bin/rails generate scaffold Post body:text
```

The `exec` command runs any command with railsup's Ruby environment, bypassing rbenv/asdf/rvm shims.

## Status

v0.3.5 — Shell integration. Ruby version management. AI agent context. Four platforms.

## License

MIT

## Trademark Notice

The Rails trademarks are the intellectual property of David Heinemeier Hanson, and exclusively licensed to the Rails Foundation. Uses of ‘Rails’ and ‘Ruby on Rails’ in this website are for identification purposes only and do not imply an endorsement by or affiliation with Rails, the trademark owner, or the Rails Foundation.
