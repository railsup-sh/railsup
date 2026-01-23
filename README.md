# RailsUp

The better way to install and run Ruby on Rails. Bring your agents. They're welcome.

## What

A single Rust binary that simplifies Ruby on Rails development.

## Quick Start

```bash
railsup new myapp
cd myapp
railsup dev
```

That's it. You're building.

## Commands (v0.1)

```
railsup new <name>    Create a new Rails application
railsup dev           Start the development server
railsup --help        Show help
railsup --version     Show version
```

### Options

```
railsup new <name> [--force]     Overwrite existing directory
railsup dev [-p, --port PORT]    Use custom port (default: 3000)
```

## Requirements

- **Ruby 3.3+** — RailsUp detects your Ruby installation
- **macOS** — v0.1 supports macOS ARM64 only

## Installation

Download from [GitHub Releases](https://github.com/thechangelog/railsup/releases):

```bash
curl -LO https://github.com/thechangelog/railsup/releases/download/v0.1.0/railsup-aarch64-apple-darwin.tar.gz
tar xzf railsup-aarch64-apple-darwin.tar.gz
sudo mv railsup /usr/local/bin/
```

## Troubleshooting

### Ruby not found

RailsUp requires Ruby 3.3+. Install with your preferred version manager:

```bash
# mise (recommended)
mise install ruby@3.3

# rbenv
rbenv install 3.3.0

# asdf
asdf install ruby 3.3.0
```

### Bundle install fails

If `railsup new` fails during gem installation:

1. Check your Ruby version: `ruby --version`
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

## Status

v0.1 — Early development. Two commands that work.

## License

MIT
