mod cmd;
mod config;
mod systemctl;
mod utils;

use std::fs;
use std::io;
use std::os::unix::prelude::PermissionsExt;
use std::process::Command;

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
use config::apply_clash_override;
use config::parse_config;
use config::Config;
use systemctl::Systemctl;
use utils::create_clash_service;
use utils::delete_file;
use utils::download_file;
use utils::extract_gzip;
use utils::proxy_export_cmd;
use utils::proxy_unset_cmd;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let prefix = "clashrup:";
    let config_path = tilde(&args.clashrup_config).to_string();

    // Initial setup and parse config file
    let config: Config = match parse_config(&config_path, prefix) {
        Ok(config) => config,
        Err(_) => return,
    };

    // Expand clash related paths and target directories
    let clash_gzipped_path = "clash.tar.gz";

    let clash_target_binary_path = tilde(&config.clash_binary_path).to_string();
    let clash_target_config_root = tilde(&config.clash_config_root).to_string();
    let clash_target_config_path =
        tilde(&format!("{}/config.yaml", config.clash_config_root)).to_string();
    let clash_target_mmdb_path =
        tilde(&format!("{}/Country.mmdb", config.clash_config_root)).to_string();
    let clash_target_service_path =
        tilde(&format!("{}/clash.service", config.user_systemd_root)).to_string();

    // Reuse http client for file download
    let client = Client::new();

    match &args.command {
        Some(Commands::Setup) => {
            println!(
                "{} Setting up clash's binary, config, and systemd service...",
                prefix.cyan()
            );

            // Attempt to download and setup clash binary if needed
            if fs::metadata(&clash_target_binary_path).is_ok() {
                // If clash binary already exists at `clash_target_binary_path`, then skip setup
                println!(
                    "{} Assuming clash binary already installed at {}, skipping setup",
                    prefix.yellow(),
                    clash_target_binary_path.underline().green()
                );
            } else {
                // Abort if `remote_clash_binary_url` is not defined in config
                if config.remote_clash_binary_url.is_empty() {
                    println!("{} `remote_clash_binary_url` undefined", "error:".red());
                    return;
                }

                // Download clash binary and set permission to executable
                download_file(&client, &config.remote_clash_binary_url, clash_gzipped_path)
                    .await
                    .unwrap();
                extract_gzip(clash_gzipped_path, &clash_target_binary_path, prefix);

                let executable = fs::Permissions::from_mode(0o755);
                fs::set_permissions(&clash_target_binary_path, executable).unwrap();
            }

            // Download remote clash config and apply override
            download_file(
                &client,
                &config.remote_config_url,
                &clash_target_config_path,
            )
            .await
            .unwrap();
            apply_clash_override(&clash_target_config_path, &config.clash_config);

            // Download remote Country.mmdb
            download_file(&client, &config.remote_mmdb_url, &clash_target_mmdb_path)
                .await
                .unwrap();

            // Create clash.service systemd file
            create_clash_service(
                &clash_target_binary_path,
                &clash_target_config_root,
                &clash_target_service_path,
                prefix,
            );

            Systemctl::new().enable("clash.service").execute();
            Systemctl::new().start("clash.service").execute();
        }
        Some(Commands::Update) => {
            // Download remote clash config and apply override
            download_file(
                &client,
                &config.remote_config_url,
                &clash_target_config_path,
            )
            .await
            .unwrap();
            apply_clash_override(&clash_target_config_path, &config.clash_config);
            println!("{} Updated and applied config overrides", prefix.yellow());

            // Download remote Country.mmdb
            download_file(&client, &config.remote_mmdb_url, &clash_target_mmdb_path)
                .await
                .unwrap();

            // Restart clash systemd service
            println!("{} Restart clash.service", prefix.green());
            Systemctl::new().restart("clash.service").execute();
        }
        Some(Commands::Apply) => {
            // Apply clash config override
            apply_clash_override(&clash_target_config_path, &config.clash_config);
            println!("{} Applied clash config overrides", prefix.yellow());

            // Restart clash systemd service
            println!("{} Restart clash.service", prefix.green());
            Systemctl::new().restart("clash.service").execute();
        }
        Some(Commands::Start) => {
            Systemctl::new().start("clash.service").execute();
            println!("{} Started clash.service", prefix.green());
        }
        Some(Commands::Status) => {
            Systemctl::new().status("clash.service").execute();
        }
        Some(Commands::Stop) => {
            Systemctl::new().stop("clash.service").execute();
            println!("{} Stopped clash.service", prefix.green());
        }
        Some(Commands::Restart) => {
            Systemctl::new().restart("clash.service").execute();
            println!("{} Restarted clash.service", prefix.green());
        }
        Some(Commands::Log) => {
            Command::new("journalctl")
                .arg("--user")
                .arg("-u")
                .arg("clash.service")
                .arg("-n")
                .arg("10")
                .arg("-f")
                .spawn()
                .expect("failed to execute process")
                .wait()
                .unwrap();
        }
        Some(Commands::Proxy { proxy }) => match proxy {
            Some(ProxyCommands::Export) => {
                println!(
                    "{}",
                    proxy_export_cmd(
                        "127.0.0.1",
                        &config.clash_config.port,
                        &config.clash_config.socks_port
                    )
                )
            }
            Some(ProxyCommands::ExportLan) => {
                if !config.clash_config.allow_lan.unwrap_or(false) {
                    println!(
                        "{} `allow_lan` is false, edit {} and `clashrup apply` to enable",
                        prefix.red(),
                        config_path.underline().yellow()
                    );
                    return;
                }

                let hostname = local_ip();
                if let Ok(hostname) = hostname {
                    println!(
                        "{}",
                        proxy_export_cmd(
                            &hostname.to_string(),
                            &config.clash_config.port,
                            &config.clash_config.socks_port
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
            Systemctl::new().stop("clash.service").execute();
            Systemctl::new().disable("clash.service").execute();

            // delete_file(&clash_target_binary_path, prefix);
            delete_file(&clash_target_service_path, prefix);
            delete_file(&clash_target_config_path, prefix);

            println!("{} Disable and reload systemd services", prefix.green());
            Systemctl::new().daemon_reload().execute();
            Systemctl::new().reset_failed().execute();
        }
        Some(Commands::Completions { shell }) => match shell {
            Some(ClapShell::Bash) => {
                generate(Bash, &mut Args::command(), "clashrup", &mut io::stdout())
            }
            Some(ClapShell::Zsh) => {
                generate(Zsh, &mut Args::command(), "clashrup", &mut io::stdout())
            }
            Some(ClapShell::Fish) => {
                generate(Fish, &mut Args::command(), "clashrup", &mut io::stdout())
            }
            _ => {
                println!("{} No shell specified, --help for usage", prefix.red());
            }
        },
        None => {
            println!("{} No command specified, --help for usage", prefix.yellow());
        }
    }
}
