# AGENTS.md

Notes for agents working in this repo.

## Project overview

Mihoro is a Rust CLI for managing Mihomo on Linux. It handles:
- Initializing and updating the Mihomo binary
- Managing remote configuration subscriptions (YAML configs)
- Bootstrapping config interactively via `mihoro init`
- Applying config overrides via TOML (local settings override remote YAML)
- Managing the per-user systemd service
- Managing optional web dashboard assets
- Exporting proxy environment variables for shells
- Self-upgrading to the latest GitHub release

## Build and development commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run -- [args]

# Check code
cargo check --all-targets

# Format
cargo fmt --all
cargo fmt --all -- --check  # Verify formatting

# Lint
cargo clippy

# Run tests
cargo test

# Local installation
cargo install --path .
```

## CI commands

From `.github/workflows/ci.yml`:
```bash
cargo fmt --all -- --check
cargo clippy
cargo check --all-targets
```

## Architecture

### Module structure

```
src/
├── main.rs       # CLI entry point, Clap parsing, command dispatch
├── init.rs       # `mihoro init` flow: bootstrap config, prompt for subscription URL, stage reporting
├── mihoro.rs     # Core Mihoro struct with init/update/apply/uninstall helpers
├── config.rs     # Config (TOML) and MihomoConfig parsing with serde defaults
├── ui.rs         # Dashboard source selection and UI asset installation
├── resolve_mihomo_bin.rs # Resolve/download mihomo release artifacts for supported architectures
├── utils.rs      # File I/O, download, gzip extraction, base64 decoding
├── systemctl.rs  # Fluent wrapper around systemctl commands
├── cmd.rs        # Clap derive enums for CLI structure
├── proxy.rs      # Shell-specific proxy env var generation
├── upgrade.rs    # Self-upgrade functionality using self_update crate
└── cron.rs       # Auto-update cron job management
```

### Key pieces

1. **Config override system**: merges local TOML overrides with remote YAML configs
   - `Config`: Main TOML config at `~/.config/mihoro.toml`
   - `MihomoConfig`: Mihomo-specific settings using `#[serde(default)]` extensively
   - `MihomoYamlConfig`: Parses remote YAML with `#[serde(flatten)]` to preserve unrecognized fields
   - Only mihomo_config fields are overridden; remote YAML fields pass through unchanged

2. **Systemctl builder**: method chaining for systemd commands
   ```rust
   Systemctl::new().start("mihomo.service").execute()?
   ```

3. **Init flow**: `mihoro init` is the main onboarding path
   - `bootstrap_config()` creates the default TOML config if missing
   - Interactive runs prompt for `remote_config_url` and continue in the same command
   - `--yes` is for non-interactive use and expects required fields to already be present
   - Stage reports make repeat runs safe and easier to follow

4. **Mihoro**: main struct holding config and derived paths
   - All methods return `anyhow::Result<T>` for consistent error handling
   - Uses Tokio async for downloads

5. **Self-upgrade**: updates from GitHub releases
   - `upgrade::run_upgrade()`: Downloads and replaces the current binary
   - `upgrade::check_for_update()`: Checks for new versions without installing
   - Uses `self_update` crate with GitHub backend
   - Runs in `tokio::task::spawn_blocking` to avoid async runtime conflicts
   - Release artifacts must be named `mihoro-<version>-<target>.tar.gz`

### Configuration flow

1. `mihoro init` creates `~/.config/mihoro.toml` if it does not exist
2. Interactive init prompts for the remote subscription URL when `remote_config_url` is empty
3. Remote YAML config is downloaded from the subscription URL
4. Local TOML overrides are merged into the final `config.yaml`
5. The user systemd service is written, enabled, and started

### Runtime paths

- Config: `~/.config/mihoro.toml`
- Mihomo binary: `~/.local/bin/mihomo`
- Mihomo config: `~/.config/mihomo/config.yaml`
- Systemd service: `~/.config/systemd/user/mihomo.service`

## Dependencies

- `clap` 4.5: CLI argument parsing with derive macros
- `tokio` 1.44: Async runtime (full features)
- `serde` + `serde_yaml`: Serialization/deserialization
- `reqwest` 0.12: HTTP client with streaming support
- `anyhow`: Error handling
- `colored`: Terminal colors
- `indicatif`: Progress bars for downloads
- `self_update` 0.42: Self-upgrade functionality with GitHub releases backend

## Code style

- Edition: Rust 2021
- Formatting: `rustfmt.toml` (max line width 100, hard tabs)
- Linting: `clippy.toml` sets thresholds for complexity/argument count
- Unit tests live alongside the modules under `src/`
- `cargo test` currently discovers 32 unit tests

## Shell integration

Proxy commands detect shell type (bash/zsh/fish) and generate appropriate export/unset commands for `eval $(mihoro proxy export)` usage.
