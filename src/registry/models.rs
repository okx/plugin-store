use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Registry {
    pub schema_version: u32,
    #[serde(default)]
    pub stats_url: Option<String>,
    pub plugins: Vec<Plugin>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plugin {
    pub name: String,
    /// Optional display name shown in `list`. Falls back to `name` when absent.
    #[serde(default)]
    pub alias: Option<String>,
    pub version: String,
    pub description: String,
    pub author: Author,
    #[serde(default)]
    pub link: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    #[serde(rename = "type")]
    pub source: String,
    pub components: Components,
    pub extra: Option<DefiInfo>,
}

impl Plugin {
    /// Display name: alias if set, otherwise name.
    pub fn display_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Author {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Components {
    pub skill: Option<SkillComponent>,
    pub mcp: Option<McpComponent>,
    pub binary: Option<BinaryComponent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillComponent {
    pub repo: String,
    /// Explicit SKILL.md path (legacy single-file mode). When omitted, auto-discover from repo tree.
    pub path: Option<String>,
    /// Directory path containing SKILL.md and its siblings (e.g. references/).
    /// When set, all files under this directory are installed, preserving structure.
    pub dir: Option<String>,
    /// Pinned commit SHA for community plugins. When set, all downloads use this ref instead of
    /// "main", preventing silent content changes after registry approval.
    #[serde(default)]
    pub commit: Option<String>,
}

/// A discovered skill directory in a repo (SKILL.md + optional references/)
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    /// Skill name derived from parent directory (e.g. "swap-integration")
    pub name: String,
    /// All file paths relative to repo root
    pub files: Vec<String>,
}

/// A discovered MCP server config from .mcp.json in the repo
#[derive(Debug, Clone)]
pub struct DiscoveredMcp {
    /// MCP server name (key in mcpServers object)
    pub name: String,
    /// Command to run
    pub command: String,
    /// Arguments
    pub args: Vec<String>,
    /// Environment variable names
    pub env: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpComponent {
    #[serde(rename = "type")]
    pub mcp_type: String,
    pub package: Option<String>,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BinaryComponent {
    pub repo: String,
    pub asset_pattern: String,
    pub checksums_asset: Option<String>,
    pub install_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DefiInfo {
    #[serde(default)]
    pub chains: Vec<String>,
    #[serde(default)]
    pub protocols: Vec<String>,
    #[serde(default)]
    pub risk_level: String,
}
