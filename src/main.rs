mod utils;

use clap::{Parser, Subcommand};
use colored::*;
use systemctl;
use utils::*;

#[derive(Parser)]
#[command(author, about, version)]
struct Args {
    /// Path to clashrup config file
    #[clap(short, long, default_value = "clashrup.toml")]
    clashrup_config: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Setup clashrup by downloading the clash binary and remote config")]
    Setup,

    #[command(about = "Update clash remote config file")]
    Update,

    #[command(about = "Clash systemd status")]
    Status,

    #[command(about = "Uninstall and remove clash")]
    Uninstall,
}

fn main() {
    let args = Args::parse();
    let prefix = "clashrup:";

    // Initial setup and parse config file
    let config: Config = match validate_clashrup_config(&args.clashrup_config, &prefix) {
        Ok(config) => config,
        Err(error) => {
            match error {
                ClashrupConfigError::ConfigMissingError => {
                    println!("{} Created default config, edit as needed", prefix.yellow());
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
    let clash_binary_path = String::from("clash");
    let clash_config_path = String::from("config.yaml");

    let clash_target_binary_path = String::from("/usr/local/bin/clash");
    let clash_target_config_path = String::from("~/.config/clash/config.yaml");

    match &args.command {
        Some(Commands::Setup) => {
            sudo_check(prefix);

            // Download both clash binary and remote clash config
            download_file(&config.remote_clash_binary_url, &clash_gzipped_path);
            extract_gzip(&clash_gzipped_path, &clash_binary_path, prefix);
            download_file(&config.remote_config_url, &clash_config_path);

            // Move clash binary to user local bin and config to clash default config directory
            move_file(&clash_binary_path, &clash_target_binary_path, prefix);
            move_file(&clash_config_path, &clash_target_config_path, prefix);

            create_clash_service(&clash_target_binary_path, &clash_target_binary_path, prefix);
            systemctl::restart("clash.service").unwrap();
        }
        Some(Commands::Update) => {
            // Download remote clash config
            download_file(&config.remote_config_url, &clash_config_path);

            // Move clash config to clash default config directory
            move_file(&clash_config_path, &clash_target_config_path, prefix);

            // Restart clash systemd service
            systemctl::restart("clash.service").unwrap();
        }
        Some(Commands::Status) => {
            systemctl::status("clash.service").unwrap();
        }
        Some(Commands::Uninstall) => {
            sudo_check(prefix);

            systemctl::stop("clash.service").unwrap();

            delete_file("/etc/systemd/system/clash.service", prefix);
            delete_file(&clash_target_binary_path, prefix);
            delete_file(&clash_target_config_path, prefix);
        }
        None => {
            println!("{} No command specified, --help for usage", prefix.yellow());
            return;
        }
    }
}
