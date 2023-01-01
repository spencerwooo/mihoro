use colored::*;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};
use toml;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub remote_clash_binary_url: String,
    pub remote_config_url: String,
    pub clash_config_root: String,
}

pub fn setup_default_config(path: &String) {
    let default_config = Config {
        remote_clash_binary_url: String::from(""),
        remote_config_url: String::from(""),
        // Reference to clash config: https://github.com/Dreamacro/clash/wiki/configuration
        clash_config_root: String::from("~/.config/clash"),
    };
    let config = toml::to_string(&default_config).unwrap();
    fs::write(path, config).unwrap();
}

pub fn parse_config(path: &String) -> Config {
    let config = fs::read_to_string(path).unwrap();
    let config: Config = toml::from_str(&config).unwrap();
    config
}

pub fn download_file(url: &String, path: &String) {
    println!(
        "{} Downloading from {}",
        "download:".blue(),
        url.underline().yellow()
    );
    let mut resp = reqwest::blocking::get(url).unwrap();
    let mut file = fs::File::create(path).unwrap();
    resp.copy_to(&mut file).unwrap();
    println!(
        "{} Downloaded to {}",
        "download:".blue(),
        path.underline().yellow()
    );
}

pub fn extract_gzip(gzip_path: &String, filename: &String, prefix: &str) {
    let mut archive = GzDecoder::new(fs::File::open(gzip_path).unwrap());
    let mut file = fs::File::create(filename).unwrap();
    io::copy(&mut archive, &mut file).unwrap();
    fs::remove_file(gzip_path).unwrap();
    println!(
        "{} Extracted to {}",
        prefix.green(),
        filename.underline().yellow()
    );
}

#[derive(Debug)]
pub enum ClashrupConfigError {
    ConfigMissingError,
    RemoteClashBinaryUrlMissingError,
    RemoteConfigUrlMissingError,
}

pub fn validate_clashrup_config(
    path: &String,
    prefix: &str,
) -> Result<Config, ClashrupConfigError> {
    // Create clashrup default config if not exists
    let config_path = Path::new(path);
    if !config_path.exists() {
        setup_default_config(path);
        return Err(ClashrupConfigError::ConfigMissingError);
    }

    // Parse config file and validate if urls are defined
    println!(
        "{} Reading config from {}",
        prefix.cyan(),
        path.underline().yellow()
    );
    let config = parse_config(path);
    if config.remote_clash_binary_url.is_empty() {
        return Err(ClashrupConfigError::RemoteClashBinaryUrlMissingError);
    }
    if config.remote_config_url.is_empty() {
        return Err(ClashrupConfigError::RemoteConfigUrlMissingError);
    }
    return Ok(config);
}
