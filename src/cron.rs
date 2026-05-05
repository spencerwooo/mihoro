use anyhow::{anyhow, Result};
use colored::Colorize;
use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Get the path to the staging file used to install crontab via `crontab <file>`.
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

/// Read the current user's crontab via `crontab -l`. Returns an empty string
/// when no crontab exists or the command fails for any other reason — the
/// resulting empty input is treated as "no preexisting entries".
fn read_current_crontab() -> String {
    Command::new("crontab")
        .arg("-l")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// Decide whether a crontab line was installed by mihoro and should be
/// replaced/removed by us. Comments and blank lines are never matched so we
/// don't accidentally drop user annotations.
fn is_mihoro_entry(line: &str, bin_path: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return false;
    }
    // Primary: exact current binary path.
    if trimmed.contains(&format!("{} update", bin_path)) {
        return true;
    }
    // Fallback: any path ending in `/mihoro update`. Catches stale entries
    // installed by a previous mihoro that lived at a different path.
    trimmed.ends_with("/mihoro update")
}

/// Merge `new_entry` into `existing` crontab content, replacing any prior
/// mihoro entry. Returns the new crontab body with a single trailing newline.
fn merge_crontab(existing: &str, new_entry: &str, bin_path: &str) -> String {
    let mut kept: Vec<String> = existing
        .lines()
        .filter(|l| !is_mihoro_entry(l, bin_path))
        .map(String::from)
        .collect();
    kept.push(new_entry.trim_end().to_string());
    let mut joined = kept.join("\n");
    joined.push('\n');
    joined
}

/// Filter out any mihoro-owned entries from `existing`. Returns `None` if no
/// non-mihoro entries remain (caller should `crontab -r` instead of installing
/// an empty file, which some `crontab` implementations reject).
fn strip_mihoro_entries(existing: &str, bin_path: &str) -> Option<String> {
    let kept: Vec<&str> = existing
        .lines()
        .filter(|l| !is_mihoro_entry(l, bin_path))
        .collect();
    if kept.is_empty() {
        None
    } else {
        let mut joined = kept.join("\n");
        joined.push('\n');
        Some(joined)
    }
}

/// Stage `content` to a temp file and install it via `crontab <file>`.
fn install_crontab(content: &str) -> Result<()> {
    let crontab_file = crontab_path();
    fs::write(&crontab_file, content)?;
    let status = Command::new("crontab").arg(&crontab_file).status()?;
    if !status.success() {
        anyhow::bail!("Failed to install crontab");
    }
    Ok(())
}

/// Enable auto-update by installing/refreshing the mihoro cron entry while
/// preserving every other entry in the user's crontab.
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

    let bin_path = mihoro_bin_path()?;
    let new_entry = generate_cron_entry(interval_hours)?;
    let existing = read_current_crontab();
    let merged = merge_crontab(&existing, &new_entry, &bin_path);

    install_crontab(&merged)?;

    println!(
        "{} Auto-update enabled with interval: {} hours",
        prefix.green().bold(),
        interval_hours.to_string().yellow()
    );
    println!("{} Cron entry: {}", "->".dimmed(), new_entry.trim());

    Ok(())
}

