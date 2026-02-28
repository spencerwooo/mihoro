use crate::utils::create_parent_dir;

use std::{collections::HashMap, fs, path::Path};

use anyhow::{bail, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

/// Mihomo release channel for automatic binary fetching.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub enum MihomoChannel {
    #[default]
    #[serde(alias = "stable", rename(serialize = "stable"))]
    Stable,
    #[serde(alias = "alpha", rename(serialize = "alpha"))]
    Alpha,
}

/// `mihoro` configurations.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub remote_config_url: String,
    pub mihomo_channel: MihomoChannel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_mihomo_binary_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mihomo_arch: Option<String>,
    pub mihomo_binary_path: String,
    pub mihomo_config_root: String,
    pub user_systemd_root: String,
    pub mihoro_user_agent: String,
    pub auto_update_interval: u16,
    pub mihomo_config: MihomoConfig,
}

// Serde defaults for Config
impl Default for Config {
    fn default() -> Self {
        Config {
            remote_mihomo_binary_url: None,
            mihomo_channel: MihomoChannel::default(),
            mihomo_arch: None,
            remote_config_url: String::from(""),
            mihomo_binary_path: String::from("~/.local/bin/mihomo"),
            mihomo_config_root: String::from("~/.config/mihomo"),
            user_systemd_root: String::from("~/.config/systemd/user"),
            mihoro_user_agent: String::from("mihoro"),
            auto_update_interval: 12,
            mihomo_config: MihomoConfig::default(),
        }
    }
}

/// `mihomo` configurations (partial).
///
/// Referenced from https://wiki.metacubex.one/config
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct MihomoConfig {
    pub port: u16,
    pub socks_port: u16,
    pub mixed_port: Option<u16>,
    pub allow_lan: Option<bool>,
    pub bind_address: Option<String>,
    mode: MihomoMode,
    log_level: MihomoLogLevel,
    ipv6: Option<bool>,
    external_controller: Option<String>,
    external_ui: Option<String>,
    secret: Option<String>,
    pub geodata_mode: Option<bool>,
    pub geo_auto_update: Option<bool>,
    pub geo_update_interval: Option<u16>,
    pub geox_url: Option<GeoxUrl>,
    pub dns: Option<MihomoDnsConfig>,
    pub tun: Option<MihomoTunConfig>,
}

impl Default for MihomoConfig {
    fn default() -> Self {
        MihomoConfig {
            port: 7891,
            socks_port: 7892,
            mixed_port: Some(7890),
            allow_lan: Some(false),
            bind_address: Some(String::from("*")),
            mode: MihomoMode::Rule,
            log_level: MihomoLogLevel::Info,
            ipv6: Some(true),
            external_controller: Some(String::from("0.0.0.0:9090")),
            external_ui: Some(String::from("ui")),
            secret: None,
            geodata_mode: Some(false),
            geo_auto_update: Some(true),
            geo_update_interval: Some(24),
            geox_url: Some(GeoxUrl {
                geoip: String::from(
                    "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geoip.dat",
                ),
                geosite: String::from(
                    "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geosite.dat",
                ),
                mmdb: String::from(
                    "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/country.mmdb",
                ),
            }),
            dns: Some(MihomoDnsConfig::default()),
            tun: Some(MihomoTunConfig::disabled()),
        }
    }
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeoxUrl {
    pub geoip: String,
    pub geosite: String,
    pub mmdb: String,
}

/// DNS configuration for mihomo.
///
/// Referenced from https://wiki.metacubex.one/config/dns
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct MihomoDnsConfig {
    pub enable: Option<bool>,
    pub listen: Option<String>,
    pub fake_ip_range: Option<String>,
}

impl Default for MihomoDnsConfig {
    fn default() -> Self {
        MihomoDnsConfig {
            enable: Some(true),
            listen: Some(String::from("0.0.0.0:5353")),
            fake_ip_range: Some(String::from("198.18.0.1/16")),
        }
    }
}

/// TUN configuration for mihomo.
///
/// Referenced from https://wiki.metacubex.one/config/tun
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct MihomoTunConfig {
    pub enable: Option<bool>,
    pub stack: Option<String>,
    pub auto_route: Option<bool>,
    pub auto_detect_interface: Option<bool>,
    pub dns_hijack: Option<Vec<String>>,
}

impl MihomoTunConfig {
    pub fn disabled() -> Self {
        MihomoTunConfig {
            enable: Some(false),
            ..Default::default()
        }
    }
}

impl Default for MihomoTunConfig {
    fn default() -> Self {
        MihomoTunConfig {
            enable: Some(true),
            stack: Some(String::from("mixed")),
            auto_route: Some(true),
            auto_detect_interface: Some(true),
            dns_hijack: Some(vec![String::from("any:53"), String::from("tcp://any:53")]),
        }
    }
}

impl Config {
    pub fn new() -> Config {
        Config::default()
    }

