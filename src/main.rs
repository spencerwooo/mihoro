mod utils;

use clap::{Parser, Subcommand};
use colored::*;
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
    #[command(about = "Setup clashrup by downloading the clash binary and config file")]
    Setup,
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

    match &args.command {
        Some(Commands::Setup) => {
            // Download both clash binary and remote clash config
            let clash_gzipped_path = String::from("clash.gz");
            let clash_binary_path = String::from("clash");
            let clash_config_path = String::from("config.yaml");

            download_file(&config.remote_clash_binary_url, &clash_gzipped_path);
            extract_gzip(&clash_gzipped_path, &clash_binary_path, &prefix);
            download_file(&config.remote_config_url, &clash_config_path);
        }
        None => {
            println!("{} No command specified", prefix.red());
            return;
        }
    }
}
