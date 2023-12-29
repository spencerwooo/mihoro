use std::cmp::min;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use clap_complete::shells::Shell;
use colored::Colorize;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use reqwest::Client;
use truncatable::Truncatable;

pub fn create_parent_dir(path: &str) -> Result<()> {
    let parent_dir = Path::new(path)
        .parent()
        .with_context(|| format!("parent directory of `{}` invalid", path))?;
    if !parent_dir.exists() {
        fs::create_dir_all(parent_dir)?;
    }
    Ok(())
}

/// Download file from url to path with a reusable http client.
///
/// Renders a progress bar if content-length is available from the url headers provided. If not,
/// renders a spinner to indicate that something is downloading.
///
/// With reference from:
/// * https://github.com/mihaigalos/tutorials/blob/800d5acbc333fd4068622e9b3d870cb5b7d34e12/rust/download_with_progressbar/src/main.rs
/// * https://github.com/console-rs/indicatif/blob/2954b1a24ac5f1900a7861992e4825bff643c9e2/examples/yarnish.rs
///
/// Note: Allow `clippy::unused_io_amount` because we are writing downloaded chunks on the fly.
#[allow(clippy::unused_io_amount)]
pub async fn download_file(client: &Client, url: &str, path: &str) -> Result<()> {
    // Create parent directory for download destination if not exists
    create_parent_dir(path)?;

    // Create shared http client for multiple downloads when possible
    let res = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to GET from '{}'", &url))?;

    // If content length is not available or 0, use a spinner instead of a progress bar
    let total_size = res.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);

    let bar_style = ProgressStyle::with_template(
        "{prefix:.blue}: {msg}\n          {elapsed_precise} [{bar:30.white/blue}] \
         {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
    )?
    .progress_chars("-  ");
    let spinner_style = ProgressStyle::with_template(
        "{prefix:.blue}: {wide_msg}\n        \
         {spinner} {elapsed_precise} - Download speed {bytes_per_sec}",
    )?;

    if total_size == 0 {
        pb.set_style(spinner_style);
    } else {
        pb.set_style(bar_style);
    }
    pb.set_prefix("download");

    let truncated_url = Truncatable::from(url)
        .truncator("...".into())
        .truncate(64)
        .underline();
    pb.set_message(format!("Downloading {truncated_url}"));

    // Start file download and update progress bar when new data chunk is received
    let mut file = File::create(path)?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.with_context(|| "error while downloading file")?;

        file.write(&chunk)
            .with_context(|| "error while writing to file")?;
        if total_size != 0 {
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb.set_position(new);
        } else {
            pb.inc(chunk.len() as u64);
        }
    }

    pb.finish_with_message(format!("Downloaded to {}", path.underline()));
    Ok(())
}

pub fn delete_file(path: &str, prefix: &str) -> Result<()> {
    // Delete file if exists
    if Path::new(path).exists() {
        fs::remove_file(path).and_then(|_| {
            println!("{} Removed {}", prefix.red(), path.underline().yellow());
            Ok(())
        })?;
    }
    Ok(())
}

pub fn extract_gzip(gzip_path: &str, filename: &str, prefix: &str) -> Result<()> {
    // Create parent directory for extraction dest if not exists
    create_parent_dir(filename)?;

    // Extract gzip file
    let mut archive = GzDecoder::new(fs::File::open(gzip_path)?);
    let mut file = fs::File::create(filename)?;
    io::copy(&mut archive, &mut file)?;
    fs::remove_file(gzip_path)?;
    println!(
        "{} Extracted to {}",
        prefix.green(),
        filename.underline().yellow()
    );
    Ok(())
}

/// Create a systemd service file for running mihomo as a service.
///
/// By default, user systemd services are created under `~/.config/systemd/user/mihomo.service` and
/// invoked with `systemctl --user start mihomo.service`. Directory is created if not present.
///
/// Reference: https://wiki.metacubex.one/startup/service/
pub fn create_mihomo_service(
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
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE CAP_SYS_TIME
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE CAP_SYS_TIME
Restart=always
ExecStartPre=/usr/bin/sleep 1s
ExecStart={} -d {}
ExecReload=/bin/kill -HUP $MAINPID

[Install]
WantedBy=multi-user.target",
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

pub fn proxy_export_cmd(hostname: &str, http_port: &u16, socks_port: &u16) -> String {
    // Check current shell
    let shell = Shell::from_env().unwrap_or(Shell::Bash);
    match shell {
        Shell::Fish => {
            // For fish, use `set -gx $ENV_VAR value` to set environment variables
            format!(
                "set -gx https_proxy http://{hostname}:{http_port} \
                set -gx http_proxy http://{hostname}:{http_port} \
                set -gx all_proxy socks5://{hostname}:{socks_port}"
            )
        }
        _ => {
            // For all other shells (bash/zsh), use `export $ENV_VAR=value`
            format!(
                "export https_proxy=http://{hostname}:{http_port} \
                http_proxy=http://{hostname}:{http_port} \
                all_proxy=socks5://{hostname}:{socks_port}"
            )
        }
    }
}

pub fn proxy_unset_cmd() -> String {
    // Check current shell
    let shell = Shell::from_env().unwrap_or(Shell::Bash);
    match shell {
        Shell::Fish => {
            // For fish, use `set -e $ENV_VAR` to unset environment variables
            "set -e https_proxy http_proxy all_proxy".to_owned()
        }
        _ => {
            // For all other shells (bash/zsh), use `unset $ENV_VAR`
            "unset https_proxy http_proxy all_proxy".to_owned()
        }
    }
}