/// Disable auto-update by removing only mihoro's cron entry. Other user
/// entries are preserved. If mihoro was the only entry, the crontab is
/// removed entirely with `crontab -r`.
pub fn disable_auto_update(prefix: &str) -> Result<()> {
    let bin_path = mihoro_bin_path()?;
    let existing = read_current_crontab();

    match strip_mihoro_entries(&existing, &bin_path) {
        Some(remaining) => {
            install_crontab(&remaining)?;
        }
        None => {
            // No other entries — remove the crontab entirely. `crontab -r`
            // exits non-zero when there is no crontab to remove; treat that
            // as success rather than an error.
            let _ = Command::new("crontab").arg("-r").status();
        }
    }

    // Best-effort cleanup of the staging file from older mihoro versions.
    let crontab_file = crontab_path();
    if crontab_file.exists() {
        let _ = fs::remove_file(&crontab_file);
    }

    println!("{} Auto-update disabled", prefix.green().bold());
    Ok(())
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

/// Get current cron status by inspecting the live crontab. This is robust
/// across reboots and across manual `crontab -e` edits.
pub fn get_cron_status(_prefix: &str, mihomo_config_path: &str) -> Result<()> {
    let bin_path = mihoro_bin_path()?;
    let existing = read_current_crontab();
    let entry = existing.lines().find(|l| is_mihoro_entry(l, &bin_path));

    match entry {
        Some(line) => {
            println!("{} Auto-update is enabled", "status:".green().bold());
            println!("{} {}", "->".dimmed(), line.dimmed());
        }
        None => {
            println!("{} Auto-update is disabled", "status:".yellow().bold());
            return Ok(());
        }
    }

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
    fn test_is_mihoro_entry_matches_current_path() {
        let bin = "/root/.local/bin/mihoro";
        assert!(is_mihoro_entry(
            "0 */12 * * * /root/.local/bin/mihoro update",
            bin
        ));
    }

    #[test]
    fn test_is_mihoro_entry_matches_legacy_path() {
        let bin = "/usr/local/bin/mihoro";
        // Path differs from current binary but still ends in `/mihoro update`.
        assert!(is_mihoro_entry(
            "0 */6 * * * /opt/mihoro/bin/mihoro update",
            bin
        ));
    }

    #[test]
    fn test_is_mihoro_entry_ignores_comments_and_blank() {
        let bin = "/root/.local/bin/mihoro";
        assert!(!is_mihoro_entry("", bin));
        assert!(!is_mihoro_entry("   ", bin));
        assert!(!is_mihoro_entry("# /root/.local/bin/mihoro update", bin));
    }

    #[test]
    fn test_is_mihoro_entry_does_not_match_unrelated() {
        let bin = "/root/.local/bin/mihoro";
        assert!(!is_mihoro_entry(
            "27 16 * * * /root/.acme.sh/acme.sh --cron --home /root/.acme.sh > /dev/null",
            bin
        ));
        assert!(!is_mihoro_entry("0 3 * * * /usr/bin/mihomo --reload", bin));
    }

    #[test]
    fn test_merge_crontab_preserves_existing() {
        let bin = "/root/.local/bin/mihoro";
        let existing = "\
27 16 * * * /root/.acme.sh/acme.sh --cron > /dev/null
0 3 * * * /opt/backup/run.sh
";
        let new_entry = "0 */12 * * * /root/.local/bin/mihoro update\n";
        let merged = merge_crontab(existing, new_entry, bin);

        assert!(merged.contains("/root/.acme.sh/acme.sh"));
        assert!(merged.contains("/opt/backup/run.sh"));
        assert!(merged.contains("/root/.local/bin/mihoro update"));
        assert!(merged.ends_with('\n'));
    }

    #[test]
    fn test_merge_crontab_replaces_existing_mihoro_entry() {
        let bin = "/root/.local/bin/mihoro";
        let existing = "\
0 */6 * * * /root/.local/bin/mihoro update
27 16 * * * /root/.acme.sh/acme.sh --cron > /dev/null
";
        let new_entry = "0 */12 * * * /root/.local/bin/mihoro update\n";
        let merged = merge_crontab(existing, new_entry, bin);

        assert_eq!(merged.matches("/root/.local/bin/mihoro update").count(), 1);
        assert!(merged.contains("0 */12 * * *"));
        assert!(!merged.contains("0 */6 * * *"));
        assert!(merged.contains("/root/.acme.sh/acme.sh"));
    }

    #[test]
    fn test_strip_mihoro_entries_keeps_others() {
        let bin = "/root/.local/bin/mihoro";
        let existing = "\
0 */12 * * * /root/.local/bin/mihoro update
27 16 * * * /root/.acme.sh/acme.sh --cron > /dev/null
";
        let stripped = strip_mihoro_entries(existing, bin).expect("non-empty");
        assert!(!stripped.contains("/root/.local/bin/mihoro update"));
        assert!(stripped.contains("/root/.acme.sh/acme.sh"));
    }

    #[test]
    fn test_strip_mihoro_entries_returns_none_when_only_mihoro() {
        let bin = "/root/.local/bin/mihoro";
        let existing = "0 */12 * * * /root/.local/bin/mihoro update\n";
        assert!(strip_mihoro_entries(existing, bin).is_none());
    }
}
