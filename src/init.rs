use crate::config::{load_config, validate_config, write_default_if_missing, Config};
use crate::mihoro::{BinaryPlan, Mihoro, StageStatus};

use std::{collections::HashSet, future::Future, net::IpAddr, path::Path};

use anyhow::{bail, Result};
use colored::Colorize;
use dialoguer::Input;
use local_ip_address::list_afinet_netifas;
use reqwest::Client;
use shellexpand::tilde;

pub struct InitOptions {
    pub force: bool,
    pub arch: Option<String>,
    pub yes: bool,
}

// ---------------------------------------------------------------------------
// Stage report
// ---------------------------------------------------------------------------

struct StageReport {
    entries: Vec<(&'static str, StageStatus)>,
}

impl StageReport {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Print the stage header and optional first detail line.
    fn begin(&self, name: &'static str, description: Option<&str>) {
        println!("{} {}", "●".cyan().bold(), name.bold());
        if let Some(description) = description {
            println!("{}  {}", " ⎿".cyan().bold(), description.italic().dimmed());
        }
    }

    async fn run<F, Fut>(&mut self, name: &'static str, description: Option<&str>, f: F)
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<StageStatus>>,
    {
        self.begin(name, description);
        let status = match f().await {
            Ok(s) => s,
            Err(e) => StageStatus::Failed(e),
        };
        self.entries.push((name, status));
    }

    /// Record a pre-computed status without printing a header.  Pair with
    /// [`Self::begin`] for stages whose header must appear *before* the work runs
    /// (e.g. the binary download / install split, where ownership constraints
    /// make the closure pattern awkward).
    fn record(&mut self, name: &'static str, status: StageStatus) {
        self.entries.push((name, status));
    }

    fn print(&self) {
        println!("{} {}", "mihoro:".cyan().bold(), "init summary".bold());
        for (name, status) in &self.entries {
            match status {
                StageStatus::Installed => {
                    println!("  {} {}", "✓".green().bold(), name);
                }
                StageStatus::Skipped(reason) => {
                    println!("  {} {} ({})", "↷".dimmed(), name.dimmed(), reason.dimmed());
                }
                StageStatus::Failed(err) => {
                    println!("  {} {}: {:#}", "✗".red().bold(), name.red(), err);
                }
            }
        }
    }

    fn has_failures(&self) -> bool {
        self.entries
            .iter()
            .any(|(_, s)| matches!(s, StageStatus::Failed(_)))
    }

    fn stage_failed(&self, name: &'static str) -> bool {
        self.entries
            .iter()
            .any(|(n, s)| *n == name && matches!(s, StageStatus::Failed(_)))
    }
}

// ---------------------------------------------------------------------------
// Dashboard URL helper
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct DashboardUrl {
    label: &'static str,
    url: String,
}

#[derive(Debug, PartialEq, Eq)]
struct ControllerAddress<'a> {
    host: &'a str,
    port: &'a str,
}

fn dashboard_urls(config: &Config) -> Option<Vec<DashboardUrl>> {
    let interfaces = list_afinet_netifas().unwrap_or_default();
    dashboard_urls_from_interfaces(config, &interfaces)
}

fn dashboard_urls_from_interfaces(
    config: &Config,
    interfaces: &[(String, IpAddr)],
) -> Option<Vec<DashboardUrl>> {
    let external_ui = config.mihomo_config.external_ui.as_deref()?.trim();
    if external_ui.is_empty() {
        return None;
    }

    let controller =
        parse_controller_address(config.mihomo_config.external_controller.as_deref()?)?;
    let mut urls = Vec::new();

    if is_wildcard_host(controller.host) {
        urls.push(DashboardUrl {
            label: "Local",
            url: dashboard_url("127.0.0.1", controller.port),
        });
        for ip in dashboard_interface_ips(interfaces) {
            urls.push(DashboardUrl {
                label: "External",
                url: dashboard_url(&format_ip_host(ip), controller.port),
            });
        }
    } else {
        let label = if is_loopback_host(controller.host) {
            "Local"
        } else {
            "Dashboard"
        };
        urls.push(DashboardUrl {
            label,
            url: dashboard_url(controller.host, controller.port),
        });
    }

    dedup_dashboard_urls(urls)
}

fn parse_controller_address(controller: &str) -> Option<ControllerAddress<'_>> {
    let controller = controller
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/');

