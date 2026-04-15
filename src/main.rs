mod cmd;
mod config;
mod cron;
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
use std::{io, process::Command};

use cmd::{Args, ClapShell, Commands};
use mihoro::Mihoro;
use systemctl::Systemctl;

#[tokio::main]
async fn main() {
    if let Err(err) = cli().await {
        eprintln!("{} {}", "error:".bright_red().bold(), err);
        std::process::exit(1);
    }
}

async fn cli() -> Result<()> {
    let args = Args::parse();
    let client = Client::new();
    let mihoro = Mihoro::new(&args.mihoro_config)?;

    match &args.command {
        Some(Commands::Setup { overwrite, arch }) => {
            mihoro.setup(client, *overwrite, arch.as_deref()).await?
        }
        Some(Commands::Update {
            config,
            core,
            geodata,
            all,
            arch,
            ui,
        }) => {
            if *all {
                // Update config (without restarting yet)
                println!(
                    "{} Updating config...",
                    mihoro.prefix.magenta().bold().italic()
                );
                if let Err(e) = mihoro.update_config(&client, false).await {
                    eprintln!("{} Failed to update config: {}", mihoro.prefix.yellow(), e);
                }
                // Update geodata
                println!(
                    "{} Updating geodata...",
                    mihoro.prefix.magenta().bold().italic()
                );
                if let Err(e) = mihoro.update_geodata(&client).await {
                    eprintln!("{} Failed to update geodata: {}", mihoro.prefix.yellow(), e);
                }
                // Update core (without restarting yet)
                println!(
                    "{} Updating core...",
                    mihoro.prefix.magenta().bold().italic()
                );
                if let Err(e) = mihoro.update_core(&client, arch.as_deref(), false).await {
                    eprintln!("{} Failed to update core: {}", mihoro.prefix.yellow(), e);
                }
                println!("{} Updating UI...", mihoro.prefix.magenta().bold().italic());
                if let Err(e) = mihoro.update_ui(&client).await {
                    eprintln!("{} Failed to update UI: {}", mihoro.prefix.yellow(), e);
                }
                // Restart service once at the end
                println!(
                    "{} Restarting mihomo.service...",
                    mihoro.prefix.green().bold().italic()
                );
                Systemctl::new().restart("mihomo.service").execute()?;
            } else if *core {
                mihoro.update_core(&client, arch.as_deref(), true).await?;
            } else if *ui {
                mihoro.update_ui(&client).await?;
            } else if *geodata {
                mihoro.update_geodata(&client).await?;
            } else if *config || (!*core && !*geodata && !*ui) {
                // Explicit --config or default (no flags)
                mihoro.update_config(&client, true).await?;
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
