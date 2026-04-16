mod cmd;
mod config;
mod cron;
mod init;
mod mihoro;
mod proxy;
mod resolve_mihomo_bin;
mod systemctl;
mod ui;
#[cfg(feature = "self_update")]
mod upgrade;
mod utils;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::{
    generate,
    shells::{Bash, Fish, Zsh},
};
use colored::Colorize;
use reqwest::Client;
use std::{future::Future, io, process::Command, time::Duration};

use cmd::{Args, ClapShell, Commands};
use mihoro::{Mihoro, StageStatus};
use systemctl::Systemctl;

struct StageReport {
    entries: Vec<(&'static str, StageStatus)>,
}

impl StageReport {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

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
            Ok(status) => status,
            Err(err) => StageStatus::Failed(err),
        };
        self.entries.push((name, status));
    }

    fn record(&mut self, name: &'static str, status: StageStatus) {
        self.entries.push((name, status));
    }

    fn print(&self, label: &str) {
        println!("{} {}", "mihoro:".cyan().bold(), label.bold());
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
            .any(|(_, status)| matches!(status, StageStatus::Failed(_)))
    }
}

#[tokio::main]
async fn main() {
    if let Err(err) = cli().await {
        eprintln!("{} {}", "error:".bright_red().bold(), err);
        std::process::exit(1);
    }
}

async fn cli() -> Result<()> {
    let args = Args::parse();
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .build()?;

    // Handle Init and Setup before constructing Mihoro, which requires a valid config.
    match &args.command {
        Some(Commands::Init { force, arch, yes }) => {
            return init::run(
                &args.mihoro_config,
                &client,
                init::InitOptions {
                    force: *force,
                    arch: arch.clone(),
                    yes: *yes,
                },
            )
            .await;
        }
        Some(Commands::Setup { overwrite, arch }) => {
            eprintln!(
                "{} `setup` is deprecated - use `mihoro init` instead",
                "warning:".yellow()
            );
            return init::run(
                &args.mihoro_config,
                &client,
                init::InitOptions {
                    force: *overwrite,
                    arch: arch.clone(),
                    yes: true,
                },
            )
            .await;
        }
        _ => {}
    }

    let mihoro = Mihoro::new(&args.mihoro_config)?;

    match &args.command {
        Some(Commands::Init { .. }) | Some(Commands::Setup { .. }) => unreachable!(),
        Some(Commands::Update {
            config,
            core,
            geodata,
            all,
            arch,
            ui,
        }) => {
            println!("{} update", "mihoro:".cyan().bold());
            let mut report = StageReport::new();

            if *all {
                report
                    .run(
                        "config",
                        Some("Download the remote config and apply local overrides"),
                        || mihoro.update_config(&client),
                    )
                    .await;
                report
                    .run(
                        "geodata",
                        Some("Refresh geoip / geosite data used by mihomo"),
                        || mihoro.update_geodata(&client),
                    )
                    .await;
                report
                    .run(
                        "ui",
                        Some("Download and install the configured web dashboard"),
                        || mihoro.update_ui(&client),
                    )
                    .await;
                report
                    .run(
                        "core",
                        Some("Download and install the latest mihomo core binary"),
                        || mihoro.update_core(&client, arch.as_deref()),
                    )
                    .await;
                if !report.has_failures() {
                    report
                        .run("service restart", None, || mihoro.restart_service())
                        .await;
                } else {
                    report.record(
                        "service restart",
                        StageStatus::Skipped("skipped due to earlier failures".to_string()),
                    );
                }
            } else if *core {
                report
                    .run(
                        "core",
                        Some("Download and install the latest mihomo core binary"),
                        || mihoro.update_core(&client, arch.as_deref()),
                    )
                    .await;
                if !report.has_failures() {
                    report
                        .run("service restart", None, || mihoro.restart_service())
                        .await;
                } else {
                    report.record(
                        "service restart",
                        StageStatus::Skipped("skipped due to earlier failures".to_string()),
                    );
                }
            } else if *ui {
                report
                    .run(
                        "ui",
                        Some("Download and install the configured web dashboard"),
                        || mihoro.update_ui(&client),
                    )
                    .await;
            } else if *geodata {
                report
                    .run(
                        "geodata",
                        Some("Refresh geoip / geosite data used by mihomo"),
                        || mihoro.update_geodata(&client),
                    )
                    .await;
            } else if *config || (!*core && !*geodata && !*ui) {
                report
                    .run(
                        "config",
                        Some("Download the remote config and apply local overrides"),
                        || mihoro.update_config(&client),
                    )
                    .await;
                if !report.has_failures() {
                    report
                        .run("service restart", None, || mihoro.restart_service())
                        .await;
                } else {
                    report.record(
                        "service restart",
                        StageStatus::Skipped("skipped due to earlier failures".to_string()),
                    );
                }
            }

            report.print("update summary");
            if report.has_failures() {
                anyhow::bail!("one or more update stages failed - see summary above");
            }
        }
        Some(Commands::Apply) => mihoro.apply().await?,
        Some(Commands::Uninstall) => mihoro.uninstall()?,
        Some(Commands::Proxy { proxy }) => mihoro.proxy_commands(proxy)?,

        Some(Commands::Start) => Systemctl::new()
            .start("mihomo.service")
            .execute()
            .map(|_| {
                println!("{} Started mihomo.service", mihoro.prefix.green());
            })?,

        Some(Commands::Status) => {
            Systemctl::new().status("mihomo.service").execute()?;
        }

        Some(Commands::Stop) => Systemctl::new().stop("mihomo.service").execute().map(|_| {
            println!("{} Stopped mihomo.service", mihoro.prefix.green());
        })?,

        Some(Commands::Restart) => {
            Systemctl::new()
                .restart("mihomo.service")
                .execute()
                .map(|_| {
                    println!("{} Restarted mihomo.service", mihoro.prefix.green());
                })?
        }

        Some(Commands::Log) => {
            Command::new("journalctl")
                .arg("--user")
                .arg("-xeu")
                .arg("mihomo.service")
                .arg("-n")
                .arg("10")
                .arg("-f")
                .spawn()
                .expect("failed to execute process")
                .wait()?;
        }

        Some(Commands::Completions { shell }) => match shell {
            Some(ClapShell::Bash) => {
                generate(Bash, &mut Args::command(), "mihoro", &mut io::stdout())
            }
            Some(ClapShell::Zsh) => {
                generate(Zsh, &mut Args::command(), "mihoro", &mut io::stdout())
            }
            Some(ClapShell::Fish) => {
                generate(Fish, &mut Args::command(), "mihoro", &mut io::stdout())
            }
            _ => (),
        },

        Some(Commands::Cron { cron }) => mihoro.cron_commands(cron)?,

        #[cfg(feature = "self_update")]
        Some(Commands::Upgrade { yes, check, target }) => {
            if *check {
                match upgrade::check_for_update().await? {
                    Some(version) => {
                        println!(
                            "{} New version available: {}",
                            mihoro.prefix.yellow(),
                            version.bold().green()
                        );
                        println!(
                            "{} Run {} to update",
                            "->".dimmed(),
                            "mihoro upgrade".bold().underline()
                        );
                    }
                    None => {
                        println!(
                            "{} You're running the latest version",
                            mihoro.prefix.green()
                        );
                    }
                }
            } else {
                upgrade::run_upgrade(*yes, target.clone()).await?;
            }
        }

        #[cfg(not(feature = "self_update"))]
        Some(Commands::Upgrade { .. }) => {
            anyhow::bail!(
                "mihoro was built without self_update support, please use your package manager to upgrade"
            );
        }

        None => (),
    }
    Ok(())
}
