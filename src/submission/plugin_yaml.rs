use serde::{Deserialize, Serialize};

/// Root structure of a plugin.yaml submission manifest.
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginYaml {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub alias: Option<String>,
    pub version: String,
    pub description: String,
    pub author: AuthorInfo,
    pub license: String,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub components: ComponentsDecl,
    /// Build configuration for MCP/Binary source code compilation.
    /// Only available to Verified Third Party and OKX Official plugins.
    /// Absent = pure Skill plugin (no compilation needed).
    #[serde(default)]
    pub build: Option<BuildConfig>,
    #[serde(default)]
    pub permissions: Option<PermissionsDecl>,
    #[serde(default)]
    pub extra: Option<ExtraDecl>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorInfo {
    pub name: String,
    pub github: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentsDecl {
    #[serde(default)]
    pub skill: Option<SkillDecl>,
    #[serde(default)]
    pub mcp: Option<McpDecl>,
    #[serde(default)]
    pub binary: Option<BinaryDecl>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillDecl {
    /// Directory containing SKILL.md (relative to submission root)
    #[serde(default)]
    pub dir: Option<String>,
    /// Explicit path to a single SKILL.md file
    #[serde(default)]
    pub path: Option<String>,
    /// External repo (for dapp-official plugins that host their own skills)
    #[serde(default)]
    pub repo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpDecl {
    #[serde(rename = "type")]
    pub mcp_type: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub package: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BinaryDecl {
    pub repo: String,
    pub asset_pattern: String,
    #[serde(default)]
    pub checksums_asset: Option<String>,
    #[serde(default)]
    pub install_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionsDecl {
    #[serde(default)]
    pub wallet: Option<WalletPerms>,
    #[serde(default)]
    pub network: Option<NetworkPerms>,
    #[serde(default)]
    pub chains: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletPerms {
    #[serde(default)]
    pub read_balance: bool,
    #[serde(default)]
    pub send_transaction: bool,
    #[serde(default)]
    pub sign_message: bool,
    #[serde(default)]
    pub contract_call: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkPerms {
    #[serde(default)]
    pub api_calls: Vec<String>,
    #[serde(default)]
    pub onchainos_commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtraDecl {
    #[serde(default)]
    pub protocols: Vec<String>,
    #[serde(default)]
    pub risk_level: Option<String>,
}

/// Build configuration — tells our CI how to compile the plugin's source code.
#[derive(Debug, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Programming language: rust, go, typescript, node, python
    pub lang: String,
    /// Path to source code directory (relative to submission root)
    pub source_dir: String,
    /// Language-specific entry file (Cargo.toml, go.mod, package.json, pyproject.toml)
    #[serde(default)]
    pub entry: Option<String>,
    /// Name of the compiled binary
    #[serde(default)]
    pub binary_name: Option<String>,
    /// Entry point file for TypeScript/Python (e.g. src/index.ts, src/main.py)
    #[serde(default)]
    pub main: Option<String>,
    /// npm scope for Node.js packages (e.g. @plugin-store)
    #[serde(default)]
    pub npm_scope: Option<String>,
    /// Target platforms to build for (optional, defaults to all supported)
    #[serde(default)]
    pub targets: Vec<String>,
}

impl PluginYaml {
    /// Parse a plugin.yaml from a string.
    pub fn from_str(s: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }

    /// Parse a plugin.yaml from a file path.
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let parsed = Self::from_str(&content)?;
        Ok(parsed)
    }

    /// Returns true if this plugin requires source code compilation.
    pub fn has_build(&self) -> bool {
        self.build.is_some()
    }
}

/// Valid categories for plugins.
pub const VALID_CATEGORIES: &[&str] = &[
    "trading-strategy",
    "defi-protocol",
    "analytics",
    "utility",
    "security",
    "wallet",
    "nft",
];

/// Valid risk levels.
pub const VALID_RISK_LEVELS: &[&str] = &["low", "medium", "high"];

/// Valid MCP types.
pub const VALID_MCP_TYPES: &[&str] = &["node", "python", "binary"];

/// Valid build languages.
pub const VALID_BUILD_LANGS: &[&str] = &["rust", "go", "typescript", "node", "python"];

/// Expected entry files per language.
pub const LANG_ENTRY_FILES: &[(&str, &str)] = &[
    ("rust", "Cargo.toml"),
    ("go", "go.mod"),
    ("typescript", "package.json"),
    ("node", "package.json"),
    ("python", "pyproject.toml"),
];

/// File extensions that must NOT appear in source submissions
/// (indicates pre-compiled binaries, which we don't accept).
pub const FORBIDDEN_BINARY_EXTENSIONS: &[&str] = &[
    // Windows
    "exe", "dll", "com", "cmd", "bat", "scr", "msi",
    // Linux/Unix
    "so", "a", "o", "elf",
    // macOS
    "dylib", "app",
    // Cross-platform
    "lib", "obj", "wasm",
    // Java
    "class", "jar", "jmod",
    // Python
    "pyc", "pyd", "pyo",
    // Node native addons
    "node",
];

/// Valid license identifiers (common SPDX).
pub const VALID_LICENSES: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "GPL-2.0",
    "GPL-3.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "MPL-2.0",
    "LGPL-2.1",
    "LGPL-3.0",
    "Unlicense",
    "CC0-1.0",
];
