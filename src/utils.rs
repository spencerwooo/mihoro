use std::{
    cmp::min,
    fs::{self, File},
    io::{self, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};

use anyhow::{Context, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use colored::Colorize;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use truncatable::Truncatable;

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
pub async fn download_file(
    client: &Client,
    url: &str,
    path: &Path,
    user_agent: &str,
) -> Result<()> {
    // Create parent directory for download destination if not exists
    create_parent_dir(path)?;

    // Create shared http client for multiple downloads when possible
    let res = client
        .get(url)
        .header("User-Agent", user_agent)
        .send()
        .await
        .with_context(|| format!("failed to GET from '{}'", &url))?;
    res.error_for_status_ref()?;

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

    pb.finish_with_message(format!(
        "Downloaded to {}",
        path.to_str().unwrap().underline()
    ));
    Ok(())
}

pub fn delete_file(path: &str, prefix: &str) -> Result<()> {
    // Delete file if exists
    if Path::new(path).exists() {
        fs::remove_file(path).map(|_| {
            println!("{} Removed {}", prefix.cyan(), path.underline().yellow());
        })?;
    }
    Ok(())
}

pub fn extract_gzip(from_path: &Path, to_path: &str, prefix: &str) -> Result<()> {
    // Create parent directory for extraction dest if not exists
    create_parent_dir(Path::new(to_path))?;

    // Extract gzip file
    let mut archive = GzDecoder::new(File::open(from_path)?);
    let mut file = File::create(to_path)?;
    io::copy(&mut archive, &mut file)?;
    // fs::remove_file(gzip_path)?;
    println!(
        "{} Extracted to {}",
        prefix.green(),
        to_path.underline().yellow()
    );
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
