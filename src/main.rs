mod config;
mod systemctl;
mod utils;

use clap::{Parser, Subcommand};
use colored::*;
use config::*;
use shellexpand::tilde;
use std::{fs, os::unix::prelude::PermissionsExt, process::Command};
use systemctl::Systemctl;
use utils::*;

#[derive(Parser)]
#[command(author, about, version)]
struct Args {
    /// Path to clashrup config file
    #[clap(short, long, default_value = "~/.config/clashrup.toml")]
    clashrup_config: String,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Setup clashrup by downloading clash binary and remote config")]
    Setup,
    #[command(about = "Update clash remote config and restart clash.service")]
    Update,
    #[command(about = "Check clash.service status with systemctl")]
    Status,
    #[command(about = "Stop clash.service with systemctl")]
    Stop,
    #[command(about = "Restart clash.service with systemctl")]
    Restart,
    #[command(about = "Check clash.service logs with journalctl")]
    Log,
    #[command(about = "Output and copy proxy export shell commands")]
    Proxy,
    #[command(about = "Output and copy proxy unset shell commands")]
    ProxyUnset,
    #[command(about = "Uninstall and remove clash and config")]
    Uninstall,
}

fn main() {
    let args = Args::parse();
    let prefix = "clashrup:";
    let clashrup_config = tilde(&args.clashrup_config).to_string();

    // Initial setup and parse config file
    let config: Config = match validate_clashrup_config(&clashrup_config, &prefix) {
        Ok(config) => config,
        Err(error) => {
            match error {
                ClashrupConfigError::ConfigMissingError => {
                    println!(
                        "{} Created default config at {}, edit as needed",
                        prefix.yellow(),
                        clashrup_config.underline()
                    );
                    println!("{} Run again to finish setup", prefix.yellow());
                }
                ClashrupConfigError::RemoteClashBinaryUrlMissingError => {
                    println!(
                        "{} Missing {}",
                        "error:".red(),
                        "remote_clash_binary_url".underline()
                    );
                }
                ClashrupConfigError::RemoteConfigUrlMissingError => {
                    println!(
                        "{} Missing {}",
                        "error:".red(),
                        "remote_config_url".underline()
                    );
                }
            }
            return;
        }
    };

    let clash_gzipped_path = "clash.tar.gz";

    let clash_target_binary_path = tilde(&config.clash_binary_path).to_string();
    let clash_target_config_root = tilde(&config.clash_config_root).to_string();
    let clash_target_config_path =
        tilde(&format!("{}/config.yaml", config.clash_config_root)).to_string();
    let clash_target_service_path =
        tilde(&format!("{}/clash.service", config.user_systemd_root)).to_string();

    match &args.command {
        Some(Commands::Setup) => {
            // Download clash binary and set permission to executable
            download_file(&config.remote_clash_binary_url, &clash_gzipped_path);
            extract_gzip(&clash_gzipped_path, &clash_target_binary_path, prefix);
            fs::set_permissions(&clash_target_binary_path, fs::Permissions::from_mode(0o755))
                .unwrap();

            // Download clash remote configuration
            download_file(&config.remote_config_url, &clash_target_config_path);

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
            // Download remote clash config
            download_file(&config.remote_config_url, &clash_target_config_path);

            // Restart clash systemd service
            Systemctl::new().restart("clash.service").execute();
            println!("{} Restarted clash.service", prefix.green());
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
        Some(Commands::Proxy) => {
            // TODO: read this from clash config.yaml
            let hostname = "127.0.0.1";
            let http_port = 7890;
            let socks_port = 7891;
            let proxy_cmd = format!("export https_proxy=http://{hostname}:{http_port} http_proxy=http://{hostname}:{http_port} all_proxy=socks5://{hostname}:{socks_port}", hostname=hostname, http_port=http_port, socks_port=socks_port);
            println!("{} Run ->\n    {}", prefix.blue(), &proxy_cmd.bold());
        }
        Some(Commands::ProxyUnset) => {
            let proxy_unset = "unset https_proxy http_proxy all_proxy";
            println!("{} Run ->\n    {}", prefix.blue(), proxy_unset.bold());
        }
        Some(Commands::Uninstall) => {
            Systemctl::new().stop("clash.service").execute();
            Systemctl::new().disable("clash.service").execute();

            delete_file(&clash_target_service_path, prefix);
            delete_file(&clash_target_binary_path, prefix);
            delete_file(&clash_target_config_path, prefix);

            Systemctl::new().daemon_reload().execute();
            Systemctl::new().reset_failed().execute();
            println!("{} Disabled and reloaded systemd services", prefix.green());
        }
        None => {
            println!("{} No command specified, --help for usage", prefix.yellow());
            return;
        }
    }
}
