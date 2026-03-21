use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct InstalledState {
    pub schema_version: u32,
    pub plugins: Vec<InstalledPlugin>,
}

impl Default for InstalledState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            plugins: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstalledPlugin {
    pub name: String,
    pub version: String,
    pub installed_at: String,
    pub agents: Vec<InstalledAgent>,
    pub components_installed: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstalledAgent {
    pub agent: String,
    pub skill_path: Option<String>,
    pub mcp_key: Option<String>,
    pub binary_path: Option<String>,
    /// All installed skill directory names (for multi-skill plugins like Uniswap)
    #[serde(default)]
    pub skill_names: Vec<String>,
    /// All installed MCP server keys
    #[serde(default)]
    pub mcp_keys: Vec<String>,
}
