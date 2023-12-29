mod cmd;
mod config;
mod systemctl;
mod utils;

use std::fs;
use std::io;
use std::os::unix::prelude::PermissionsExt;
use std::process::Command;

use anyhow::bail;
use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;
use clap_complete::shells::Bash;
use clap_complete::shells::Fish;
use clap_complete::shells::Zsh;
use colored::Colorize;
use local_ip_address::local_ip;
use reqwest::Client;
use shellexpand::tilde;

use cmd::Args;
use cmd::ClapShell;
use cmd::Commands;
use cmd::ProxyCommands;
use config::apply_mihomo_override;
use config::parse_config;
use config::Config;
use systemctl::Systemctl;
use utils::create_mihomo_service;
use utils::delete_file;
use utils::download_file;
use utils::extract_gzip;
use utils::proxy_export_cmd;
use utils::proxy_unset_cmd;

#[tokio::main]
async fn main() {
    if let Err(err) = cli().await {
        eprintln!("{} {}", "error:".bright_red().bold(), err);
        std::process::exit(1);
    }
}

async fn cli() -> Result<()> {
    let args = Args::parse();
    let prefix = "mihoro:";
    let config_path = tilde(&args.mihoro_config).to_string();

    // Initial setup and parse config file
    let config: Config = parse_config(&config_path, prefix)?;

    // Expand mihomo related paths and target directories
    let mihomo_gzipped_path = "mihomo.tar.gz";

    let mihomo_target_binary_path = tilde(&config.mihomo_binary_path).to_string();
    let mihomo_target_config_root = tilde(&config.mihomo_config_root).to_string();
    let mihomo_target_config_path =
        tilde(&format!("{}/config.yaml", config.mihomo_config_root)).to_string();
    let mihomo_target_mmdb_path =
        tilde(&format!("{}/Country.mmdb", config.mihomo_config_root)).to_string();
    let mihomo_target_service_path =
        tilde(&format!("{}/mihomo.service", config.user_systemd_root)).to_string();

    // Reuse http client for file download
    let client = Client::new();

    match &args.command {
        Some(Commands::Setup) => {
            println!(
                "{} Setting up mihomo's binary, config, and systemd service...",
                prefix.cyan()
            );

            // Attempt to download and setup mihomo binary if needed
            if fs::metadata(&mihomo_target_binary_path).is_ok() {
                // If mihomo binary already exists at `mihomo_target_binary_path`, then skip setup
                println!(
                    "{} Assuming mihomo binary already installed at {}, skipping setup",
                    prefix.yellow(),
                    mihomo_target_binary_path.underline().green()
                );
            } else {
                // Abort if `remote_mihomo_binary_url` is not defined in config
                if config.remote_mihomo_binary_url.is_empty() {
                    bail!("`remote_mihomo_binary_url` undefined");
                }

                // Download mihomo binary and set permission to executable
                download_file(
                    &client,
                    &config.remote_mihomo_binary_url,
                    mihomo_gzipped_path,
                )
                .await?;
                extract_gzip(mihomo_gzipped_path, &mihomo_target_binary_path, prefix)?;

                let executable = fs::Permissions::from_mode(0o755);
                fs::set_permissions(&mihomo_target_binary_path, executable)?;
            }

            // Download remote mihomo config and apply override
            download_file(
                &client,
                &config.remote_config_url,
                &mihomo_target_config_path,
            )
            .await?;
            apply_mihomo_override(&mihomo_target_config_path, &config.mihomo_config)?;

            // Download remote Country.mmdb
            download_file(&client, &config.remote_mmdb_url, &mihomo_target_mmdb_path).await?;

            // Create mihomo.service systemd file
            create_mihomo_service(
                &mihomo_target_binary_path,
                &mihomo_target_config_root,
                &mihomo_target_service_path,
                prefix,
            )?;

            Systemctl::new().enable("mihomo.service").execute()?;
            Systemctl::new().start("mihomo.service").execute()?;
        }
        Some(Commands::Update) => {
            // Download remote mihomo config and apply override
            download_file(
                &client,
                &config.remote_config_url,
                &mihomo_target_config_path,
            )
            .await?;
            apply_mihomo_override(&mihomo_target_config_path, &config.mihomo_config)?;
            println!("{} Updated and applied config overrides", prefix.yellow());

            // Download remote Country.mmdb
            download_file(&client, &config.remote_mmdb_url, &mihomo_target_mmdb_path).await?;

            // Restart mihomo systemd service
            println!("{} Restart mihomo.service", prefix.green());
            Systemctl::new().restart("mihomo.service").execute()?;
        }
        Some(Commands::Apply) => {
            // Apply mihomo config override
            apply_mihomo_override(&mihomo_target_config_path, &config.mihomo_config).and_then(
                |_| {
                    println!("{} Applied mihomo config overrides", prefix.green().bold());
                    Ok(())
                },
            )?;

            // Restart mihomo systemd service
            Systemctl::new()
                .restart("mihomo.service")
                .execute()
                .and_then(|_| {
                    println!("{} Restarted mihomo.service", prefix.green().bold());
                    Ok(())
                })?;
        }
        Some(Commands::Start) => {
            Systemctl::new()
                .start("mihomo.service")
                .execute()
                .and_then(|_| {
                    println!("{} Started mihomo.service", prefix.green());
                    Ok(())
                })?;
        }
        Some(Commands::Status) => {
            Systemctl::new().status("mihomo.service").execute()?;
        }
        Some(Commands::Stop) => {
            Systemctl::new()
                .stop("mihomo.service")
                .execute()
                .and_then(|_| {
                    println!("{} Stopped mihomo.service", prefix.green());
                    Ok(())
                })?;
        }
        Some(Commands::Restart) => {
            Systemctl::new()
                .restart("mihomo.service")
                .execute()
                .and_then(|_| {
                    println!("{} Restarted mihomo.service", prefix.green());
                    Ok(())
                })?;
        }
        Some(Commands::Log) => {
            Command::new("journalctl")
                .arg("--user")
                .arg("-u")
                .arg("mihomo.service")
                .arg("-n")
                .arg("10")
                .arg("-f")
                .spawn()
                .expect("failed to execute process")
                .wait()?;
        }
        Some(Commands::Proxy { proxy }) => match proxy {
            Some(ProxyCommands::Export) => {
                println!(
                    "{}",
                    proxy_export_cmd(
                        "127.0.0.1",
                        &config.mihomo_config.port,
                        &config.mihomo_config.socks_port
                    )
                )
            }
            Some(ProxyCommands::ExportLan) => {
                if !config.mihomo_config.allow_lan.unwrap_or(false) {
                    bail!(
                        "`allow_lan` is false, edit {} and `mihoro apply` to enable",
                        config_path.underline().yellow()
                    );
                }

                let hostname = local_ip();
                if let Ok(hostname) = hostname {
                    println!(
                        "{}",
                        proxy_export_cmd(
                            &hostname.to_string(),
                            &config.mihomo_config.port,
                            &config.mihomo_config.socks_port
                        )
                    )
                } else {
                    println!("{} Failed to get local IP address", prefix.red());
                }
            }
            Some(ProxyCommands::Unset) => {
                println!("{}", proxy_unset_cmd())
            }
            _ => {
                println!("{} No proxy command, --help for ussage", prefix.red());
            }
        },
        Some(Commands::Uninstall) => {
            Systemctl::new().stop("mihomo.service").execute()?;
            Systemctl::new().disable("mihomo.service").execute()?;

            // delete_file(&mihomo_target_binary_path, prefix);
            delete_file(&mihomo_target_service_path, prefix)?;
            delete_file(&mihomo_target_config_path, prefix)?;

            println!("{} Disable and reload systemd services", prefix.green());
            Systemctl::new().daemon_reload().execute()?;
            Systemctl::new().reset_failed().execute()?;
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
            _ => println!("{} No shell specified, --help for usage", prefix.red()),
        },
        None => println!("{} No command specified, --help for usage", prefix.yellow()),
    }
    Ok(())
}
