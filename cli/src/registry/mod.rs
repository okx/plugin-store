pub mod models;
pub mod fetcher;

use anyhow::Result;
use models::{Plugin, Registry};
use fetcher::RegistryFetcher;

pub struct RegistryManager {
    fetcher: RegistryFetcher,
}

impl RegistryManager {
    pub fn new() -> Self {
        Self {
            fetcher: RegistryFetcher::new(),
        }
    }

    pub async fn get_registry(&self, force_refresh: bool) -> Result<Registry> {
        self.fetcher.fetch(force_refresh).await
    }

    pub async fn search(&self, keyword: &str) -> Result<Vec<Plugin>> {
        let registry = self.get_registry(false).await?;
        let kw = keyword.to_lowercase();
        let results: Vec<Plugin> = registry
            .plugins
            .into_iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&kw)
                    || p.description.to_lowercase().contains(&kw)
                    || p.tags.iter().any(|t| t.to_lowercase().contains(&kw))
                    || p.category.to_lowercase().contains(&kw)
            })
            .collect();
        Ok(results)
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<Plugin>> {
        let registry = self.get_registry(false).await?;
        Ok(registry.plugins.into_iter().find(|p| p.name == name))
    }
}