    if let Some(rest) = controller.strip_prefix('[') {
        let (host, rest) = rest.split_once(']')?;
        let port = rest.strip_prefix(':')?;
        return Some(ControllerAddress {
            host: controller.get(..host.len() + 2)?,
            port,
        });
    }

    let (host, port) = controller.rsplit_once(':')?;
    Some(ControllerAddress {
        host: host.trim(),
        port: port.trim(),
    })
}

fn dashboard_interface_ips(interfaces: &[(String, IpAddr)]) -> Vec<IpAddr> {
    interfaces
        .iter()
        .filter(|(name, ip)| is_dashboard_interface(name) && is_external_dashboard_ip(ip))
        .map(|(_, ip)| *ip)
        .collect()
}

fn is_dashboard_interface(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    !matches!(
        name.as_str(),
        "lo" | "docker0" | "podman0" | "cni0" | "virbr0" | "lxdbr0"
    ) && !name.starts_with("br-")
        && !name.starts_with("veth")
        && !name.starts_with("flannel")
        && !name.starts_with("kube")
}

fn is_external_dashboard_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !ip.is_loopback()
                && !ip.is_unspecified()
                && !ip.is_link_local()
                && !ip.is_broadcast()
                && !ip.is_multicast()
        }
        IpAddr::V6(_) => false,
    }
}

fn is_wildcard_host(host: &str) -> bool {
    matches!(host.trim(), "" | "*" | "0.0.0.0" | "::" | "[::]")
}

fn is_loopback_host(host: &str) -> bool {
    let host = host.trim();
    host.eq_ignore_ascii_case("localhost")
        || matches!(host, "127.0.0.1" | "::1" | "[::1]")
        || host
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}

fn format_ip_host(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => format!("[{ip}]"),
    }
}

fn dashboard_url(host: &str, port: &str) -> String {
    format!("http://{host}:{port}/ui")
}

fn dedup_dashboard_urls(urls: Vec<DashboardUrl>) -> Option<Vec<DashboardUrl>> {
    let mut seen = HashSet::new();
    let urls = urls
        .into_iter()
        .filter(|entry| seen.insert(entry.url.clone()))
        .collect::<Vec<_>>();

    if urls.is_empty() {
        None
    } else {
        Some(urls)
    }
}

// ---------------------------------------------------------------------------
// Interactive bootstrap
// ---------------------------------------------------------------------------

