use anyhow::{bail, Context, Result};
use sha2::{Sha256, Digest};
use std::path::PathBuf;
use crate::utils::platform::current_target;

pub struct BinaryInstaller;

impl BinaryInstaller {
    pub async fn install(
        repo: &str,
        asset_pattern: &str,
        checksums_asset: Option<&str>,
        install_dir: &str,
    ) -> Result<PathBuf> {
        let target = current_target();
        let asset_name = asset_pattern.replace("{target}", &target);

        let install_path = shellexpand_path(install_dir);
        std::fs::create_dir_all(&install_path)?;
        let binary_path = install_path.join(&asset_name);

        let client = reqwest::Client::new();
        let release_url = format!(
            "https://api.github.com/repos/{}/releases/latest",
            repo
        );
        let release: serde_json::Value = client
            .get(&release_url)
            .header("User-Agent", "plugin-store")
            .send()
            .await?
            .error_for_status()?
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

        let binary_bytes = client
            .get(download_url)
            .header("User-Agent", "plugin-store")
            .send()
            .await?
            .error_for_status()?
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
        // is accessible by its base name (e.g. "strategy-ranking-sniper").
        let canonical_name = asset_pattern.replace("-{target}", "").replace("{target}", "");
        if !canonical_name.is_empty() && canonical_name != asset_name {
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
        }

        Ok(binary_path)
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
