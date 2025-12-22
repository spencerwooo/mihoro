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

![screenshot](https://github.com/user-attachments/assets/f1120e69-650e-4714-9f57-2fe793115d13)

## Install

```shell
curl -fsSL https://ghfast.top/https://raw.githubusercontent.com/spencerwooo/mihoro/main/install.sh | sh -
```

> [!IMPORTANT]
> `mihoro` is installed to `~/.local/bin` by default. Ensure this is on your `$PATH`.

## Setup

`mihoro`, like `mihomo`, is a config-based CLI client.

After installing `mihoro`, initialize its config `~/.config/mihoro.toml` first by:

```bash
mihoro setup
```

The default config will be generated:

```toml
remote_mihomo_binary_url = ""
remote_config_url = ""
mihomo_binary_path = "~/.local/bin/mihomo"
mihomo_config_root = "~/.config/mihomo"
user_systemd_root = "~/.config/systemd/user"
mihoro_user_agent = "mihoro"

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
geoip = "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geoip.dat"
geosite = "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geosite.dat"
mmdb = "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/country.mmdb"
```

**Before doing anything, fill in:**

* `remote_mihomo_binary_url`, the `.gz` download url found in [`mihomo`'s GitHub release](https://github.com/MetaCubeX/mihomo/releases/latest).
* `remote_config_url`, your remote `mihomo` or `clash` subscription url.

Example:

```toml
remote_mihomo_binary_url = "https://ghfast.top/https://github.com/MetaCubeX/mihomo/releases/download/v1.19.3/mihomo-linux-amd64-v1.19.3.gz"
remote_config_url = "https://tt.vg/freeclash"  # DO NOT USE THIS IF YOU CAN!
```

Customize other settings as needed, then, run setup once more:

```bash
mihoro setup
```

... to start downloading `mihomo` binary and your remote configurations.

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

To overwrite the current mihomo binary for a new version (provided you have updated `remote_mihomo_binary_url`):

```bash
mihoro setup --overwrite
``` 

Shell auto-completions are available under `mihoro completions` for bash, fish, zsh:

```bash
# For bash:
mihoro completions bash > $XDG_CONFIG_HOME/bash_completion/mihoro  # or /etc/bash_completion.d/mihoro

# For fish:
mihoro completions fish > $HOME/.config/fish/completions/mihoro.fish

# For zsh:
mihoro completions zsh > $XDG_CONFIG_HOME/zsh/completions/_mihoro  # or to one of your $fpath directories
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

## Dashboard

On controlling `mihomo` itself, we recommend using a web-based dashboard. Some options include [metacubexd](https://github.com/MetaCubeX/metacubexd), [zashboard](https://github.com/Zephyruso/zashboard), or [yacd](https://github.com/MetaCubeX/Yacd-meta).

Web-based dashboards require enabling `external_controller` under `[mihomo_config]`. Applying this config will expose `mihomo`'s control API under this address, which you can then configure your dashboard to use this as its backend.

You can also put the static files of these dashboards under the `external_ui` directory if defined. In this case, `mihomo` will serve the dashboard locally under `{external_controller}/ui`. Please refer to the official documentation of mihomo for more information: [docs/external_controller](https://wiki.metacubex.one/config/general/#api), [docs/external_ui](https://wiki.metacubex.one/config/general/#_7).

## License

[MIT](LICENSE)
