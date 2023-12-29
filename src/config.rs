use std::collections::HashMap;
use std::fs;
use std::path::Path;

use colored::Colorize;
use serde::Deserialize;
use serde::Serialize;

/// `mihoro` configurations.
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub remote_mihomo_binary_url: String,
    pub remote_config_url: String,
    pub remote_mmdb_url: String,
    pub mihomo_binary_path: String,
    pub mihomo_config_root: String,
    pub user_systemd_root: String,
    pub mihomo_config: MihomoConfig,
}

/// `mihomo` configurations (partial).
///
/// Referenced from https://github.com/Dreamacro/mihomo/wiki/configuration
#[derive(Serialize, Deserialize, Debug)]
pub struct MihomoConfig {
    pub port: u16,
    pub socks_port: u16,
    pub allow_lan: Option<bool>,
    pub bind_address: Option<String>,
    mode: MihomoMode,
    log_level: MihomoLogLevel,
    ipv6: Option<bool>,
    external_controller: Option<String>,
    external_ui: Option<String>,
    secret: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MihomoMode {
    #[serde(alias = "global", rename(serialize = "global"))]
    Global,
    #[serde(alias = "rule", rename(serialize = "rule"))]
    Rule,
    #[serde(alias = "direct", rename(serialize = "direct"))]
    Direct,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MihomoLogLevel {
    #[serde(alias = "silent", rename(serialize = "silent"))]
    Silent,
    #[serde(alias = "error", rename(serialize = "error"))]
    Error,
    #[serde(alias = "warning", rename(serialize = "warning"))]
    Warning,
    #[serde(alias = "info", rename(serialize = "info"))]
    Info,
    #[serde(alias = "debug", rename(serialize = "debug"))]
    Debug,
}

impl Config {
    pub fn new() -> Config {
        Config {
            remote_mihomo_binary_url: String::from(""),
            remote_config_url: String::from(""),
            remote_mmdb_url: String::from(
                "https://cdn.jsdelivr.net/gh/Dreamacro/maxmind-geoip@release/Country.mmdb",
            ),
            mihomo_binary_path: String::from("~/.local/bin/mihomo"),
            mihomo_config_root: String::from("~/.config/mihomo"),
            user_systemd_root: String::from("~/.config/systemd/user"),
            mihomo_config: MihomoConfig {
                port: 7890,
                socks_port: 7891,
                allow_lan: Some(false),
                bind_address: Some(String::from("*")),
                mode: MihomoMode::Rule,
                log_level: MihomoLogLevel::Info,
                ipv6: Some(false),
                external_controller: Some(String::from("127.0.0.1:9090")),
                external_ui: None,
                secret: None,
            },
        }
    }

    /// Read raw config string from path and parse with crate toml.
    ///
    /// TODO: Currently this will return error that shows a missing field error when parse fails,
    /// however the error message always shows the line and column number as `line 1 column 1`,
    /// which is because the function `fs::read_to_string` preserves newline characters as `\n`,
    /// resulting in a single-lined string.
    pub fn setup_from(path: &str) -> Result<Config, toml::de::Error> {
        let raw_config = fs::read_to_string(path).unwrap();
        toml::from_str(&raw_config)
    }

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

/// Tries to parse mihoro config as toml from path.
///
/// * If config file does not exist, creates default config file to path and returns error.
/// * If found, tries to parse the file and returns error if parse fails or fields found undefined.
pub fn parse_config(path: &str, prefix: &str) -> Result<Config, ConfigError> {
    // Create `~/.config` directory if not exists
    let parent_dir = Path::new(path).parent().unwrap();
    if !parent_dir.exists() {
        fs::create_dir_all(parent_dir).unwrap();
    }

    // Create mihoro default config if not exists
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
    // println!("{} Reading config from {}", prefix.cyan(), path.underline());
    match Config::setup_from(path) {
        Ok(config) => {
            let required_urls = [
                ("remote_config_url", &config.remote_config_url),
                ("remote_mmdb_url", &config.remote_mmdb_url),
                ("mihomo_binary_path", &config.mihomo_binary_path),
                ("mihomo_config_root", &config.mihomo_config_root),
                ("user_systemd_root", &config.user_systemd_root),
            ];

            for (field, value) in required_urls.iter() {
                if value.is_empty() {
                    println!("{} `{}` undefined", "error:".red(), field);
                    return Err(ConfigError::ParseError);
                }
            }

            Ok(config)
        }
        Err(error) => {
            println!("{} {}", "error:".red(), error);
            Err(ConfigError::ParseError)
        }
    }
}

/// `mihomoYamlConfig` is defined to support serde serialization and deserialization of arbitrary
/// mihomo `config.yaml`, with support for fields defined in `mihomoConfig` for overrides and also
/// extra fields that are not managed by `mihoro` by design (namely `proxies`, `proxy-groups`,
/// `rules`, etc.)
#[derive(Serialize, Deserialize, Debug)]
pub struct MihomoYamlConfig {
    port: Option<u16>,

    #[serde(rename = "socks-port")]
    socks_port: Option<u16>,

    #[serde(rename = "allow-lan", skip_serializing_if = "Option::is_none")]
    allow_lan: Option<bool>,

    #[serde(rename = "bind-address", skip_serializing_if = "Option::is_none")]
    bind_address: Option<String>,

    mode: Option<MihomoMode>,

    #[serde(rename = "log-level")]
    log_level: Option<MihomoLogLevel>,

    #[serde(skip_serializing_if = "Option::is_none")]
    ipv6: Option<bool>,

    #[serde(
        rename = "external-controller",
        skip_serializing_if = "Option::is_none"
    )]
    external_controller: Option<String>,

    #[serde(rename = "external-ui", skip_serializing_if = "Option::is_none")]
    external_ui: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    secret: Option<String>,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

/// Apply config overrides to mihomo's `config.yaml`.
///
/// Only a subset of mihomo's config fields are supported, as defined in `mihomoConfig`.
///
/// Rules:
/// * Fields defined in `mihoro.toml` will override the downloaded remote `config.yaml`.
/// * Fields undefined will be removed from the downloaded `config.yaml`.
/// * Fields not supported by `mihoro` will be kept as is.
pub fn apply_mihomo_override(path: &str, override_config: &MihomoConfig) {
    let raw_mihomo_yaml = fs::read_to_string(path).unwrap();
    let mut mihomo_yaml: MihomoYamlConfig = serde_yaml::from_str(&raw_mihomo_yaml).unwrap();

    // Apply config overrides
    mihomo_yaml.port = Some(override_config.port);
    mihomo_yaml.socks_port = Some(override_config.socks_port);
    mihomo_yaml.allow_lan = override_config.allow_lan;
    mihomo_yaml.bind_address = override_config.bind_address.clone();
    mihomo_yaml.mode = Some(override_config.mode.clone());
    mihomo_yaml.log_level = Some(override_config.log_level.clone());
    mihomo_yaml.ipv6 = override_config.ipv6;
    mihomo_yaml.external_controller = override_config.external_controller.clone();
    mihomo_yaml.external_ui = override_config.external_ui.clone();
    mihomo_yaml.secret = override_config.secret.clone();

    // Write to file
    let serialized_mihomo_yaml = serde_yaml::to_string(&mihomo_yaml).unwrap();
    fs::write(path, serialized_mihomo_yaml).unwrap();
}
