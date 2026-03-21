use anyhow::Result;
use std::path::PathBuf;
use super::{AgentAdapter, AgentKind, DetectedAgent};

pub struct OpenClawAdapter {
    base_dir: PathBuf,
}

impl OpenClawAdapter {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Cannot determine home directory");
        Self {
            base_dir: home.join(".openclaw"),
        }
    }
}

impl AgentAdapter for OpenClawAdapter {
    fn detect(&self) -> DetectedAgent {
        let found = self.base_dir.exists()
            || std::process::Command::new("which")
                .arg("openclaw")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
        DetectedAgent {
            kind: AgentKind::OpenClaw,
            found,
            path_hint: self.base_dir.display().to_string(),
        }
    }

    fn skill_dir(&self, plugin_name: &str) -> PathBuf {
        self.base_dir.join("skills").join(plugin_name)
    }

    fn install_mcp_config(&self, _name: &str, _command: &str, _args: &[String], _env: &[String]) -> Result<()> {
        Ok(())
    }

    fn remove_mcp_config(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    fn remove_skill(&self, plugin_name: &str) -> Result<()> {
        let dir = self.skill_dir(plugin_name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }
}
