use crate::config::{Config, MihomoChannel};
use crate::utils::{retry_strategy, DETAIL_PREFIX, MAX_RETRIES};

use anyhow::{bail, Context, Result};
use colored::Colorize;
use reqwest::Client;
use tokio_retry::Retry;

const STABLE_VERSION_URL: &str =
    "https://github.com/MetaCubeX/mihomo/releases/latest/download/version.txt";
const ALPHA_VERSION_URL: &str =
    "https://github.com/MetaCubeX/mihomo/releases/download/Prerelease-Alpha/version.txt";

/// Fetches the latest Mihomo version from GitHub based on the release channel.
///
/// Retries up to 3 attempts total with exponential backoff on any failure.
pub async fn fetch_latest_version(
    client: &Client,
    channel: &MihomoChannel,
    user_agent: &str,
) -> Result<String> {
    let url = match channel {
        MihomoChannel::Stable => STABLE_VERSION_URL,
        MihomoChannel::Alpha => ALPHA_VERSION_URL,
    };

    let mut attempt = 0usize;
    Retry::spawn(retry_strategy(), || {
        let retry_no = attempt;
        attempt += 1;
        async move {
            if retry_no > 0 {
                println!(
                    "{} Retrying version fetch (attempt {}/{})...",
                    DETAIL_PREFIX.yellow(),
                    retry_no,
                    MAX_RETRIES
                );
            }
            fetch_latest_version_once(client, url, user_agent).await
        }
    })
    .await
}

async fn fetch_latest_version_once(client: &Client, url: &str, user_agent: &str) -> Result<String> {
    let response = client
        .get(url)
        .header("User-Agent", user_agent)
        .send()
        .await
        .with_context(|| format!("failed to fetch version from '{}'", url))?;

    response.error_for_status_ref()?;

    let version = response
        .text()
        .await
        .with_context(|| "failed to read version response")?
        .trim()
        .to_string();

    if version.is_empty() {
        bail!("received empty version from GitHub");
    }

    Ok(version)
}

/// Detects the current system architecture and maps it to Mihomo's asset naming convention.
///
/// Maps Rust's std::env::consts::ARCH to Mihomo's default variant for each architecture.
/// For more specific variants (e.g., amd64-v3, armv5), use the --arch flag or mihomo_arch config.
///
/// Supported Mihomo architectures:
/// - x86: 386, 386-go120, 386-go123, 386-softfloat
/// - x86_64: amd64, amd64-compatible, amd64-v1, amd64-v2, amd64-v3 (with go120/go123 variants)
/// - ARM: arm64, armv5, armv6, armv7
/// - MIPS: mips-hardfloat, mips-softfloat, mips64, mips64le, mipsle-hardfloat, mipsle-softfloat
/// - Others: loong64-abi1, loong64-abi2, ppc64le, riscv64, s390x
pub fn detect_arch() -> Result<String> {
    let arch = std::env::consts::ARCH;
    match arch {
        // x86_64: Default to amd64-compatible for maximum compatibility
        "x86_64" => Ok("amd64-compatible".to_string()),
        // ARM 64-bit
        "aarch64" => Ok("arm64".to_string()),
        // ARM 32-bit: Default to armv7 (most common)
        "arm" => Ok("armv7".to_string()),
        // x86 32-bit
        "x86" => Ok("386".to_string()),
        // MIPS 64-bit little-endian
        "mips64" => Ok("mips64".to_string()),
        // MIPS 64-bit little-endian
        "mips64el" => Ok("mips64le".to_string()),
        // MIPS 32-bit
        "mips" => Ok("mips-softfloat".to_string()),
        // MIPS 32-bit little-endian
        "mipsel" => Ok("mipsle-softfloat".to_string()),
        // PowerPC 64-bit little-endian
        "powerpc64le" | "ppc64le" => Ok("ppc64le".to_string()),
        // RISC-V 64-bit
        "riscv64" => Ok("riscv64".to_string()),
        // s390x (IBM Z)
        "s390x" => Ok("s390x".to_string()),
        // LoongArch 64-bit
        "loongarch64" => Ok("loong64-abi2".to_string()),
        _ => bail!(
            "unsupported architecture: {} (use --arch to specify manually)",
            arch
        ),
    }
}

/// List of all supported Mihomo architectures.
const SUPPORTED_ARCHS: &[&str] = &[
    "386",
    "386-go120",
    "386-go123",
    "386-softfloat",
    "amd64",
    "amd64-compatible",
    "amd64-v1",
    "amd64-v1-go120",
    "amd64-v1-go123",
    "amd64-v2",
    "amd64-v2-go120",
    "amd64-v2-go123",
    "amd64-v3",
    "amd64-v3-go120",
    "amd64-v3-go123",
    "arm64",
    "armv5",
    "armv6",
    "armv7",
    "loong64-abi1",
    "loong64-abi2",
    "mips-hardfloat",
    "mips-softfloat",
    "mips64",
    "mips64le",
    "mipsle-hardfloat",
    "mipsle-softfloat",
    "ppc64le",
    "riscv64",
    "s390x",
];

