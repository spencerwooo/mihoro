use crate::cmd::ProxyCommands;
use crate::config::{apply_mihomo_override, parse_config, Config};
use crate::proxy::{proxy_export_cmd, proxy_unset_cmd};
use crate::systemctl::Systemctl;
use crate::utils::{create_parent_dir, delete_file, download_file, extract_gzip};

use std::fs;
use std::os::unix::prelude::PermissionsExt;

use anyhow::Result;
use colored::Colorize;
use local_ip_address::local_ip;
use reqwest::Client;
use shellexpand::tilde;

#[derive(Debug)]
pub struct Mihoro {
    // global mihoro config
    pub prefix: String,
    pub config: Config,

    // mihomo global variables derived from mihoro config
    pub mihomo_target_binary_path: String,
    pub mihomo_target_config_root: String,
    pub mihomo_target_config_path: String,
    pub mihomo_target_service_path: String,
}

impl Mihoro {
    pub fn new(config_path: &String) -> Result<Mihoro> {
        let config = parse_config(tilde(&config_path).as_ref())?;
        return Ok(Mihoro {
            prefix: String::from("mihoro:"),
            config: config.clone(),
            mihomo_target_binary_path: tilde(&config.mihomo_binary_path).to_string(),
            mihomo_target_config_root: tilde(&config.mihomo_config_root).to_string(),
            mihomo_target_config_path: tilde(&format!("{}/config.yaml", config.mihomo_config_root))
                .to_string(),
            mihomo_target_service_path: tilde(&format!(
                "{}/mihomo.service",
                config.user_systemd_root
            ))
            .to_string(),
        });
    }

    pub async fn setup(&self, client: Client) -> Result<()> {
        println!(
            "{} Setting up mihomo's binary, config, and systemd service...",
            &self.prefix.cyan()
        );

        // Attempt to download and setup mihomo binary if needed
        if fs::metadata(&self.mihomo_target_binary_path).is_ok() {
            // If mihomo binary already exists at `mihomo_target_binary_path`, then skip setup
            println!(
                "{} Assuming mihomo binary already installed at {}, skipping setup",
                self.prefix.yellow(),
                self.mihomo_target_binary_path.underline().green()
            );
        } else {
            // Download mihomo binary and set permission to executable
            download_file(
                &client,
                &self.config.remote_mihomo_binary_url,
                "mihomo-downloaded-binary.tar.gz",
            )
            .await?;
            extract_gzip(
                "mihomo-downloaded-binary.tar.gz",
                &self.mihomo_target_binary_path,
                &self.prefix,
            )?;

            let executable = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&self.mihomo_target_binary_path, executable)?;
        }

        // Download remote mihomo config and apply override
        download_file(
            &client,
            &self.config.remote_config_url,
            &self.mihomo_target_config_path,
        )
        .await?;
        apply_mihomo_override(&self.mihomo_target_config_path, &self.config.mihomo_config)?;

        // Download geodata
        self.update_geodata(client).await?;

        // Create mihomo.service systemd file
        create_mihomo_service(
            &self.mihomo_target_binary_path,
            &self.mihomo_target_config_root,
            &self.mihomo_target_service_path,
            &self.prefix,
        )?;

