<div align="center">
  <div><img src="https://github.com/user-attachments/assets/b292facf-b4d0-4087-b33c-e9ffba061e73" alt="mihoro banner" width="512" /></div>

  <a href="https://github.com/spencerwooo/mihoro/actions/workflows/ci.yml">
    <img src="https://github.com/spencerwooo/mihoro/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://github.com/spencerwooo/mihoro/actions/workflows/release.yml">
    <img src="https://github.com/spencerwooo/mihoro/actions/workflows/release.yml/badge.svg" alt="Release">
  </a>
  <a href="https://github.com/spencerwooo/mihoro/releases/latest">
    <img src="https://img.shields.io/github/v/release/spencerwooo/mihoro" alt="GitHub release (latest by date)">
  </a>
</div>

---

**mihoro** - The 🦀 Rust™-based [Mihomo](https://github.com/MetaCubeX/mihomo) CLI client on Linux.

* Setup, update, apply overrides, and manage with systemd. **No more, no less.**
* No root privilege required. Maintains per-user instance.
* First-class support for config subscription.

![Mihoro CLI](https://github.com/user-attachments/assets/381c1b9b-5ce2-49c5-a9f6-62f552b19e4d)

## Install

```shell
curl -fsSL https://ghgo.xyz/https://raw.githubusercontent.com/spencerwooo/mihoro/main/install.sh | sh -
```

> [!IMPORTANT]
> `mihoro` is installed to `~/.local/bin` by default. Ensure this is on your `$PATH`.

## Setup

`mihoro`, like `mihomo`, is a config-based CLI client.

After installing, run `mihoro setup` once to initialize `~/.config/mihoro.toml`. The default config is:

```toml
remote_mihomo_binary_url = ""
remote_config_url = ""
mihomo_binary_path = "~/.local/bin/mihomo"
mihomo_config_root = "~/.config/mihomo"
user_systemd_root = "~/.config/systemd/user"

[mihomo_config]
port = 7890
socks_port = 7891
allow_lan = false
bind_address = "*"
mode = "rule"
log_level = "info"
ipv6 = true
external_controller = "0.0.0.0:9090"
external_ui = "ui"
geodata_mode = false
geo_auto_update = true
geo_update_interval = 24

[mihomo_config.geox_url]
geoip = "https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geoip.dat"
geosite = "https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geosite.dat"
mmdb = "https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/country.mmdb"
```

**Before doing anything, fill in:**

* `remote_mihomo_binary_url`, the `.gz` download url found in [`mihomo`'s GitHub release](https://github.com/MetaCubeX/mihomo/releases/latest).
* `remote_config_url`, your remote `mihomo` or `clash` subscription url.

Example:

```toml
remote_mihomo_binary_url = "https://ghgo.xyz/https://github.com/MetaCubeX/mihomo/releases/download/v1.18.8/mihomo-linux-amd64-compatible-v1.18.8.gz"
remote_config_url = "https://tt.vg/freeclash"  # DO NOT USE THIS IF YOU CAN!
```

Finally, run `mihoro setup` once more, to start downloading `mihomo` binary and your remote configurations.

> [!CAUTION]
>
> :warning: **DISCLAIMER!** Use your own `remote_config_url` at all times! The link provided comes from a **free, third-party** Clash/Mihomo provider, and `mihoro` cannot guarantee its integrity.

## Usage

To configure proxy for the current terminal session:

```bash
eval $(mihoro proxy export)
```

To revert proxy settings:

```bash
eval $(mihoro proxy unset)
```

To check running status of `mihomo` core:

```bash
mihoro status
```

To update subscribed remote config:

```bash
mihoro update
```

To apply settings changes after modifying `mihoro.toml`:

```bash
mihoro apply
```

Full list of commands:

```console
$ mihoro --help
Mihomo CLI client on Linux.

Usage: mihoro [OPTIONS] [COMMAND]

Commands:
  setup           Setup mihoro by downloading mihomo binary and remote config
  update          Update mihomo remote config and restart mihomo.service
  update-geodata  Update mihomo geodata
  apply           Apply mihomo config overrides and restart mihomo.service
  start           Start mihomo.service with systemctl
  status          Check mihomo.service status with systemctl
  stop            Stop mihomo.service with systemctl
  restart         Restart mihomo.service with systemctl
  log             Check mihomo.service logs with journalctl
  proxy           Output proxy export commands
  uninstall       Uninstall and remove mihoro and config
  completions     Generate shell completions for mihoro
  help            Print this message or the help of the given subcommand(s)

Options:
  -m, --mihoro-config <MIHORO_CONFIG>  Path to mihoro config file [default: ~/.config/mihoro.toml]
  -h, --help                           Print help
  -V, --version                        Print version
```

## License

[MIT](LICENSE)
