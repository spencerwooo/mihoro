# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Mihoro** is a Rust-based CLI tool for managing Mihomo (formerly Clash), a network proxy client on Linux. It handles:
- Downloading and updating the Mihomo binary
- Managing remote configuration subscriptions (YAML configs)
- Applying config overrides via TOML (local settings override remote YAML)
- Managing the per-user systemd service
- Exporting proxy environment variables for shells

## Build and Development Commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run -- [args]

# Check code without building
cargo check --all-targets

# Format
cargo fmt --all
cargo fmt --all -- --check  # Verify formatting

# Lint
cargo clippy

# Tests
cargo test

# Local installation
cargo install --path .
```

## CI Commands

From `.github/workflows/ci.yml`:
```bash
cargo fmt --all -- --check
cargo clippy
cargo check --all-targets
```

## Architecture

### Module Structure

```
src/
├── main.rs       # CLI entry point, Clap parsing, command dispatch
├── mihoro.rs     # Core Mihoro struct with setup/update/apply/uninstall methods
├── config.rs     # Config (TOML) and MihomoConfig parsing with serde defaults
├── utils.rs      # File I/O, download, gzip extraction, base64 decoding
├── systemctl.rs  # Fluent wrapper around systemctl commands
├── cmd.rs        # Clap derive enums for CLI structure
└── proxy.rs      # Shell-specific proxy env var generation
```

### Key Abstractions

1. **Config Override System**: The core feature that merges local TOML overrides with remote YAML configs
   - `Config`: Main TOML config at `~/.config/mihoro.toml`
   - `MihomoConfig`: Mihomo-specific settings using `#[serde(default)]` extensively
   - `MihomoYamlConfig`: Parses remote YAML with `#[serde(flatten)]` to preserve unrecognized fields
   - Only mihomo_config fields are overridden; remote YAML fields pass through unchanged

2. **Systemctl Fluent Builder**: Method chaining for systemd commands
   ```rust
   Systemctl::new().start("mihomo.service").execute()?
   ```

3. **Mihoro**: Main struct holding config and derived paths
   - All methods return `anyhow::Result<T>` for consistent error handling
   - Uses Tokio async for downloads

### Configuration Flow

1. User edits `~/.config/mihoro.toml` with local overrides
2. Remote YAML config is downloaded from subscription URL
3. `apply()` merges: remote YAML + local TOML overrides → final config.yaml
4. systemd service is restarted to apply changes

### Runtime Paths (Defaults)

- Config: `~/.config/mihoro.toml`
- Mihomo binary: `~/.local/bin/mihomo`
- Mihomo config: `~/.config/mihomo/config.yaml`
- Systemd service: `~/.config/systemd/user/mihomo.service`

## Important Dependencies

- `clap` 4.5: CLI argument parsing with derive macros
- `tokio` 1.44: Async runtime (full features)
- `serde` + `serde_yaml`: Serialization/deserialization
- `reqwest` 0.12: HTTP client with streaming support
- `anyhow`: Error handling
- `colored`: Terminal colors
- `indicatif`: Progress bars for downloads

## Code Style

- Edition: Rust 2021
- Formatting: `rustfmt.toml` (max line width 100, hard tabs)
- Linting: `clippy.toml` sets thresholds for complexity/argument count
- No tests currently exist in the codebase

## Shell Integration

Proxy commands detect shell type (bash/zsh/fish) and generate appropriate export/unset commands for `eval $(mihoro proxy export)` usage.
