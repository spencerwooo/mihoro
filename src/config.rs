use std::fs;
use std::path::Path;

use colored::Colorize;
use serde::Deserialize;
use serde::Serialize;
use toml;

/// `clashrup` configurations
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub remote_clash_binary_url: String,
    pub remote_config_url: String,
    pub remote_mmdb_url: String,
    pub clash_binary_path: String,
    pub clash_config_root: String,
    pub user_systemd_root: String,
    pub clash_config: ClashConfig,
}

/// `clash` configurations (partial)
///
/// Referenced from https://github.com/Dreamacro/clash/wiki/configuration
#[derive(Serialize, Deserialize, Debug)]
pub struct ClashConfig {
    port: Option<u16>,
    socks_port: Option<u16>,
    allow_lan: Option<bool>,
    bind_address: Option<String>,
    mode: Option<ClashMode>,
    log_level: Option<ClashLogLevel>,
    ipv6: Option<bool>,
    external_controller: Option<String>,
    external_ui: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClashMode {
    Global,
    Rule,
    Direct,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClashLogLevel {
    Silent,
    Error,
    Warning,
    Info,
    Debug,
}

impl Config {
    pub fn new() -> Config {
        Config {
            remote_clash_binary_url: String::from(""),
            remote_config_url: String::from(""),
            remote_mmdb_url: String::from(
                "https://cdn.jsdelivr.net/gh/Dreamacro/maxmind-geoip@release/Country.mmdb",
            ),
            clash_binary_path: String::from("~/.local/bin/clash"),
            clash_config_root: String::from("~/.config/clash"),
            user_systemd_root: String::from("~/.config/systemd/user"),
            clash_config: ClashConfig {
                port: Some(7890),
                socks_port: Some(7891),
                allow_lan: Some(false),
                bind_address: Some(String::from("*")),
                mode: Some(ClashMode::Rule),
                log_level: Some(ClashLogLevel::Info),
                ipv6: Some(false),
                external_controller: Some(String::from("127.0.0.1:9090")),
                external_ui: None,
            },
        }
    }

    /// Read raw config string from path and parse with crate toml
    ///
    /// TODO: Currently this will return error that shows a missing field error when parse fails, however the error
    /// message always shows the line and column number as `line 1 column 1`, which is because the function
    /// `fs::read_to_string` preserves newline characters as `\n`, resulting in a single-lined string.
    pub fn setup_from(path: &str) -> Result<Config, toml::de::Error> {
        let raw_config = fs::read_to_string(path).unwrap();
        toml::from_str(&raw_config)
    }

    /// Write config to path
    pub fn write(&mut self, path: &Path) {
        let serialized_config = toml::to_string(&self).unwrap();
        fs::write(path, serialized_config).unwrap();
    }
}

#[derive(Debug)]
pub enum ConfigError {
    FileMissing,
    ParseError,
}

/// Tries to parse clashrup config as toml from path
///
/// * If config file does not exist, creates default config file to path and returns error
/// * If found, tries to parse config file and returns error if parse fails or some fields are not defined
pub fn parse_config(path: &str, prefix: &str) -> Result<Config, ConfigError> {
    // Create clashrup default config if not exists
    let config_path = Path::new(path);
    if !config_path.exists() {
        Config::new().write(config_path);
        println!(
            "{prefix} Created default config at {path}, edit as needed\n{prefix} Run again to finish setup",
            prefix = prefix.yellow(),
            path = path.underline()
        );
        return Err(ConfigError::FileMissing);
    }

    // Parse config file and validate if urls are defined
    println!(
        "{} Reading config from {}",
        prefix.cyan(),
        path.underline().yellow()
    );
    match Config::setup_from(path) {
        Ok(config) => {
            if config.remote_clash_binary_url.is_empty() {
                println!("{} `remote_clash_binary_url` undefined", "error:".red());
                return Err(ConfigError::ParseError);
            }
            if config.remote_config_url.is_empty() {
                println!("{} `remote_config_url` undefined", "error:".red());
                return Err(ConfigError::ParseError);
            }
            if config.remote_mmdb_url.is_empty() {
                println!("{} `remote_mmdb_url` undefined", "error:".red());
                return Err(ConfigError::ParseError);
            }
            return Ok(config);
        }
        Err(error) => {
            println!("{} {}", "error:".red(), error);
            return Err(ConfigError::ParseError);
        }
    };
}
