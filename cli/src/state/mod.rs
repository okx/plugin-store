pub mod models;

use anyhow::Result;
use std::path::PathBuf;
use models::{InstalledState, InstalledPlugin};

pub struct StateManager {
    path: PathBuf,
}

impl StateManager {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Cannot determine home directory");
        Self {
            path: home.join(".plugin-store").join("installed.json"),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<InstalledState> {
        if !self.path.exists() {
            return Ok(InstalledState::default());
        }
        let content = std::fs::read_to_string(&self.path)?;
        let state: InstalledState = serde_json::from_str(&content)?;
        Ok(state)
    }

    pub fn save(&self, state: &InstalledState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn add(&mut self, plugin: InstalledPlugin) -> Result<()> {
        let mut state = self.load()?;
        state.plugins.retain(|p| p.name != plugin.name);
        state.plugins.push(plugin);
        self.save(&state)
    }

    pub fn remove(&mut self, name: &str) -> Result<()> {
        let mut state = self.load()?;
        state.plugins.retain(|p| p.name != name);
        self.save(&state)
    }

    pub fn find(&self, name: &str) -> Result<Option<InstalledPlugin>> {
        let state = self.load()?;
        Ok(state.plugins.into_iter().find(|p| p.name == name))
    }
}
