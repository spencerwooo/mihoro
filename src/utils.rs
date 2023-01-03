use std::fs;
use std::io;
use std::path::Path;

use colored::Colorize;
use flate2::read::GzDecoder;

pub fn download_file(url: &str, path: &str) {
    // Create parent directory for download dest if not exists
    let parent_dir = Path::new(path).parent().unwrap();
    if !parent_dir.exists() {
        fs::create_dir_all(parent_dir).unwrap();
    }

    // Download file
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

pub fn delete_file(path: &str, prefix: &str) {
    // Delete file if exists
    if Path::new(path).exists() {
        fs::remove_file(path).unwrap();
        println!("{} Removed {}", prefix.red(), path.underline().yellow());
    }
}

pub fn extract_gzip(gzip_path: &str, filename: &str, prefix: &str) {
    // Create parent directory for extraction dest if not exists
    let parent_dir = Path::new(filename).parent().unwrap();
    if !parent_dir.exists() {
        fs::create_dir_all(parent_dir).unwrap();
    }

    // Extract gzip file
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

/// Create a systemd service file for running clash as a service.
///
/// By default, user systemd services are created under `~/.config/systemd/user/clash.service` and invoked with
/// `systemctl --user start clash.service`. Directory is created if not present.
///
/// Reference: https://github.com/Dreamacro/clash/wiki/Running-Clash-as-a-service
pub fn create_clash_service(
    clash_binary_path: &str,
    clash_config_root: &str,
    clash_service_path: &str,
    prefix: &str,
) {
    let service = format!(
        "[Unit]
Description=Clash - A rule-based tunnel in Go.
After=network.target

[Service]
Type=simple
ExecStart={clash_binary_path} -d {clash_config_root}
Restart=always

[Install]
WantedBy=multi-user.target",
        clash_binary_path = clash_binary_path,
        clash_config_root = clash_config_root
    );

    // Create clash service directory if not exists
    let clash_service_dir = Path::new(clash_service_path).parent().unwrap();
    if !clash_service_dir.exists() {
        fs::create_dir_all(clash_service_dir).unwrap();
    }

    // Write clash.service contents to file
    fs::write(clash_service_path, service).unwrap();

    println!(
        "{} Created clash.service at {}",
        prefix.green(),
        clash_service_path.underline().yellow()
    );
}
