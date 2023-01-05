# clashrup

[![Release](https://github.com/spencerwooo/clashrup/actions/workflows/release.yml/badge.svg)](https://github.com/spencerwooo/clashrup/actions/workflows/release.yml)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/spencerwooo/clashrup)](https://github.com/spencerwooo/clashrup/releases/latest)

Simple CLI to manage your systemd `clash.service` and config subscriptions on Linux.

- Setup, update, apply overrides, and manage via `systemctl`. **No more, no less.**
- Systemd configuration is created with reference from [*Running Clash as a service*](https://github.com/Dreamacro/clash/wiki/Running-Clash-as-a-service).
- No root privilege is required. `clash.service` is created under user systemd by default.
- `clashrup` got its name from [clashup](https://github.com/felinae98/clashup), a friendly Python alternative.

![clashrup setup, update, apply and status](https://user-images.githubusercontent.com/32114380/210590025-35ac3977-60ab-452c-a0f1-7c3d5707b0e1.png)

## Installation

Download prebuilt binary for Linux from [releases](https://github.com/spencerwooo/clashrup/releases/latest). Move under
`/usr/local/bin` (system-wide), `~/.local/bin` (user) or any other directory in your `$PATH`. Example:

```bash
curl -LO https://github.com/spencerwooo/clashrup/releases/download/{VERSION}/clashrup-{TARGET_ARCH}.tar.gz
tar -xvzf clashrup-{TARGET_ARCH}.tar.gz
mv clashrup ~/.local/bin/clashrup
```

Alternatively, clone the repo and install from source:

```bash
cargo install --path .
```

## Usage

```text
Simple CLI to manage your systemd clash.service and config subscriptions on Linux.

Usage: clashrup [OPTIONS] [COMMAND]

Commands:
  setup      Setup clashrup by downloading clash binary and remote config
  update     Update clash remote config, mmdb, and restart clash.service
  apply      Apply clash config overrides and restart clash.service
  start      Start clash.service with systemctl
  status     Check clash.service status with systemctl
  stop       Stop clash.service with systemctl
  restart    Restart clash.service with systemctl
  log        Check clash.service logs with journalctl
  proxy      Proxy export commands, `clashrup proxy --help` to see more
  uninstall  Uninstall and remove clash and config
  help       Print this message or the help of the given subcommand(s)

Options:
  -c, --clashrup-config <CLASHRUP_CONFIG>
          Path to clashrup config file [default: ~/.config/clashrup.toml]
  -h, --help
          Print help information
  -V, --version
          Print version information
```

## Configuration

`clashrup` stores its config at `~/.config/clashrup.toml` by default.

Default config is generated upon setup (with command `setup`) as:

```toml
# ~/.config/clashrup.toml
remote_clash_binary_url = ""
remote_config_url = ""
remote_mmdb_url = "https://cdn.jsdelivr.net/gh/Dreamacro/maxmind-geoip@release/Country.mmdb"
clash_binary_path = "~/.local/bin/clash"
clash_config_root = "~/.config/clash"
user_systemd_root = "~/.config/systemd/user"

[clash_config]
port = 7890
socks_port = 7891
allow_lan = false
bind_address = "*"
mode = "rule"
log_level = "info"
ipv6 = false
external_controller = "127.0.0.1:9090"
# external-ui = "folder"
```

where,

- Field `remote_clash_binary_url` should point to a downloadable gzipped `clash` binary URL
  ([example](https://github.com/MetaCubeX/Clash.Meta/releases/download/v1.14.0/Clash.Meta-linux-amd64-v1.14.0.gz)).
- Field `remote_config_url` should point to your subscription provider's config URL, which will be downloaded to
  `{clash_config_root}/config.yaml` during `setup` and `update`.
- Field `clash_config` holds a subset of supported config overrides for clash's `config.yaml`. Inside, `port`,
  `socks_port`, `mode`, and `log_level` are required. Other fields are optional. For a full list of configurable clash
  `config.yaml` fields, see [clash - Configuration](https://github.com/Dreamacro/clash/wiki/configuration).

If clash binary already exists at `clash_binary_path`, then `remote_clash_binary_url` will be ignored and `setup` will
skip downloading and setting up clash binary. (In this case, `remote_clash_binary_url` can be left empty.)

Other fields should be self-explanatory.

## License

[MIT](LICENSE)
