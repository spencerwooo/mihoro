use clap::{Parser, Subcommand};
use colored::*;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use toml;

use std::{fs, io, path::Path};

#[derive(Parser)]
#[command(author, about, version)]
struct Args {
    /// Path to clashrup config file
    #[clap(short, long, default_value = "config.toml")]
    clashrup_config: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Setup clashrup by downloading the clash binary and config file")]
    Setup,
}

#[derive(Serialize, Deserialize)]
struct Config {
    remote_clash_binary_url: String,
    remote_config_url: String,
    clash_config_root: String,
}

fn setup_default_config(path: &String) {
    let default_config = Config {
        remote_clash_binary_url: String::from(""),
        remote_config_url: String::from(""),
        // Reference to clash config: https://github.com/Dreamacro/clash/wiki/configuration
        clash_config_root: String::from("~/.config/clash"),
    };
    let config = toml::to_string(&default_config).unwrap();
    fs::write(path, config).unwrap();
}

fn parse_config(path: &String) -> Config {
    let config = fs::read_to_string(path).unwrap();
    let config: Config = toml::from_str(&config).unwrap();
    config
}

fn download_file(url: &String, path: &String) {
    println!(
        "{} Downloading from {}",
        "download:".bold().blue(),
        url.underline().yellow()
    );
    let mut resp = reqwest::blocking::get(url).unwrap();
    let mut file = fs::File::create(path).unwrap();
    resp.copy_to(&mut file).unwrap();
    println!(
        "{} Downloaded to {}",
        "download:".bold().blue(),
        path.underline().yellow()
    );
}

fn extract_gzip(gzip_path: &String, filename: &String) {
    let mut archive = GzDecoder::new(fs::File::open(gzip_path).unwrap());
    let mut file = fs::File::create(filename).unwrap();
    io::copy(&mut archive, &mut file).unwrap();
    fs::remove_file(gzip_path).unwrap();
    println!(
        "{} Extracted to {}",
        "clashrup:".bold().green(),
        filename.underline().yellow()
    );
}

fn validate_clashrup_config(path: &String, output_prefix: &str) -> Config {
    // Create clashrup default config if not exists
    let config_path = Path::new(path);
    if !config_path.exists() {
        setup_default_config(path);
        panic!(
            "{} Config file not found, created default one at {}",
            output_prefix,
            path.underline().yellow()
        );
    }

    // Parse config file and validate if urls are defined
    println!(
        "{} Reading config file {}",
        output_prefix,
        path.underline().yellow()
    );
    let config = parse_config(path);
    if config.remote_clash_binary_url.is_empty() {
        panic!(
            "{} Please set remote_clash_binary_url in config file",
            output_prefix.red()
        );
    }
    if config.remote_config_url.is_empty() {
        panic!(
            "{} Please set remote_config_url in config file",
            output_prefix.red()
        );
    }
    return config;
}

fn main() {
    let args = Args::parse();
    let prefix = "clashrup:".bold().cyan();

    // Initial setup and parse config file
    let config = validate_clashrup_config(&args.clashrup_config, &prefix);

    match &args.command {
        Some(Commands::Setup) => {
            // Download both clash binary and remote clash config
            let clash_gzipped_path = String::from("clash.gz");
            let clash_binary_path = String::from("clash");
            let clash_config_path = String::from("config.yaml");

            download_file(&config.remote_clash_binary_url, &clash_gzipped_path);
            extract_gzip(&clash_gzipped_path, &clash_binary_path);
            download_file(&config.remote_config_url, &clash_config_path);
        }
        None => {
            println!("{} No command specified", prefix.red());
            return;
        }
    }
}
