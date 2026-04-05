use anyhow::Result;
use std::path::PathBuf;
use super::{AgentAdapter, AgentKind, DetectedAgent};

pub struct ClaudeCodeAdapter {
    base_dir: PathBuf,
    /// MCP config lives at ~/.claude.json (NOT ~/.claude/settings.json)
    mcp_config_path: PathBuf,
}

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Cannot determine home directory");
        Self {
            base_dir: home.join(".claude"),
            mcp_config_path: home.join(".claude.json"),
        }
    }
}

impl AgentAdapter for ClaudeCodeAdapter {
    fn detect(&self) -> DetectedAgent {
        DetectedAgent {
            kind: AgentKind::ClaudeCode,
            found: self.base_dir.exists(),
            path_hint: self.base_dir.display().to_string(),
        }
    }

    fn skill_dir(&self, plugin_name: &str) -> PathBuf {
        self.base_dir.join("skills").join(plugin_name)
    }

    fn install_mcp_config(&self, name: &str, command: &str, args: &[String], env: &[String]) -> Result<()> {
        let config_path = self.mcp_config_path.clone();
        let mut config: serde_json::Value = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            serde_json::from_str(&content)?
        } else {
            serde_json::json!({})
        };

        let env_obj: serde_json::Map<String, serde_json::Value> = env
            .iter()
            .map(|e| (e.clone(), serde_json::Value::String(format!("${{{}}}", e))))
            .collect();

        let mcp_entry = serde_json::json!({
            "command": command.split_whitespace().next().unwrap_or(command),
            "args": command.split_whitespace().skip(1).chain(args.iter().map(|s| s.as_str())).collect::<Vec<_>>(),
            "env": env_obj,
        });

        config
            .as_object_mut()
            .unwrap()
            .entry("mcpServers")
            .or_insert(serde_json::json!({}))
            .as_object_mut()
            .unwrap()
            .insert(name.to_string(), mcp_entry);

        let json = serde_json::to_string_pretty(&config)?;
        std::fs::write(&config_path, json)?;
        Ok(())
    }

    fn remove_mcp_config(&self, name: &str) -> Result<()> {
        let config_path = self.mcp_config_path.clone();
        if !config_path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&config_path)?;
        let mut config: serde_json::Value = serde_json::from_str(&content)?;
        if let Some(servers) = config.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
            servers.remove(name);
        }
        let json = serde_json::to_string_pretty(&config)?;
        std::fs::write(&config_path, json)?;
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