fn prompt_subscription_url() -> Result<String> {
    let url: String = Input::new()
        .with_prompt("Remote subscription URL")
        .validate_with(|s: &String| {
            if s.trim().is_empty() {
                Err("URL cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact_text()?;
    Ok(url.trim().to_string())
}

fn bootstrap_config(config_path: &str, yes: bool) -> Result<Config> {
    let just_created = write_default_if_missing(config_path)?;

    // After write_default_if_missing the file always exists.
    let mut config =
        load_config(config_path)?.expect("config file must exist after write_default_if_missing");

    if config.remote_config_url.is_empty() {
        if yes {
            bail!(
                "`remote_config_url` is not set - edit `{}` or run `mihoro init` interactively",
                config_path
            );
        }

        if just_created {
            println!(
                "{} Created default config at {}",
                "mihoro:".cyan(),
                config_path.underline().yellow()
            );
        }
        println!("{} Enter your remote subscription URL:", "mihoro:".yellow());
        config.remote_config_url = prompt_subscription_url()?;
        config.write(Path::new(config_path))?;
        println!("{} Saved", "mihoro:".green());
    }

    Ok(config)
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub async fn run(config_path: &str, client: &Client, opts: InitOptions) -> Result<()> {
    let config_path = tilde(config_path);
    let config_path = config_path.as_ref();
    let config = bootstrap_config(config_path, opts.yes)?;
    validate_config(&config)?;

    let mihoro = Mihoro::from_config(config.clone());

    println!("{} initializing", "mihoro:".cyan().bold());

    let force = opts.force;
    let arch = opts.arch.as_deref();
    let mut report = StageReport::new();

    // --- download phase -------------------------------------------------------
    //
    // Binary is downloaded first so that the running mihomo proxy is still alive
    // for subsequent requests (config / geodata / UI may all go through
    // https_proxy=http://127.0.0.1:<port>).  The actual service stop + binary
    // swap is deferred to the "install binary" stage after all downloads finish.

    report.begin("mihomo binary", Some("downloading mihomo binary"));
    let binary_temp = match mihoro.prepare_binary(client, force, arch).await {
        Ok(BinaryPlan::Install(temp)) => {
            report.record("mihomo binary", StageStatus::Installed);
            Some(temp)
        }
        Ok(BinaryPlan::Skip(reason)) => {
            report.record("mihomo binary", StageStatus::Skipped(reason));
            None
        }
        Err(e) => {
            report.record("mihomo binary", StageStatus::Failed(e));
            None
        }
    };

    report
        .run(
            "remote config",
            Some("downloading and merging remote config"),
            || mihoro.ensure_remote_config(client, force),
        )
        .await;
    report
        .run("geodata", Some("downloading geodata"), || {
            mihoro.ensure_geodata(client, force)
        })
        .await;
    report
        .run(
            "web dashboard",
            Some("downloading dashboard assets"),
            || mihoro.ensure_ui(client, force),
        )
        .await;

    // --- install phase --------------------------------------------------------
    //
    // All network calls are done.  Now it is safe to stop the running service
    // (which also tears down the proxy) and swap in the new binary.
    //
    // If remote config stage failed we must not proceed: installing a new
    // binary or restarting mihomo.service on top of a missing / corrupt config.yaml
    // would break an environment that may have been working before.

    if report.stage_failed("remote config") {
        let skip = || StageStatus::Skipped("skipped: remote config stage failed".to_string());
        report.record("install binary", skip());
        report.record("systemd service", skip());
        report.record("service start", skip());
    } else {
        report.begin("install binary", Some("installing mihomo binary"));
        let install_status = match binary_temp {
            None => StageStatus::Skipped("nothing to install".to_string()),
            Some(temp) => match mihoro.install_binary(temp).await {
                Ok(s) => s,
                Err(e) => StageStatus::Failed(e),
            },
        };
        report.record("install binary", install_status);

        report
            .run(
                "systemd service",
                Some("writing systemd service"),
                || async { mihoro.ensure_service().await },
            )
            .await;
        report
            .run(
                "service start",
                Some("starting and enabling mihomo.service"),
                || async { mihoro.ensure_service_running().await },
            )
            .await;
    }

    report.print();

    // Print dashboard URL if UI is configured
    if config.ui.is_some() {
        if let Some(urls) = dashboard_urls(&config) {
            println!();
            if urls.len() == 1 && urls[0].label == "Dashboard" {
                println!(
                    "  {}: {}",
                    "Dashboard".bold(),
                    urls[0].url.as_str().underline().cyan()
                );
            } else {
                println!("  {}:", "Dashboard".bold());
                for entry in &urls {
                    println!(
                        "    {}: {}",
                        entry.label.bold(),
                        entry.url.as_str().underline().cyan()
                    );
                }
            }
            let ui_name = config
                .ui
                .as_ref()
                .map(|u| u.as_config_value())
                .unwrap_or("ui");
            println!(
                "  Using {} - change via the {} field in {}",
                ui_name.bold(),
                "`ui`".bold(),
                "mihoro.toml".underline()
            );
            if config.mihomo_config.secret.is_some() {
                println!("  Authentication required (secret is set in mihoro.toml)");
            } else {
                println!(
                    "  Set {} in {} to require a password",
                    "`mihomo_config.secret`".bold(),
                    "mihoro.toml".underline()
                );
            }
        }
    }

    if report.has_failures() {
        bail!("one or more stages failed - see summary above");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::net::{Ipv4Addr, Ipv6Addr};

    fn config_with_controller(controller: &str) -> Config {
        let mut config = Config::default();
        config.mihomo_config.external_controller = Some(controller.to_string());
        config.mihomo_config.external_ui = Some("ui".to_string());
        config
    }

    #[test]
    fn dashboard_urls_include_real_interfaces_for_wildcard_controller() {
        let config = config_with_controller("0.0.0.0:19090");
        let interfaces = vec![
            ("lo".to_string(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            (
                "eth0".to_string(),
                IpAddr::V4(Ipv4Addr::new(10, 108, 25, 191)),
            ),
            (
                "tailscale0".to_string(),
                IpAddr::V4(Ipv4Addr::new(100, 120, 119, 21)),
            ),
            (
                "docker0".to_string(),
                IpAddr::V4(Ipv4Addr::new(172, 17, 0, 1)),
            ),
        ];

        let urls = dashboard_urls_from_interfaces(&config, &interfaces).unwrap();

        assert_eq!(
            urls,
            vec![
                DashboardUrl {
                    label: "Local",
                    url: "http://127.0.0.1:19090/ui".to_string(),
                },
                DashboardUrl {
                    label: "External",
                    url: "http://10.108.25.191:19090/ui".to_string(),
                },
                DashboardUrl {
                    label: "External",
                    url: "http://100.120.119.21:19090/ui".to_string(),
                },
            ]
        );
    }

    #[test]
    fn dashboard_urls_keep_loopback_controller_local_only() {
        let config = config_with_controller("127.0.0.1:9090");
        let interfaces = vec![(
            "eth0".to_string(),
            IpAddr::V4(Ipv4Addr::new(10, 108, 25, 191)),
        )];

        let urls = dashboard_urls_from_interfaces(&config, &interfaces).unwrap();

        assert_eq!(
            urls,
            vec![DashboardUrl {
                label: "Local",
                url: "http://127.0.0.1:9090/ui".to_string(),
            }]
        );
    }

    #[test]
    fn dashboard_urls_keep_explicit_controller_host() {
        let config = config_with_controller("10.108.25.191:19090");
        let interfaces = vec![(
            "tailscale0".to_string(),
            IpAddr::V4(Ipv4Addr::new(100, 120, 119, 21)),
        )];

        let urls = dashboard_urls_from_interfaces(&config, &interfaces).unwrap();

        assert_eq!(
            urls,
            vec![DashboardUrl {
                label: "Dashboard",
                url: "http://10.108.25.191:19090/ui".to_string(),
            }]
        );
    }

    #[test]
    fn dashboard_urls_skip_ipv6_interfaces_for_now() {
        let config = config_with_controller("[::]:19090");
        let interfaces = vec![(
            "wlan0".to_string(),
            IpAddr::V6(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1)),
        )];

        let urls = dashboard_urls_from_interfaces(&config, &interfaces).unwrap();

        assert_eq!(
            urls,
            vec![DashboardUrl {
                label: "Local",
                url: "http://127.0.0.1:19090/ui".to_string(),
            }]
        );
    }

    #[test]
    fn dashboard_urls_are_hidden_without_external_ui() {
        let mut config = config_with_controller("0.0.0.0:19090");
        config.mihomo_config.external_ui = None;

        assert_eq!(dashboard_urls_from_interfaces(&config, &[]), None);
    }
}