        Systemctl::new().enable("mihomo.service").execute()?;
        Systemctl::new().start("mihomo.service").execute()?;
        Ok(())
    }

    pub async fn update(&self, client: Client) -> Result<()> {
        // Download remote mihomo config and apply override
        download_file(
            &client,
            &self.config.remote_config_url,
            &self.mihomo_target_config_path,
        )
        .await?;
        apply_mihomo_override(&self.mihomo_target_config_path, &self.config.mihomo_config)?;
        println!(
            "{} Updated and applied config overrides",
            self.prefix.yellow()
        );

        // Restart mihomo systemd service
        println!("{} Restart mihomo.service", self.prefix.green());
        Systemctl::new().restart("mihomo.service").execute()?;
        Ok(())
    }

    pub async fn update_geodata(&self, client: Client) -> Result<()> {
        match self.config.mihomo_config.geox_url.clone() {
            Some(geox_url) => {
                // Download geodata files based on `geodata_mode`
                let geodata_mode = self.config.mihomo_config.geodata_mode.unwrap_or(false);
                if geodata_mode {
                    download_file(
                        &client,
                        &geox_url.geoip,
                        format!("{}/geoip.dat", &self.mihomo_target_config_root).as_str(),
                    )
                    .await?;
                    download_file(
                        &client,
                        &geox_url.geosite,
                        format!("{}/geosite.dat", &self.mihomo_target_config_root).as_str(),
                    )
                    .await?;
                } else {
                    download_file(
                        &client,
                        &geox_url.mmdb,
                        format!("{}/country.mmdb", &self.mihomo_target_config_root).as_str(),
                    )
                    .await?;
                }

                println!("{} Downloaded and updated geodata", self.prefix.green());
            }
            None => {
                println!(
                    "{} `geox_url` undefined, refer to {}",
                    self.prefix.yellow(),
                    "'https://wiki.metacubex.one/config/general/#geo_3'"
                        .bold()
                        .underline()
                );
            }
        }
        Ok(())
    }

    pub async fn apply(&self) -> Result<()> {
        // Apply mihomo config override
        apply_mihomo_override(&self.mihomo_target_config_path, &self.config.mihomo_config).map(
            |_| {
                println!(
                    "{} Applied mihomo config overrides",
                    self.prefix.green().bold()
                );
            },
        )?;

        // Restart mihomo systemd service
        Systemctl::new()
            .restart("mihomo.service")
            .execute()
            .map(|_| {
                println!("{} Restarted mihomo.service", self.prefix.green().bold());
            })?;
        Ok(())
    }

    pub fn uninstall(&self) -> Result<()> {
        Systemctl::new().stop("mihomo.service").execute()?;
        Systemctl::new().disable("mihomo.service").execute()?;

        delete_file(&self.mihomo_target_service_path, &self.prefix)?;
        delete_file(&self.mihomo_target_config_path, &self.prefix)?;

        Systemctl::new().daemon_reload().execute()?;
        Systemctl::new().reset_failed().execute()?;
        println!(
            "{} Disabled and reloaded systemd services",
            self.prefix.green()
        );
        println!(
            "{} You may need to remove mihomo binary and config directory manually",
            self.prefix.yellow()
        );

        let remove_cmd = format!(
            "rm -R {} {}",
            self.mihomo_target_binary_path, self.mihomo_target_config_root
        );
        println!("{} `{}`", "->".dimmed(), remove_cmd.underline().bold());
        Ok(())
    }

    pub fn proxy_commands(&self, proxy: &Option<ProxyCommands>) -> Result<()> {
        match proxy {
            Some(ProxyCommands::Export) => {
                println!(
                    "{}",
                    proxy_export_cmd(
                        "127.0.0.1",
                        &self.config.mihomo_config.port,
                        &self.config.mihomo_config.socks_port
                    )
                )
            }
            Some(ProxyCommands::ExportLan) => {
                if !self.config.mihomo_config.allow_lan.unwrap_or(false) {
                    println!(
                        "{} `{}` is false, proxy is not available for LAN",
                        "warning:".yellow(),
                        "allow_lan".bold()
                    );
                }

                println!(
                    "{}",
                    proxy_export_cmd(
                        &local_ip()?.to_string(),
                        &self.config.mihomo_config.port,
                        &self.config.mihomo_config.socks_port
                    )
                );
            }
            Some(ProxyCommands::Unset) => {
                println!("{}", proxy_unset_cmd())
            }
            _ => (),
        }
        Ok(())
    }
}

/// Create a systemd service file for running mihomo as a service.
///
/// By default, user systemd services are created under `~/.config/systemd/user/mihomo.service` and
/// invoked with `systemctl --user start mihomo.service`. Directory is created if not present.
///
/// Reference: https://wiki.metacubex.one/startup/service/
fn create_mihomo_service(
    mihomo_binary_path: &str,
    mihomo_config_root: &str,
    mihomo_service_path: &str,
    prefix: &str,
) -> Result<()> {
    let service = format!(
        "[Unit]
Description=mihomo Daemon, Another Clash Kernel.
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
LimitNPROC=500
LimitNOFILE=1000000
Restart=always
ExecStartPre=/usr/bin/sleep 1s
ExecStart={} -d {}
ExecReload=/bin/kill -HUP $MAINPID

[Install]
WantedBy=default.target",
        mihomo_binary_path, mihomo_config_root
    );

    // Create mihomo service directory if not exists
    create_parent_dir(mihomo_service_path)?;

    // Write mihomo.service contents to file
    fs::write(mihomo_service_path, service)?;

    println!(
        "{} Created mihomo.service at {}",
        prefix.green(),
        mihomo_service_path.underline().yellow()
    );
    Ok(())
}
