use std::{
    borrow::Cow,
    cmp::min,
    fs::{self, File},
    io::{self, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
    time::Duration,
};

use anyhow::{Context, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use colored::Colorize;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio_retry::{
    strategy::{jitter, ExponentialBackoff},
    Retry,
};
use truncatable::Truncatable;

/// Total number of retries attempted on top of the initial request.
pub const MAX_RETRIES: usize = 3;
pub const DETAIL_PREFIX: &str = "   ";
pub const MIHORO_GITHUB_MIRROR_ENV: &str = "MIHORO_GITHUB_MIRROR";

/// Shared retry strategy for HTTP operations.
///
/// Yields up to [`MAX_RETRIES`] retries (so up to `MAX_RETRIES + 1` total attempts) with
/// exponential backoff of ~1s, ~2s, ~4s, each with jitter and capped at 5s.
/// `ExponentialBackoff::from_millis(2).factor(500)` seeds `current = 2` and multiplies by
/// `base = 2` each step, so the yielded delays are `2 * 500`, `4 * 500`, `8 * 500`, ... ms
/// before jitter.
pub fn retry_strategy() -> impl Iterator<Item = Duration> {
    ExponentialBackoff::from_millis(2)
        .factor(500)
        .max_delay(Duration::from_secs(5))
        .map(jitter)
        .take(MAX_RETRIES)
}

/// Creates the parent directory for a given path if it does not exist.
///
/// # Arguments
///
/// * `path` - A string slice that holds the path for which the parent directory should be created.
pub fn create_parent_dir(path: &Path) -> Result<()> {
    // let parent_dir = Path::new(path)
    let parent_dir = path
        .parent()
        .with_context(|| format!("parent directory of `{}` invalid", path.to_string_lossy()))?;
    if !parent_dir.exists() {
        fs::create_dir_all(parent_dir)?;
    }
    Ok(())
}

fn github_mirror_base() -> Option<String> {
    let mirror = std::env::var(MIHORO_GITHUB_MIRROR_ENV).ok()?;
    let mirror = mirror.trim().trim_end_matches('/').to_string();
    if mirror.is_empty() {
        return None;
    }
    Some(mirror)
}

fn is_github_download_host(host: &str) -> bool {
    host == "github.com" || host.ends_with(".githubusercontent.com")
}

/// Prefix GitHub-hosted download urls with the configured mirror, if any.
///
/// This intentionally excludes `api.github.com` so API metadata requests continue to use
/// GitHub directly while large artifact downloads can still flow through a mirror.
pub fn resolve_download_url(url: &str) -> Cow<'_, str> {
    let Some(mirror) = github_mirror_base() else {
        return Cow::Borrowed(url);
    };

    let Ok(parsed) = reqwest::Url::parse(url) else {
        return Cow::Borrowed(url);
    };

    let Some(host) = parsed.host_str() else {
        return Cow::Borrowed(url);
    };

    if !is_github_download_host(host) {
        return Cow::Borrowed(url);
    }

    if url == mirror || url.starts_with(&format!("{mirror}/")) {
        return Cow::Borrowed(url);
    }

    Cow::Owned(format!("{mirror}/{url}"))
}

/// Download file from url to path with a reusable http client.
///
/// Performs the initial request, then retries up to [`MAX_RETRIES`] more times on any
/// failure (connection, HTTP status, stream, or IO error). Each attempt truncates the
/// destination file.
pub async fn download_file(
    client: &Client,
    url: &str,
    path: &Path,
    user_agent: &str,
) -> Result<()> {
    let mut attempt = 0usize;
    Retry::spawn(retry_strategy(), || {
        // attempt = 0 is the initial request; retries are 1..=MAX_RETRIES.
        let retry_no = attempt;
        attempt += 1;
        async move {
            if retry_no > 0 {
                println!(
                    "{} Retrying download (attempt {}/{})...",
                    DETAIL_PREFIX.yellow(),
                    retry_no,
                    MAX_RETRIES
                );
            }
            download_file_once(client, url, path, user_agent).await
        }
    })
    .await
}

/// Single-shot download with progress bar. Called by [`download_file`] on each retry.
///
/// Renders a progress bar if content-length is available from the url headers provided. If not,
/// renders a spinner to indicate that something is downloading. On failure the bar is cleared so
/// the next retry renders cleanly.
///
/// With reference from:
/// * https://github.com/mihaigalos/tutorials/blob/800d5acbc333fd4068622e9b3d870cb5b7d34e12/rust/download_with_progressbar/src/main.rs
/// * https://github.com/console-rs/indicatif/blob/2954b1a24ac5f1900a7861992e4825bff643c9e2/examples/yarnish.rs
///
/// Note: Allow `clippy::unused_io_amount` because we are writing downloaded chunks on the fly.
#[allow(clippy::unused_io_amount)]
async fn download_file_once(
    client: &Client,
    url: &str,
    path: &Path,
    user_agent: &str,
) -> Result<()> {
    let resolved_url = resolve_download_url(url);

    // Create parent directory for download destination if not exists
    create_parent_dir(path)?;

    // Create shared http client for multiple downloads when possible
    let res = client
        .get(resolved_url.as_ref())
        .header("User-Agent", user_agent)
        .send()
        .await
        .with_context(|| format!("failed to GET from '{}'", resolved_url.as_ref()))?;
    res.error_for_status_ref()?;

    // If content length is not available or 0, use a spinner instead of a progress bar
    let total_size = res.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);

    let bar_style = ProgressStyle::with_template(
        "{prefix:.cyan} Downloading {msg}\n{prefix:.cyan} {elapsed_precise} \
         [{bar:30.white/cyan}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
    )?
    .progress_chars("-> ");
    let spinner_style = ProgressStyle::with_template(
        "{prefix:.cyan} Downloading {wide_msg}\n{prefix:.cyan} \
         {spinner} {elapsed_precise} \u{2014} {bytes_per_sec}",
    )?;

    if total_size == 0 {
        pb.set_style(spinner_style);
    } else {
        pb.set_style(bar_style);
    }
    pb.set_prefix(DETAIL_PREFIX);

    let truncated_url = Truncatable::from(url)
        .truncator("...".into())
        .truncate(64)
        .underline();
    pb.set_message(format!("{truncated_url}"));

    // Perform the streamed write in a scoped async block so we can clean up the progress bar
    // regardless of success or failure.
    let result: Result<()> = async {
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
        Ok(())
    }
    .await;

    match &result {
        Ok(()) => {
            // Clear the progress bar and print a single summary line so the output
            // stays visually aligned inside the stage body output.
            pb.finish_and_clear();
            println!(
                "{} Downloaded to {}",
                DETAIL_PREFIX.cyan(),
                path.to_str().unwrap_or("").underline()
            );
        }
        // Clear the bar before the outer retry loop prints its next message.
        Err(_) => pb.finish_and_clear(),
    }

    result
}

