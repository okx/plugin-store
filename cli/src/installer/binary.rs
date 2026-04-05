use anyhow::{bail, Context, Result};
use sha2::{Sha256, Digest};
use std::path::PathBuf;
use std::time::Duration;
use crate::utils::platform::current_target;

pub struct BinaryInstaller;

impl BinaryInstaller {
    pub async fn install(
        repo: &str,
        asset_pattern: &str,
        checksums_asset: Option<&str>,
        install_dir: &str,
        release_tag: Option<&str>,
    ) -> Result<PathBuf> {
        let target = current_target();
        let asset_name = asset_pattern.replace("{target}", &target);

        let install_path = shellexpand_path(install_dir);
        std::fs::create_dir_all(&install_path)?;
        let binary_path = install_path.join(&asset_name);

        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .build()?;

        let release_url = if let Some(tag) = release_tag {
            format!(
                "https://api.github.com/repos/{}/releases/tags/{}",
                repo, tag
            )
        } else {
            format!(
                "https://api.github.com/repos/{}/releases/latest",
                repo
            )
        };

        println!("  Fetching release info...");
        let release: serde_json::Value = client
            .get(&release_url)
            .header("User-Agent", "plugin-store")
            .send()
            .await?
            .error_for_status()
            .context(format!("Failed to fetch release from {}", release_url))?
            .json()
            .await?;

        let assets = release["assets"].as_array().context("No assets in release")?;

        let binary_asset = assets
            .iter()
            .find(|a| a["name"].as_str() == Some(&asset_name))
            .context(format!("Asset {} not found in release", asset_name))?;
        let download_url = binary_asset["browser_download_url"]
            .as_str()
            .context("No download URL")?;

        let size_bytes = binary_asset["size"].as_u64().unwrap_or(0);
        let size_display = if size_bytes > 1_048_576 {
            format!("{:.1} MB", size_bytes as f64 / 1_048_576.0)
        } else if size_bytes > 0 {
            format!("{:.0} KB", size_bytes as f64 / 1024.0)
        } else {
            "unknown size".to_string()
        };
        println!("  Downloading {} ({})...", asset_name, size_display);

        let binary_bytes = client
            .get(download_url)
            .header("User-Agent", "plugin-store")
            .send()
            .await?
            .error_for_status()
            .context(format!("Failed to download {}", asset_name))?
            .bytes()
            .await?;

        if let Some(checksums_name) = checksums_asset {
            let checksums_file = assets
                .iter()
                .find(|a| a["name"].as_str() == Some(checksums_name));
            if let Some(cs) = checksums_file {
                let cs_url = cs["browser_download_url"].as_str().context("No checksum URL")?;
                let cs_content = client
                    .get(cs_url)
                    .header("User-Agent", "plugin-store")
                    .send()
                    .await?
                    .text()
                    .await?;
                let expected = crate::utils::find_checksum(&cs_content, &asset_name);
                if let Some(expected_hash) = expected {
                    let mut hasher = Sha256::new();
                    hasher.update(&binary_bytes);
                    let actual_hash = hex::encode(hasher.finalize());
                    if actual_hash != expected_hash {
                        bail!("Checksum verification failed. Expected: {}, Got: {}", expected_hash, actual_hash);
                    }
                    println!("  Checksum verified ✓");
                }
            }
        }

        std::fs::write(&binary_path, &binary_bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary_path, std::fs::Permissions::from_mode(0o755))?;
        }

        // Create a canonical symlink/copy without the platform suffix so the binary
        // is accessible by its base name (e.g. "signal-tracker").
        let canonical_name = asset_pattern.replace("-{target}", "").replace("{target}", "");
        let returned_path = if !canonical_name.is_empty() && canonical_name != asset_name {
            let canonical_path = install_path.join(&canonical_name);
            #[cfg(unix)]
            {
                if canonical_path.exists() {
                    std::fs::remove_file(&canonical_path)?;
                }
                std::os::unix::fs::symlink(&binary_path, &canonical_path)?;
            }
            #[cfg(not(unix))]
            {
                std::fs::copy(&binary_path, &canonical_path)?;
            }
            canonical_path
        } else {
            binary_path
        };

        Ok(returned_path)
    }

}

fn shellexpand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}
