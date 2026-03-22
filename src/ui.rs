use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use flate2::read::GzDecoder;
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tar::Archive;
use tempfile::{tempdir_in, NamedTempFile};

use crate::utils::{create_parent_dir, download_file};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Ui {
    #[default]
    Metacubexd,
    Zashboard,
    YacdMeta,
    Custom(String),
}

impl Ui {
    pub fn parse(raw: &str) -> Result<Self> {
        let value = raw.trim();
        if value.is_empty() {
            bail!("ui must not be empty");
        }

        match value {
            "metacubexd" => Ok(Self::Metacubexd),
            "zashboard" => Ok(Self::Zashboard),
            "yacd-meta" => Ok(Self::YacdMeta),
            _ => {
                let Some(url) = value.strip_prefix("custom:") else {
                    bail!("unsupported ui `{value}`");
                };
                if url.is_empty() {
                    bail!("custom ui download url must not be empty");
                }
                Ok(Self::Custom(url.to_string()))
            }
        }
    }

    pub fn as_config_value(&self) -> &str {
        match self {
            Self::Metacubexd => "metacubexd",
            Self::Zashboard => "zashboard",
            Self::YacdMeta => "yacd-meta",
            Self::Custom(url) => url.as_str(),
        }
    }

    pub fn download_url(&self) -> &str {
        match self {
            Self::Metacubexd => {
                "https://github.com/MetaCubeX/metacubexd/archive/refs/heads/gh-pages.tar.gz"
            }
            Self::Zashboard => {
                "https://github.com/Zephyruso/zashboard/archive/refs/heads/gh-pages.tar.gz"
            }
            Self::YacdMeta => {
                "https://github.com/MetaCubeX/Yacd-meta/archive/refs/heads/gh-pages.tar.gz"
            }
            Self::Custom(url) => url.as_str(),
        }
    }
}

impl Serialize for Ui {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match self {
            Self::Custom(url) => format!("custom:{url}"),
            _ => self.as_config_value().to_string(),
        };
        serializer.serialize_str(&value)
    }
}

impl<'de> Deserialize<'de> for Ui {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ui::parse(&value).map_err(serde::de::Error::custom)
    }
}

pub fn default_ui() -> Option<Ui> {
    Some(Ui::default())
}

pub fn resolve_external_ui_path(config_root: &str, external_ui: &str) -> PathBuf {
    let path = Path::new(external_ui);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(config_root).join(path)
    }
}

pub async fn install_ui(
    client: &Client,
    ui: &Ui,
    target_dir: &Path,
    user_agent: &str,
    prefix: &str,
) -> Result<()> {
    let archive_file = NamedTempFile::new()?;
    download_file(client, ui.download_url(), archive_file.path(), user_agent).await?;

    create_parent_dir(target_dir)?;
    let target_parent = target_dir
        .parent()
        .with_context(|| format!("parent directory of `{}` invalid", target_dir.display()))?;
    let extract_dir = tempdir_in(target_parent)?;
    extract_tar_gz(archive_file.path(), extract_dir.path())?;

    let extracted_root = find_archive_root(extract_dir.path())?;
    replace_dir(&extracted_root, target_dir)?;

    println!(
        "{} Installed UI `{}` to {}",
        prefix,
        ui.as_config_value(),
        target_dir.display()
    );
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, extract_dir: &Path) -> Result<()> {
    let archive = fs::File::open(archive_path)?;
    let decoder = GzDecoder::new(archive);
    let mut archive = Archive::new(decoder);
    archive.unpack(extract_dir)?;
    Ok(())
}

fn find_archive_root(extract_dir: &Path) -> Result<PathBuf> {
    let mut entries = fs::read_dir(extract_dir)?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();

    if entries.len() != 1 {
        bail!(
            "expected one root entry in extracted ui archive, found {}",
            entries.len()
        );
    }

    let root = entries.remove(0);
    if !root.is_dir() {
        bail!("expected extracted ui archive root to be a directory");
    }
    Ok(root)
}

fn replace_dir(source_dir: &Path, target_dir: &Path) -> Result<()> {
    create_parent_dir(target_dir)?;

    let parent = target_dir
        .parent()
        .with_context(|| format!("parent directory of `{}` invalid", target_dir.display()))?;

    let staged_dir = parent.join(format!(
        ".{}.tmp",
        target_dir
            .file_name()
            .ok_or_else(|| anyhow!("invalid ui target directory"))?
            .to_string_lossy()
    ));
    let backup_dir = parent.join(format!(
        ".{}.bak",
        target_dir
            .file_name()
            .ok_or_else(|| anyhow!("invalid ui target directory"))?
            .to_string_lossy()
    ));

    if staged_dir.exists() {
        fs::remove_dir_all(&staged_dir)?;
    }
    if backup_dir.exists() {
        fs::remove_dir_all(&backup_dir)?;
    }

    fs::rename(source_dir, &staged_dir)?;

    if target_dir.exists() {
        fs::rename(target_dir, &backup_dir)?;
    }

    if let Err(err) = fs::rename(&staged_dir, target_dir) {
        if backup_dir.exists() {
            let _ = fs::rename(&backup_dir, target_dir);
        }
        return Err(err).with_context(|| {
            format!(
                "failed to move extracted ui into `{}`",
                target_dir.display()
            )
        });
    }

    if backup_dir.exists() {
        fs::remove_dir_all(backup_dir)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct UiConfig {
        ui: Ui,
    }

    #[test]
    fn test_builtin_ui_parse() -> Result<()> {
        assert_eq!(Ui::parse("metacubexd")?, Ui::Metacubexd);
        assert_eq!(Ui::parse("zashboard")?, Ui::Zashboard);
        assert_eq!(Ui::parse("yacd-meta")?, Ui::YacdMeta);
        Ok(())
    }

    #[test]
    fn test_custom_ui_parse() -> Result<()> {
        assert_eq!(
            Ui::parse("custom:https://example.com/ui.tar.gz")?,
            Ui::Custom("https://example.com/ui.tar.gz".to_string())
        );
        Ok(())
    }

    #[test]
    fn test_ui_download_url() {
        assert_eq!(
            Ui::Metacubexd.download_url(),
            "https://github.com/MetaCubeX/metacubexd/archive/refs/heads/gh-pages.tar.gz"
        );
        assert_eq!(
            Ui::Zashboard.download_url(),
            "https://github.com/Zephyruso/zashboard/archive/refs/heads/gh-pages.tar.gz"
        );
        assert_eq!(
            Ui::YacdMeta.download_url(),
            "https://github.com/MetaCubeX/Yacd-meta/archive/refs/heads/gh-pages.tar.gz"
        );
    }

    #[test]
    fn test_resolve_external_ui_path() {
        assert_eq!(
            resolve_external_ui_path("/tmp/mihomo", "ui"),
            PathBuf::from("/tmp/mihomo/ui")
        );
        assert_eq!(
            resolve_external_ui_path("/tmp/mihomo", "/var/www/ui"),
            PathBuf::from("/var/www/ui")
        );
    }

    #[test]
    fn test_ui_serde_roundtrip() -> Result<()> {
        let encoded = r#"ui = "custom:https://example.com/ui.tar.gz""#;
        let decoded: UiConfig = toml::from_str(encoded)?;
        assert_eq!(
            decoded,
            UiConfig {
                ui: Ui::Custom("https://example.com/ui.tar.gz".to_string())
            }
        );
        assert_eq!(toml::to_string(&decoded)?.trim(), encoded);
        Ok(())
    }
}