pub fn delete_file(path: &str, prefix: impl std::fmt::Display) -> Result<()> {
    // Delete file if exists
    if Path::new(path).exists() {
        fs::remove_file(path).map(|_| {
            println!("{} Removed {}", prefix, path.underline().yellow());
        })?;
    }
    Ok(())
}

pub fn extract_gzip(from_path: &Path, to_path: &str, prefix: impl std::fmt::Display) -> Result<()> {
    // Create parent directory for extraction dest if not exists
    create_parent_dir(Path::new(to_path))?;

    // Extract gzip file
    let mut archive = GzDecoder::new(File::open(from_path)?);
    let mut file = File::create(to_path)?;
    io::copy(&mut archive, &mut file)?;
    // fs::remove_file(gzip_path)?;
    println!("{} Extracted to {}", prefix, to_path.underline().yellow());
    Ok(())
}

/// Try and decode a base64 encoded file in place.
///
/// Decodes the base64 encoded content of a file in place and writes the decoded content back to the
/// file. If the file does not contain base64 encoded content, maintains the file as is.
///
/// # Arguments
///
/// * `filepath` - Path to the file to decode base64 content in place.
pub fn try_decode_base64_file_inplace(filepath: &str) -> Result<()> {
    // Open the file for reading and writing
    let mut file = File::options().read(true).write(true).open(filepath)?;
    let mut base64_buf = Vec::new();

    // Read the file content into the buffer
    file.read_to_end(&mut base64_buf)?;

    // Try to decode the base64 content
    match BASE64_STANDARD.decode(&base64_buf) {
        Ok(decoded_bytes) => {
            // Truncate the file and seek to the beginning
            file.set_len(0)?;
            file.seek(SeekFrom::Start(0))?;

            // Write the decoded bytes back to the file
            let mut writer = BufWriter::new(&file);
            writer.write_all(&decoded_bytes)?;
        }
        Err(_) => {
            // If decoding fails, do nothing and return Ok
            return Ok(());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::{Mutex, OnceLock},
    };
    use tempfile::tempdir;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_create_parent_dir_creates_directories() -> Result<()> {
        let dir = tempdir()?;
        let nested_path = dir.path().join("nested/dir/file.txt");

        create_parent_dir(&nested_path)?;

        let parent = nested_path.parent().unwrap();
        assert!(parent.exists());
        Ok(())
    }

    #[test]
    fn test_delete_file_removes_existing_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "test content")?;

        delete_file(file_path.to_str().unwrap(), "prefix")?;

        assert!(!file_path.exists());
        Ok(())
    }

    #[test]
    fn test_delete_file_handles_nonexistent_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("nonexistent.txt");

        // Should not error on non-existent file
        delete_file(file_path.to_str().unwrap(), "prefix")?;
        assert!(!file_path.exists());

        Ok(())
    }

    #[test]
    fn test_extract_gzip() -> Result<()> {
        let dir = tempdir()?;
        let gzip_path = dir.path().join("test.gz");
        let output_path = dir.path().join("output.txt");

        // Create a simple gzip file
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let gzip_file = fs::File::create(&gzip_path)?;
        let mut encoder = GzEncoder::new(gzip_file, Compression::default());
        encoder.write_all(b"test content")?;
        encoder.finish()?;

        extract_gzip(&gzip_path, output_path.to_str().unwrap(), "prefix")?;

        let content = fs::read_to_string(&output_path)?;
        assert_eq!(content, "test content");

        Ok(())
    }

    #[test]
    fn test_try_decode_base64_file_inplace_valid_base64() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");

        let encoded = base64::engine::general_purpose::STANDARD.encode("test content");
        fs::write(&file_path, &encoded)?;

        try_decode_base64_file_inplace(file_path.to_str().unwrap())?;

        let decoded = fs::read_to_string(&file_path)?;
        assert_eq!(decoded, "test content");

        Ok(())
    }

    #[test]
    fn test_try_decode_base64_file_inplace_invalid_base64() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");

        fs::write(&file_path, "not valid base64!!!")?;

        // Should not error on invalid base64
        try_decode_base64_file_inplace(file_path.to_str().unwrap())?;

        // File should remain unchanged
        let content = fs::read_to_string(&file_path)?;
        assert_eq!(content, "not valid base64!!!");

        Ok(())
    }

    #[test]
    fn test_resolve_download_url_uses_mirror_for_github_downloads() {
        let _guard = env_lock().lock().unwrap();
        std::env::set_var(MIHORO_GITHUB_MIRROR_ENV, "https://gh-proxy.org/");

        let resolved = resolve_download_url(
            "https://github.com/MetaCubeX/mihomo/releases/latest/download/version.txt",
        );
        assert_eq!(
            resolved.as_ref(),
            "https://gh-proxy.org/https://github.com/MetaCubeX/mihomo/releases/latest/download/version.txt"
        );

        std::env::remove_var(MIHORO_GITHUB_MIRROR_ENV);
    }

    #[test]
    fn test_resolve_download_url_keeps_non_github_urls_and_api_urls() {
        let _guard = env_lock().lock().unwrap();
        std::env::set_var(MIHORO_GITHUB_MIRROR_ENV, "https://gh-proxy.org");

        assert_eq!(
            resolve_download_url("https://example.com/file.tar.gz").as_ref(),
            "https://example.com/file.tar.gz"
        );
        assert_eq!(
            resolve_download_url("https://api.github.com/repos/spencerwooo/mihoro/releases/latest")
                .as_ref(),
            "https://api.github.com/repos/spencerwooo/mihoro/releases/latest"
        );

        std::env::remove_var(MIHORO_GITHUB_MIRROR_ENV);
    }
}
