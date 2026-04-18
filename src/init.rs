use crate::config::{load_config, validate_config, write_default_if_missing, Config};
use crate::mihoro::{BinaryPlan, Mihoro, StageStatus};

use std::{future::Future, path::Path};

use anyhow::{bail, Result};
use colored::Colorize;
use dialoguer::Input;
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

fn dashboard_url(config: &Config) -> Option<String> {
    let controller = config.mihomo_config.external_controller.as_deref()?;
    let (host, port) = controller.rsplit_once(':')?;
    let host = match host {
        "0.0.0.0" | "[::]" | "" => "127.0.0.1",
        h => h,
    };
    Some(format!("http://{host}:{port}/ui/"))
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

    report.begin("mihomo binary", Some("downloading the mihomo binary"));
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
        if let Some(url) = dashboard_url(&config) {
            println!();
            println!("  {}: {}", "Dashboard".bold(), url.underline().cyan());
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