/// Validates that the architecture is supported by Mihomo.
///
/// Returns the architecture if valid, or an error with suggestions if invalid.
pub fn validate_arch(arch: &str) -> Result<String> {
    if SUPPORTED_ARCHS.contains(&arch) {
        return Ok(arch.to_string());
    }

    // Find similar architectures for helpful error message
    let suggestions: Vec<&str> = SUPPORTED_ARCHS
        .iter()
        .filter(|a| a.starts_with(&arch[..arch.len().min(3)]))
        .copied()
        .collect();

    if suggestions.is_empty() {
        bail!(
            "unsupported architecture: '{}'\nSupported: {}",
            arch,
            SUPPORTED_ARCHS.join(", ")
        );
    } else {
        bail!(
            "unsupported architecture: '{}'\nDid you mean: {}",
            arch,
            suggestions.join(", ")
        );
    }
}

/// Constructs the download URL for a specific Mihomo version and architecture.
pub fn build_download_url(version: &str, arch: &str, channel: &MihomoChannel) -> String {
    let base = match channel {
        MihomoChannel::Stable => "https://github.com/MetaCubeX/mihomo/releases/latest/download",
        MihomoChannel::Alpha => {
            "https://github.com/MetaCubeX/mihomo/releases/download/Prerelease-Alpha"
        }
    };
    format!("{}/mihomo-linux-{}-{}.gz", base, arch, version)
}

/// Resolves the Mihomo binary download URL.
///
/// If `remote_mihomo_binary_url` is set in the config, returns it directly.
/// Otherwise, auto-detects the architecture and fetches the latest version from GitHub.
pub async fn resolve_binary_url(
    client: &Client,
    config: &Config,
    arch_override: Option<&str>,
    prefix: &str,
) -> Result<String> {
    // If a URL is explicitly configured, use it directly
    if let Some(ref url) = config.remote_mihomo_binary_url {
        if !url.is_empty() {
            println!(
                "{} Using configured binary URL: {}",
                prefix.cyan(),
                url.underline()
            );
            return Ok(url.clone());
        }
    }

    // Determine architecture: CLI override > config override > auto-detect
    let arch = if let Some(arch) = arch_override {
        validate_arch(arch)?
    } else if let Some(ref arch) = config.mihomo_arch {
        validate_arch(arch)?
    } else {
        detect_arch()?
    };

    let channel = &config.mihomo_channel;
    let channel_name = match channel {
        MihomoChannel::Stable => "stable",
        MihomoChannel::Alpha => "alpha",
    };

    println!(
        "{} Fetching latest mihomo {} release for {}...",
        prefix.cyan(),
        channel_name.bold(),
        format!("linux-{}", arch).bold()
    );

    let version = fetch_latest_version(client, channel, &config.mihoro_user_agent).await?;

    println!(
        "{} Found mihomo version: {}",
        prefix.green(),
        version.bold()
    );

    let url = build_download_url(&version, &arch, channel);
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_arch_returns_valid_value() {
        // This test verifies that detect_arch() returns a valid architecture on the current system
        let result = detect_arch();
        assert!(result.is_ok());
        let arch = result.unwrap();
        // Updated to include amd64-compatible as the new default for x86_64
        assert!(SUPPORTED_ARCHS.contains(&arch.as_str()));
    }

    #[test]
    fn test_build_download_url_stable() {
        let url = build_download_url("v1.19.0", "amd64", &MihomoChannel::Stable);
        assert_eq!(
			url,
			"https://github.com/MetaCubeX/mihomo/releases/latest/download/mihomo-linux-amd64-v1.19.0.gz"
		);
    }

    #[test]
    fn test_build_download_url_alpha() {
        let url = build_download_url("alpha-abc123", "arm64", &MihomoChannel::Alpha);
        assert_eq!(
			url,
			"https://github.com/MetaCubeX/mihomo/releases/download/Prerelease-Alpha/mihomo-linux-arm64-alpha-abc123.gz"
		);
    }

    #[test]
    fn test_build_download_url_compatible_arch() {
        let url = build_download_url("v1.19.0", "amd64-compatible", &MihomoChannel::Stable);
        assert_eq!(
			url,
			"https://github.com/MetaCubeX/mihomo/releases/latest/download/mihomo-linux-amd64-compatible-v1.19.0.gz"
		);
    }

    #[test]
    fn test_validate_arch_accepts_valid_archs() {
        assert!(validate_arch("amd64").is_ok());
        assert!(validate_arch("amd64-compatible").is_ok());
        assert!(validate_arch("amd64-v3").is_ok());
        assert!(validate_arch("arm64").is_ok());
        assert!(validate_arch("armv7").is_ok());
        assert!(validate_arch("riscv64").is_ok());
        assert!(validate_arch("loong64-abi2").is_ok());
    }

    #[test]
    fn test_validate_arch_rejects_invalid_archs() {
        assert!(validate_arch("invalid").is_err());
        assert!(validate_arch("x86_64").is_err());
        assert!(validate_arch("aarch64").is_err());
    }

    #[test]
    fn test_validate_arch_provides_suggestions() {
        let result = validate_arch("amd");
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Did you mean"));
        assert!(error.contains("amd64"));
    }
}
