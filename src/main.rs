mod config;
mod systemctl;
mod utils;

use std::fs;
use std::os::unix::prelude::PermissionsExt;
use std::process::Command;

use clap::Parser;
use clap::Subcommand;
use colored::Colorize;
use shellexpand::tilde;

use config::apply_clash_override;
use config::parse_config;
use config::Config;
use systemctl::Systemctl;
use utils::create_clash_service;
use utils::delete_file;
use utils::download_file;
use utils::extract_gzip;

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
    #[command(about = "Update clash remote config, mmdb, and restart clash.service")]
    Update,
    #[command(about = "Apply clash config overrides and restart clash.service")]
    Apply,
    #[command(about = "Start clash.service with systemctl")]
    Start,
    #[command(about = "Check clash.service status with systemctl")]
    Status,
    #[command(about = "Stop clash.service with systemctl")]
    Stop,
    #[command(about = "Restart clash.service with systemctl")]
    Restart,
    #[command(about = "Check clash.service logs with journalctl")]
    Log,
    #[command(about = "Output and copy proxy export shell commands")]
    Proxy,
    #[command(about = "Output and copy proxy unset shell commands")]
    ProxyUnset,
    #[command(about = "Uninstall and remove clash and config")]
    Uninstall,
}

fn main() {
    let args = Args::parse();
    let prefix = "clashrup:";
    let config_path = tilde(&args.clashrup_config).to_string();

    // Initial setup and parse config file
    let config: Config = match parse_config(&config_path, &prefix) {
        Ok(config) => config,
        Err(_) => return,
    };

    // Clash related paths and target directories
    let clash_gzipped_path = "clash.tar.gz";

    let clash_target_binary_path = tilde(&config.clash_binary_path).to_string();
    let clash_target_config_root = tilde(&config.clash_config_root).to_string();
    let clash_target_config_path =
        tilde(&format!("{}/config.yaml", config.clash_config_root)).to_string();
    let clash_target_mmdb_path =
        tilde(&format!("{}/Country.mmdb", config.clash_config_root)).to_string();
    let clash_target_service_path =
        tilde(&format!("{}/clash.service", config.user_systemd_root)).to_string();

    match &args.command {
        Some(Commands::Setup) => {
            // Download clash binary and set permission to executable
            download_file(&config.remote_clash_binary_url, &clash_gzipped_path);
            extract_gzip(&clash_gzipped_path, &clash_target_binary_path, prefix);

            let executable = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&clash_target_binary_path, executable).unwrap();

            // Download remote clash config and apply override
            download_file(&config.remote_config_url, &clash_target_config_path);
            apply_clash_override(&clash_target_config_path, &config.clash_config);

            // Download remote Country.mmdb
            download_file(&config.remote_mmdb_url, &clash_target_mmdb_path);

            // Create clash.service systemd file
            create_clash_service(
                &clash_target_binary_path,
                &clash_target_config_root,
                &clash_target_service_path,
                prefix,
            );

            Systemctl::new().enable("clash.service").execute();
            Systemctl::new().start("clash.service").execute();
        }
        Some(Commands::Update) => {
            // Download remote clash config and apply override
            download_file(&config.remote_config_url, &clash_target_config_path);
            apply_clash_override(&clash_target_config_path, &config.clash_config);
            println!("{} Updated and applied config overrides", prefix.yellow());

            // Download remote Country.mmdb
            download_file(&config.remote_mmdb_url, &clash_target_mmdb_path);

            // Restart clash systemd service
            println!("{} Restart clash.service", prefix.green());
            Systemctl::new().restart("clash.service").execute();
        }
        Some(Commands::Apply) => {
            // Apply clash config override
            apply_clash_override(&clash_target_config_path, &config.clash_config);
            println!("{} Applied clash config overrides", prefix.yellow());

            // Restart clash systemd service
            println!("{} Restart clash.service", prefix.green());
            Systemctl::new().restart("clash.service").execute();
        }
        Some(Commands::Start) => {
            Systemctl::new().start("clash.service").execute();
            println!("{} Started clash.service", prefix.green());
        }
        Some(Commands::Status) => {
            Systemctl::new().status("clash.service").execute();
        }
        Some(Commands::Stop) => {
            Systemctl::new().stop("clash.service").execute();
            println!("{} Stopped clash.service", prefix.green());
        }
        Some(Commands::Restart) => {
            Systemctl::new().restart("clash.service").execute();
            println!("{} Restarted clash.service", prefix.green());
        }
        Some(Commands::Log) => {
            Command::new("journalctl")
                .arg("--user")
                .arg("-u")
                .arg("clash.service")
                .arg("-n")
                .arg("10")
                .arg("-f")
                .spawn()
                .expect("failed to execute process")
                .wait()
                .unwrap();
        }
        Some(Commands::Proxy) => {
            // TODO: read this from clash config.yaml
            let hostname = "127.0.0.1";
            let http_port = config.clash_config.port;
            let socks_port = config.clash_config.socks_port;
            let proxy_cmd = format!(
                "export https_proxy=http://{hostname}:{http_port} \
                 http_proxy=http://{hostname}:{http_port} \
                 all_proxy=socks5://{hostname}:{socks_port}",
                hostname = hostname,
                http_port = http_port,
                socks_port = socks_port
            );
            println!("{} Run ->\n    {}", prefix.blue(), &proxy_cmd.bold());
        }
        Some(Commands::ProxyUnset) => {
            let proxy_unset = "unset https_proxy http_proxy all_proxy";
            println!("{} Run ->\n    {}", prefix.blue(), proxy_unset.bold());
        }
        Some(Commands::Uninstall) => {
            Systemctl::new().stop("clash.service").execute();
            Systemctl::new().disable("clash.service").execute();

            delete_file(&clash_target_service_path, prefix);
            delete_file(&clash_target_binary_path, prefix);
            delete_file(&clash_target_config_path, prefix);

            println!("{} Disable and reload systemd services", prefix.green());
            Systemctl::new().daemon_reload().execute();
            Systemctl::new().reset_failed().execute();
        }
        None => {
            println!("{} No command specified, --help for usage", prefix.yellow());
            return;
        }
    }
}
