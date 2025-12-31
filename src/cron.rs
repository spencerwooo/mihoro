use anyhow::{anyhow, Result};
use colored::Colorize;
use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Get the path to the user's crontab file
fn crontab_path() -> PathBuf {
    let run_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        // Use current user's UID as fallback
        let uid = fs::metadata(".").map(|m| m.uid()).unwrap_or(1000);
        format!("/run/user/{}", uid)
    });
    PathBuf::from(run_dir).join("mihoro-crontab")
}

/// Get the mihoro binary path from current executable
fn mihoro_bin_path() -> Result<String> {
    env::current_exe()?
        .to_str()
        .map(String::from)
        .ok_or_else(|| anyhow!("Failed to get mihoro binary path"))
}

/// Generate cron entry for auto-update
fn generate_cron_entry(interval_hours: u16) -> Result<String> {
    let bin_path = mihoro_bin_path()?;
    Ok(format!(
        "0 */{} * * * {} update\n",
        interval_hours, bin_path
    ))
}

/// Generate the crontab content with mihoro entry
fn generate_crontab(interval_hours: u16) -> Result<String> {
    let mihoro_entry = generate_cron_entry(interval_hours)?;
    Ok(mihoro_entry)
}

/// Enable auto-update by installing cron job
pub fn enable_auto_update(interval_hours: u16, prefix: &str) -> Result<()> {
    if interval_hours == 0 {
        println!(
            "{} Auto-update interval is 0, disabling auto-update",
            prefix.yellow()
        );
        return disable_auto_update(prefix);
    }

    if interval_hours > 24 {
        anyhow::bail!("Auto-update interval must be between 1 and 24 hours");
    }

    let crontab_content = generate_crontab(interval_hours)?;
    let crontab_file = crontab_path();

    // Write crontab to runtime directory for reference
    fs::write(&crontab_file, crontab_content)?;

    // Install crontab using crontab command
    let status = std::process::Command::new("crontab")
        .arg(&crontab_file)
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to install crontab");
    }

    println!(
        "{} Auto-update enabled with interval: {} hours",
        prefix.green().bold(),
        interval_hours.to_string().yellow()
    );
    println!(
        "{} Cron entry: {}",
        "->".dimmed(),
        generate_cron_entry(interval_hours)?.trim()
    );

    Ok(())
}

/// Disable auto-update by removing cron job
pub fn disable_auto_update(prefix: &str) -> Result<()> {
    let crontab_file = crontab_path();

    // Remove our crontab reference file
    if crontab_file.exists() {
        fs::remove_file(&crontab_file)?;
    }

    // Install empty crontab to remove all entries
    let status = std::process::Command::new("crontab").arg("-r").status();

    match status {
        Ok(status) if status.success() => {
            println!("{} Auto-update disabled", prefix.green().bold());
            Ok(())
        }
        Ok(_) => {
            // crontab -r returns non-zero if no crontab exists, which is fine
            println!(
                "{} Auto-update disabled (no active cron job)",
                prefix.yellow()
            );
            Ok(())
        }
        Err(e) => Err(anyhow!("Failed to disable crontab: {}", e)),
    }
}

/// Format Unix timestamp to local datetime string using date command
fn format_datetime(secs: u64) -> String {
    let output = Command::new("date")
        .arg("-d")
        .arg(format!("@{}", secs))
        .arg("+%Y-%m-%d %H:%M:%S")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => format!("<unknown: {} secs>", secs),
    }
}

/// Get current cron status
pub fn get_cron_status(_prefix: &str, mihomo_config_path: &str) -> Result<()> {
    let crontab_file = crontab_path();

    if !crontab_file.exists() {
        println!("{} Auto-update is disabled", "status:".yellow().bold());
        return Ok(());
    }

    let content = fs::read_to_string(&crontab_file)?;
    let cron_entry = content.lines().next().unwrap_or("");

    println!("{} Auto-update is enabled", "status:".green().bold());
    println!("{} {}", "->".dimmed(), cron_entry.dimmed());

    // Show last updated time from mihomo config file
    let config_path = Path::new(mihomo_config_path);
    if let Ok(metadata) = fs::metadata(config_path) {
        if let Ok(modified) = metadata.modified() {
            use std::time::UNIX_EPOCH;
            if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                let secs = duration.as_secs();
                let datetime = format_datetime(secs);
                println!("{} Last updated: {}", "->".dimmed(), datetime.dimmed());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_cron_entry() {
        let entry = generate_cron_entry(12).unwrap();
        assert!(entry.contains("0 */12 * * *"));
        assert!(entry.contains("update"));
    }

    #[test]
    fn test_generate_crontab() {
        let crontab = generate_crontab(6).unwrap();
        assert!(crontab.contains("0 */6 * * *"));
    }
}
