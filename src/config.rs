use colored::*;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use toml;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub remote_clash_binary_url: String,
    pub remote_config_url: String,
    pub clash_config_root: String,
    pub user_systemd_root: String,
}

impl Config {
    pub fn new() -> Config {
        Config {
            remote_clash_binary_url: String::from(""),
            remote_config_url: String::from(""),
            clash_config_root: String::from("~/.config/clash"),
            user_systemd_root: String::from("~/.config/systemd/user"),
        }
    }
}

pub fn setup_default_config(path: &str) {
    let default_config = Config::new();
    let config = toml::to_string(&default_config).unwrap();
    fs::write(path, config).unwrap();
}

pub fn parse_config(path: &str) -> Config {
    let config = fs::read_to_string(path).unwrap();
    let config: Config = toml::from_str(&config).unwrap();
    config
}

#[derive(Debug)]
pub enum ClashrupConfigError {
    ConfigMissingError,
    RemoteClashBinaryUrlMissingError,
    RemoteConfigUrlMissingError,
}

pub fn validate_clashrup_config(path: &str, prefix: &str) -> Result<Config, ClashrupConfigError> {
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
