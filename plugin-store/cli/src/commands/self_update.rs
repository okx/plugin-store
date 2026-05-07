use anyhow::{bail, Context, Result};
use colored::Colorize;
use sha2::{Digest, Sha256};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn execute() -> Result<()> {
    println!("Checking for updates...");
    println!("  Current version: {}", CURRENT_VERSION);

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    // Fetch latest release from GitHub
    let repo = format!("{}/{}", plugin_store::config::GITHUB_OWNER, plugin_store::config::CLI_REPO);
    let release_url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let resp = client
        .get(&release_url)
        .header("User-Agent", "plugin-store")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await;

    let release: serde_json::Value = match resp {
        Ok(r) => match r.error_for_status() {
            Ok(r) => r.json().await?,
            Err(_) => {
                println!("  No releases found on GitHub. You're on the latest build.");
                return Ok(());
            }
        },
        Err(e) => {
            println!("  Cannot check for updates: {}", e);
            return Ok(());
        }
    };

    let latest_tag = release["tag_name"]
        .as_str()
        .context("No tag_name in release")?;
    let latest_version = latest_tag.trim_start_matches('v');

    println!("  Latest version:  {}", latest_version);

    if latest_version == CURRENT_VERSION {
        println!("\n{}", "Already up to date!".green());
        return Ok(());
    }

    println!(
        "\n  Update available: {} → {}",
        CURRENT_VERSION.yellow(),
        latest_version.green()
    );

    // Find binary asset for current platform
    let target = plugin_store::utils::platform::current_target();
    let asset_name = format!("plugin-store-{}", target);

    let assets = release["assets"]
        .as_array()
        .context("No assets in release")?;

    let binary_asset = assets
        .iter()
        .find(|a| {
            a["name"]
                .as_str()
                .map(|n| n == asset_name)
                .unwrap_or(false)
        })
        .context(format!(
            "No binary found for platform '{}'. Available assets: {}",
            target,
            assets
                .iter()
                .filter_map(|a| a["name"].as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))?;

    let download_url = binary_asset["browser_download_url"]
        .as_str()
        .context("No download URL")?;
    let download_name = binary_asset["name"]
        .as_str()
        .unwrap_or(&asset_name);

    println!("  Downloading {}...", download_name);

    let binary_bytes = client
        .get(download_url)
        .header("User-Agent", "plugin-store")
        .send()
        .await?
        .error_for_status()
        .context("Failed to download binary")?
        .bytes()
        .await?;

    // Verify checksum if checksums.txt is present in the release
    let checksums_asset = assets.iter().find(|a| a["name"].as_str() == Some("checksums.txt"));
    if let Some(cs) = checksums_asset {
        let cs_url = cs["browser_download_url"].as_str().context("No checksum URL")?;
        let cs_content = client
            .get(cs_url)
            .header("User-Agent", "plugin-store")
            .send()
            .await?
            .text()
            .await?;
        if let Some(expected) = plugin_store::utils::find_checksum(&cs_content, &asset_name) {
            let mut hasher = Sha256::new();
            hasher.update(&binary_bytes);
            let actual = hex::encode(hasher.finalize());
            if actual != expected {
                bail!("Checksum verification failed.\n  Expected: {}\n  Got:      {}", expected, actual);
            }
            println!("  Checksum verified ✓");
        }
    }

    // Determine current binary path
    let current_exe = std::env::current_exe().context("Cannot determine current executable path")?;
    let current_exe = current_exe
        .canonicalize()
        .unwrap_or(current_exe);

    // Write to a temp file first, then replace
    let tmp_path = current_exe.with_extension("tmp");
    std::fs::write(&tmp_path, &binary_bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))?;
    }

    // Atomic replace: rename old → .bak, rename new → current
    let bak_path = current_exe.with_extension("bak");
    let _ = std::fs::remove_file(&bak_path); // ignore if no .bak exists
    std::fs::rename(&current_exe, &bak_path)
        .context("Failed to backup current binary")?;
    match std::fs::rename(&tmp_path, &current_exe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&bak_path); // clean up backup
        }
        Err(e) => {
            // Rollback
            let _ = std::fs::rename(&bak_path, &current_exe);
            return Err(e).context("Failed to replace binary, rolled back");
        }
    }

    println!(
        "\n{}  {} → {}",
        "Updated!".green().bold(),
        CURRENT_VERSION,
        latest_version
    );
    Ok(())
}

