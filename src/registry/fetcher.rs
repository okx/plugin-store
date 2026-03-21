use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;

use crate::config;
use crate::registry::models::Registry;
use crate::utils::cache;

const CACHE_TTL: Duration = Duration::from_secs(12 * 60 * 60);

pub struct RegistryFetcher {
    registry_url: String,
    cache_path: PathBuf,
}

impl RegistryFetcher {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Cannot determine home directory");
        Self {
            registry_url: format!(
                "https://raw.githubusercontent.com/{}/{}/main/registry.json",
                config::GITHUB_OWNER,
                config::REGISTRY_REPO
            ),
            cache_path: home.join(".plugin-store").join("cache").join("registry.json"),
        }
    }

    pub async fn fetch(&self, force_refresh: bool) -> Result<Registry> {
        if !force_refresh && cache::is_fresh(&self.cache_path, CACHE_TTL) {
            if let Ok(content) = cache::read_cache(&self.cache_path) {
                if let Ok(mut registry) = serde_json::from_str::<Registry>(&content) {
                    self.expand_self_repo(&mut registry);
                    return Ok(registry);
                }
            }
        }

        match self.fetch_remote().await {
            Ok(mut registry) => {
                if let Ok(json) = serde_json::to_string_pretty(&registry) {
                    let _ = cache::write_cache(&self.cache_path, &json);
                }
                self.expand_self_repo(&mut registry);
                Ok(registry)
            }
            Err(e) => {
                if self.cache_path.exists() {
                    eprintln!("Warning: Using cached registry. Fetch failed: {}", e);
                    let content = cache::read_cache(&self.cache_path)?;
                    let mut registry = serde_json::from_str(&content)?;
                    self.expand_self_repo(&mut registry);
                    Ok(registry)
                } else {
                    Err(e).context("Cannot fetch registry. Check your network and try again.")
                }
            }
        }
    }

    /// Expand `{self}` placeholder in component repo fields to the configured repo.
    fn expand_self_repo(&self, registry: &mut Registry) {
        let self_repo = format!("{}/{}", config::GITHUB_OWNER, config::CLI_REPO);
        for plugin in &mut registry.plugins {
            if let Some(ref mut skill) = plugin.components.skill {
                skill.repo = skill.repo.replace("{self}", &self_repo);
            }
            if let Some(ref mut binary) = plugin.components.binary {
                binary.repo = binary.repo.replace("{self}", &self_repo);
            }
        }
    }

    async fn fetch_remote(&self) -> Result<Registry> {
        let client = reqwest::Client::new();
        let resp = client
            .get(&self.registry_url)
            .header("User-Agent", "plugin-store")
            .send()
            .await?
            .error_for_status()?;
        let registry: Registry = resp.json().await?;
        Ok(registry)
    }
}
