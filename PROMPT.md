# RailsUp v0.1 Implementation

## Objective

Build a Rust CLI that creates and runs Rails applications with opinionated defaults. Two commands: `railsup new` and `railsup dev`.

## Key Requirements

- **`railsup new <name>`** — Create Rails 8.1.2 app with SQLite, Tailwind, Importmap
- **`railsup dev [-p PORT]`** — Start development server (default port 3000)
- **Ruby 3.3+** required (detect, validate, suggest install if missing)
- **Upward search** for Rails root (like git/cargo)
- **Safety checks** — Reject `.`, `/`, `..` in app names
- **Helpful errors** — One actionable suggestion per error

## Rails Command

```bash
rails _8.1.2_ new <name> \
  --database=sqlite3 \
  --css=tailwind \
  --javascript=importmap \
  --skip-jbuilder \
  --skip-action-mailbox \
  --skip-action-text
```

## Dependencies

```toml
clap = { version = "4", features = ["derive"] }
which = "7"
anyhow = "1"
thiserror = "2"
```

## Acceptance Criteria

1. [ ] `cargo build --release` succeeds
2. [ ] `railsup new testapp` creates runnable Rails app
3. [ ] `railsup dev` starts server, prints `Starting Rails on http://localhost:3000`
4. [ ] `railsup dev` works from subdirectories (finds Rails root)
5. [ ] No Ruby → helpful error with install command
6. [ ] `railsup new .` and `railsup new foo/bar` → rejected with clear message
7. [ ] All unit tests pass (`cargo test`)

## Implementation Details

See `.sop/planning/` for full context:
- `design/detailed-design.md` — Technical specification
- `implementation/plan.md` — Step-by-step build plan with code samples
- `idea-honing.md` — All 12 requirement decisions

## Quick Start

```bash
cd railsup  # The Rust project directory
cargo init  # If not already initialized

# Follow implementation/plan.md Step 1-7
# Each step is demoable before moving to the next
```

## Not in Scope (v0.1)

- Ruby installation
- `console`, `db`, `routes` commands
- MCP server
- Config files
- Multi-platform builds (macOS ARM64 only)
