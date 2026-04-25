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

- Setup, update, apply overrides, and manage with systemd. **No more, no less.**
- No root privilege required. Maintains per-user instance.
- First-class support for config subscription.

<img width="1136" height="911" alt="screenshot" src="https://github.com/user-attachments/assets/abfeb381-3ea2-45c8-ac0a-d55f7ba35fbb" />

## Install

```shell
curl -fsSL https://raw.githubusercontent.com/spencerwooo/mihoro/main/install.sh | sh
```

Optionally, download over a mirror:

```shell
curl -fsSL https://raw.githubusercontent.com/spencerwooo/mihoro/main/install.sh | sh -s -- --mirror https://gh-proxy.org
```

> [!IMPORTANT]
> `mihoro` is installed to `~/.local/bin` by default. Ensure this is on your `$PATH`.

## Initialize

`mihoro`, like `mihomo`, is a config-based CLI client.

After installing `mihoro`, run:

```bash
mihoro init
```

If `~/.config/mihoro.toml` does not exist yet, `mihoro init` will create it, prompt for your remote `mihomo` or `clash` subscription URL, save it, then finish the full onboarding flow in the same run.

Upon onboarding, `mihoro` will:

- download the `mihomo` core binary
- download your remote config and apply local overrides
- download geodata and the default web dashboard
- install and enable `mihomo.service`
- start the service and print dashboard URLs for the configured controller

You can also proxy GitHub-hosted runtime downloads by setting `MIHORO_GITHUB_MIRROR` before commands such as `mihoro init` or `mihoro update`:

```shell
MIHORO_GITHUB_MIRROR=https://gh-proxy.org mihoro init
```

Note that this only applies to GitHub-hosted resource downloads and does not affect `mihoro upgrade` yet.

The generated config uses sensible defaults, including `metacubexd` as the managed dashboard:

```toml
remote_config_url = "https://example.com/subscription"
ui = "metacubexd"
mihomo_channel = "stable"
mihomo_binary_path = "~/.local/bin/mihomo"
mihomo_config_root = "~/.config/mihomo"
user_systemd_root = "~/.config/systemd/user"
mihoro_user_agent = "mihoro"
auto_update_interval = 12

[mihomo_config]
port = 7891
socks_port = 7892
mixed_port = 7890
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

By default, `ui = "metacubexd"` enables dashboard management, so `mihoro init` also downloads the web UI assets and serves them from the configured `external_controller`. When the controller binds all interfaces, `mihoro init` prints localhost plus detected non-loopback machine IPs such as LAN or Tailscale/ZeroTier addresses.

`init` is idempotent — re-running it skips any artifacts that are already in place. Use `--force` to re-download everything:

```bash
mihoro init --force
```

For non-interactive environments, pre-populate `remote_config_url` in `mihoro.toml` and use:

```bash
mihoro init --yes
```

Use `--arch` if auto-detection picks the wrong mihomo build for your machine:

```bash
mihoro init --arch amd64-v3
```

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
# or explicitly: mihoro update --config
```

To apply settings changes after modifying `mihoro.toml`:

```bash
mihoro apply
```

To update `mihomo` binary (core) and/or geodata:

```bash
mihoro update --core     # updates core
mihoro update --geodata  # updates geodata
mihoro update --ui       # updates external UI assets
mihoro update --all      # updates config -> geodata -> core -> ui -> restarts mihomo
```

To enable auto-update via cron job:

```bash
mihoro cron enable
```

To disable auto-update:

```bash
mihoro cron disable
```

To check auto-update status:

```bash
mihoro cron status
```

The `auto_update_interval` in `mihoro.toml` controls the update frequency in hours (default: 12, range: 1-24). Set to `0` to disable.

To upgrade `mihoro` itself to the latest version:

```bash
mihoro upgrade
```

Or check for updates without installing:

```bash
mihoro upgrade --check
```

To manually specify a target architecture (useful when auto-detection fails, e.g., on Ubuntu 20.04):

```bash
mihoro upgrade --target x86_64-unknown-linux-musl
mihoro upgrade --target aarch64-unknown-linux-musl
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
  init         Initialize mihoro: download binary, config, geodata, and set up the systemd service
  update       Update mihomo components (config by default)
  apply        Apply mihomo config overrides and restart mihomo.service
  start        Start mihomo.service with systemctl
  status       Check mihomo.service status with systemctl
  stop         Stop mihomo.service with systemctl
  restart      Restart mihomo.service with systemctl
  log          Check mihomo.service logs with journalctl [aliases: logs]
  proxy        Output proxy export commands
  uninstall    Uninstall and remove mihoro and config
  completions  Generate shell completions for mihoro
  cron         Manage auto-update cron job
  upgrade      Upgrade mihoro to the latest version
  help         Print this message or the help of the given subcommand(s)

Options:
  -m, --mihoro-config <MIHORO_CONFIG>  Path to mihoro config file [default: ~/.config/mihoro.toml]
  -h, --help                           Print help
  -V, --version                        Print version
```

## Dashboard

On controlling `mihomo` itself, we recommend using a web-based dashboard. Some options include [metacubexd](https://github.com/MetaCubeX/metacubexd), [zashboard](https://github.com/Zephyruso/zashboard), or [yacd](https://github.com/MetaCubeX/Yacd-meta).

Web-based dashboards require enabling `external_controller` under `[mihomo_config]`. Applying this config will expose `mihomo`'s control API under this address, which you can then configure your dashboard to use this as its backend.

`mihoro` manages dashboard source via top-level `ui` config, which defaults to `metacubexd` and also supports `zashboard`, `yacd-meta`, or `custom:download_url`. The downloaded static files are placed into `mihomo_config.external_ui`. In this case, `mihomo` will serve the dashboard locally under `{external_controller}/ui`. Please refer to the official documentation of mihomo for more information: [docs/external_controller](https://wiki.metacubex.one/config/general/#api), [docs/external_ui](https://wiki.metacubex.one/config/general/#_7).

## License

[MIT](LICENSE)
