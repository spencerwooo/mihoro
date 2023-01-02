mod utils;

use clap::{Parser, Subcommand};
use colored::*;
use shellexpand::tilde;
use systemctl;
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
    #[command(about = "Update clash remote config file")]
    Update,
    #[command(about = "Check clash.service status with systemctl")]
    Status,
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

    let clash_gzipped_path = String::from("clash.gz");
    // let clash_binary_path = String::from("clash");
    // let clash_config_path = String::from("config.yaml");

    let clash_target_binary_path = tilde(&format!("~/.local/bin/clash")).to_string();
    let clash_target_config_path =
        tilde(&format!("{}/config.yaml", config.clash_config_root)).to_string();
    let clash_target_service_path =
        tilde(&format!("{}/clash.service", config.user_systemd_root)).to_string();

    match &args.command {
        Some(Commands::Setup) => {
            // Download both clash binary and remote clash config
            download_file(&config.remote_clash_binary_url, &clash_gzipped_path);
            extract_gzip(&clash_gzipped_path, &clash_target_binary_path, prefix);
            download_file(&config.remote_config_url, &clash_target_config_path);

            // Move clash binary to user local bin and config to clash default config directory
            // move_file(&clash_binary_path, &clash_target_binary_path, prefix);
            // move_file(&clash_config_path, &clash_target_config_path, prefix);

            create_clash_service(
                &clash_target_binary_path,
                &clash_target_binary_path,
                &clash_target_service_path,
                prefix,
            );
            systemctl::restart("clash.service").unwrap();
        }
        Some(Commands::Update) => {
            // Download remote clash config
            download_file(&config.remote_config_url, &clash_target_config_path);

            // Move clash config to clash default config directory
            // move_file(&clash_config_path, &clash_target_config_path, prefix);

            // Restart clash systemd service
            systemctl::restart("clash.service").unwrap();
        }
        Some(Commands::Status) => {
            systemctl::status("clash.service").unwrap();
        }
        Some(Commands::Uninstall) => {
            systemctl::stop("clash.service").unwrap();

            delete_file(&clash_target_service_path, prefix);
            delete_file(&clash_target_binary_path, prefix);
            delete_file(&clash_target_config_path, prefix);
        }
        None => {
            println!("{} No command specified, --help for usage", prefix.yellow());
            return;
        }
    }
}