    /// Read raw config string from path and parse with crate toml.
    pub fn setup_from(path: &str) -> Result<Config> {
        let raw_config = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&raw_config)?;
        Ok(config)
    }

    pub fn write(&mut self, path: &Path) -> Result<()> {
        let serialized_config = toml::to_string(&self)?;
        fs::write(path, serialized_config)?;
        Ok(())
    }
}

/// Tries to parse mihoro config as toml from path.
///
/// * If config file does not exist, creates default config file to path and returns error.
/// * If found, tries to parse the file and returns error if parse fails or fields found undefined.
pub fn parse_config(path: &str) -> Result<Config> {
    // Create mihoro default config if not exists
    let config_path = Path::new(path);
    create_parent_dir(config_path)?;

    if !config_path.exists() {
        Config::new().write(config_path)?;
        bail!(
            "created default config at `{path}`, run again to finish setup",
            path = path.underline()
        );
    }

    // Parse config file
    let config = Config::setup_from(path)?;
    let required_urls = [
        ("remote_config_url", &config.remote_config_url),
        ("mihomo_binary_path", &config.mihomo_binary_path),
        ("mihomo_config_root", &config.mihomo_config_root),
        ("user_systemd_root", &config.user_systemd_root),
    ];

    // Validate if urls are defined
    for (field, value) in required_urls.iter() {
        if value.is_empty() {
            bail!("`{}` undefined", field)
        }
    }

    Ok(config)
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

    #[serde(rename = "mixed-port", skip_serializing_if = "Option::is_none")]
    mixed_port: Option<u16>,

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

    #[serde(rename = "geodata-mode", skip_serializing_if = "Option::is_none")]
    geodata_mode: Option<bool>,

    #[serde(rename = "geo-auto-update", skip_serializing_if = "Option::is_none")]
    geo_auto_update: Option<bool>,

    #[serde(
        rename = "geo-update-interval",
        skip_serializing_if = "Option::is_none"
    )]
    geo_update_interval: Option<u16>,

    #[serde(rename = "geox-url", skip_serializing_if = "Option::is_none")]
    geox_url: Option<GeoxUrl>,

    #[serde(skip_serializing_if = "Option::is_none")]
    dns: Option<MihomoDnsYamlConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    tun: Option<MihomoTunYamlConfig>,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MihomoDnsYamlConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    enable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    listen: Option<String>,

    #[serde(rename = "fake-ip-range", skip_serializing_if = "Option::is_none")]
    fake_ip_range: Option<String>,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MihomoTunYamlConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    enable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    stack: Option<String>,

    #[serde(rename = "auto-route", skip_serializing_if = "Option::is_none")]
    auto_route: Option<bool>,

    #[serde(
        rename = "auto-detect-interface",
        skip_serializing_if = "Option::is_none"
    )]
    auto_detect_interface: Option<bool>,

    #[serde(rename = "dns-hijack", skip_serializing_if = "Option::is_none")]
    dns_hijack: Option<Vec<String>>,

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
pub fn apply_mihomo_override(path: &str, override_config: &MihomoConfig) -> Result<()> {
    let raw_mihomo_yaml = fs::read_to_string(path)?;
    let mut mihomo_yaml: MihomoYamlConfig = serde_yaml::from_str(&raw_mihomo_yaml)?;

    // Apply config overrides
    mihomo_yaml.port = Some(override_config.port);
    mihomo_yaml.socks_port = Some(override_config.socks_port);
    mihomo_yaml.mixed_port = override_config.mixed_port;
    mihomo_yaml.allow_lan = override_config.allow_lan;
    mihomo_yaml.bind_address = override_config.bind_address.clone();
    mihomo_yaml.mode = Some(override_config.mode.clone());
    mihomo_yaml.log_level = Some(override_config.log_level.clone());
    mihomo_yaml.ipv6 = override_config.ipv6;
    mihomo_yaml.external_controller = override_config.external_controller.clone();
    mihomo_yaml.external_ui = override_config.external_ui.clone();
    mihomo_yaml.secret = override_config.secret.clone();
    mihomo_yaml.geodata_mode = override_config.geodata_mode;
    mihomo_yaml.geo_auto_update = override_config.geo_auto_update;
    mihomo_yaml.geo_update_interval = override_config.geo_update_interval;
    mihomo_yaml.geox_url = override_config.geox_url.clone();

    if let Some(ref dns_override) = override_config.dns {
        let dns = mihomo_yaml.dns.get_or_insert_with(|| MihomoDnsYamlConfig {
            enable: None,
            listen: None,
            fake_ip_range: None,
            extra: HashMap::new(),
        });
        dns.enable = dns_override.enable;
        dns.listen = dns_override.listen.clone();
        dns.fake_ip_range = dns_override.fake_ip_range.clone();
    }

    if let Some(ref tun_override) = override_config.tun {
        let tun = mihomo_yaml.tun.get_or_insert_with(|| MihomoTunYamlConfig {
            enable: None,
            stack: None,
            auto_route: None,
            auto_detect_interface: None,
            dns_hijack: None,
            extra: HashMap::new(),
        });
        tun.enable = tun_override.enable;
        tun.stack = tun_override.stack.clone();
        tun.auto_route = tun_override.auto_route;
        tun.auto_detect_interface = tun_override.auto_detect_interface;
        tun.dns_hijack = tun_override.dns_hijack.clone();
    }

    // Write to file
    let serialized_mihomo_yaml = serde_yaml::to_string(&mihomo_yaml)?;
    fs::write(path, serialized_mihomo_yaml)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_parse_config_creates_default_if_not_exists() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("test.toml");

        let result = parse_config(config_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(config_path.exists());

        Ok(())
    }

    #[test]
    fn test_config_write_and_read() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("test.toml");

        let mut config = Config::new();
        config.remote_config_url = "http://example.com/config.yaml".to_string();
        config.write(&config_path)?;

        let read_config = Config::setup_from(config_path.to_str().unwrap())?;
        assert_eq!(
            read_config.remote_config_url,
            "http://example.com/config.yaml"
        );

        Ok(())
    }

    #[test]
    fn test_parse_config_validates_required_fields() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("test.toml");

        let toml_content = r#"
            mihomo_binary_path = "~/.local/bin/mihomo"
            mihomo_config_root = "~/.config/mihomo"
            user_systemd_root = "~/.config/systemd/user"
        "#;
        fs::write(&config_path, toml_content)?;

        let result = parse_config(config_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("remote_config_url"));

        Ok(())
    }

    #[test]
    fn test_apply_mihomo_override() -> Result<()> {
        let dir = tempdir()?;
        let yaml_path = dir.path().join("config.yaml");

        let yaml_content = r#"
            port: 8080
            socks-port: 8081
            mixed-port: 7890
            allow-lan: false
            mode: rule
            log-level: info
            proxies:
              - name: "test"
                type: http
                server: example.com
                port: 443
        "#;
        fs::write(&yaml_path, yaml_content)?;

        let override_config = MihomoConfig {
            port: 7891,
            socks_port: 7892,
            ..Default::default()
        };

        apply_mihomo_override(yaml_path.to_str().unwrap(), &override_config)?;

        let updated_content = fs::read_to_string(&yaml_path)?;
        assert!(updated_content.contains("port: 7891"));
        assert!(updated_content.contains("socks-port: 7892"));
        assert!(updated_content.contains("proxies:"));

        Ok(())
    }
}
