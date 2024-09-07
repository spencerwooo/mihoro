use std::{
    cmp::min,
    fs::{self, File},
    io::{self, Write},
    path::Path,
};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use anyhow::{anyhow, Context, Result};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
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
        fs::remove_file(path).map(|_| {
            println!("{} Removed {}", prefix.cyan(), path.underline().yellow());
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
//try to decode a base64 file in place, the file must exist,if the file is not base64 encoded ,it is ok
pub fn decode_base64(filename: &str) -> Result<()> {
    // copy file to buffer
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(filename)?;
    let mut base64_buf = Vec::<u8>::new();
    file.read_to_end(&mut base64_buf)?;
    //try decode
    let decoded = BASE64_STANDARD.decode(base64_buf.as_slice());
    // the file is not base64 encoded .It is ok .Do Nothing
    if let Err(_) = decoded {
        return Ok(());
    }
    //try to clear file
    if let Err(e) = file.set_len(0) {
        return Err(anyhow!("fail to clear file,why? {}",e));
    }
    if let Err(e) = file.seek(SeekFrom::Start(0)) {
        return Err(anyhow!("fail to clear file,why? {}",e));
    }
    //write bytes to file
    let decoded_bytes = decoded.expect("this can't be happening");
    file.write_all(&decoded_bytes)?;
    Ok(())
}