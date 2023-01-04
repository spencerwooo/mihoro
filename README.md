# clashrup

> Simple CLI to manage your systemd clash.service and config subscriptions on Linux.

Download, setup, update, and check logs. **No more, no less.** Systemd configuration is created with reference from
[*Running Clash as a service*](https://github.com/Dreamacro/clash/wiki/Running-Clash-as-a-service). No root privilege is
required. `clash.service` is created under user systemd by default. Friendly Python alternative and also where
`clashrup` got its name - [clashup](https://github.com/felinae98/clashup).

![clashrup setup and status](https://user-images.githubusercontent.com/32114380/210215197-e326ab4f-6b9e-40ee-9459-1ecededc869c.png)

## Installation

Download prebuilt binary for Linux from [releases](https://github.com/spencerwooo/clashrup/releases/latest). Move under
`/usr/local/bin` (system-wide), `~/.local/bin` (user) or any other directory in your `$PATH`.

Alternatively, clone the repo and install from source:

```bash
cargo install --path .
```

## Usage

```
Simple CLI to manage your systemd clash.service and config subscriptions on Linux.

Usage: clashrup [OPTIONS] [COMMAND]

Commands:
  setup        Setup clashrup by downloading clash binary and remote config
  update       Update clash remote config, mmdb, and restart clash.service
  apply        Apply clash config override and restart clash.service
  start        Start clash.service with systemctl
  status       Check clash.service status with systemctl
  stop         Stop clash.service with systemctl
  restart      Restart clash.service with systemctl
  log          Check clash.service logs with journalctl
  proxy        Output and copy proxy export shell commands
  proxy-unset  Output and copy proxy unset shell commands
  uninstall    Uninstall and remove clash and config
  help         Print this message or the help of the given subcommand(s)

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
# external-ui: folder
```

Field `clash_config` holds a subset of supported config overrides for clash's `config.yaml`. Inside, `port`,
`socks_port`, `mode`, and `log_level` are required. Other fields are optional. For a full list of configurable clash
`config.yaml` fields, see [clash - Configuration](https://github.com/Dreamacro/clash/wiki/configuration).

## License

[MIT](LICENSE)
