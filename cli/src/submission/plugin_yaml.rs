use serde::{Deserialize, Serialize};

/// Root structure of a plugin.yaml submission manifest.
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginYaml {
    pub schema_version: u32,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: AuthorInfo,
    pub license: String,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub components: ComponentsDecl,
    /// Plugin source type (free-form string, displayed in `plugin-store list`).
    /// Examples: "official", "dapp-official", "community-developer".
    /// Defaults to "community-developer" in registry if absent.
    #[serde(default, rename = "type")]
    pub plugin_type: Option<String>,
    /// Link URL for the plugin (project homepage, repo, etc.).
    #[serde(default)]
    pub link: Option<String>,
    /// Build configuration for Binary source code compilation.
    /// Any developer can submit source code — our CI compiles it.
    /// Absent = pure Skill plugin (no compilation needed).
    #[serde(default)]
    pub build: Option<BuildConfig>,
    /// External API domains this plugin calls (informational, for reviewer reference).
    /// Lint uses this to distinguish expected vs unexpected external URLs.
    #[serde(default)]
    pub api_calls: Vec<String>,
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
    /// Single skill (backward compat). Use `skills` for multiple.
    #[serde(default)]
    pub skill: Option<SkillDecl>,
    /// Multiple skills — each must have a dir containing SKILL.md.
    /// Preferred over `skill` (singular) when plugin provides multiple skills.
    #[serde(default)]
    pub skills: Vec<SkillDecl>,
    #[serde(default)]
    pub binary: Option<BinaryDecl>,
}

impl ComponentsDecl {
    /// Return all skill declarations (merging singular `skill` + plural `skills`).
    pub fn all_skills(&self) -> Vec<&SkillDecl> {
        let mut result: Vec<&SkillDecl> = self.skills.iter().collect();
        if let Some(ref s) = self.skill {
            if result.is_empty() {
                result.push(s);
            }
        }
        result
    }

    /// Returns true if any skill is declared.
    pub fn has_skill(&self) -> bool {
        self.skill.is_some() || !self.skills.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillDecl {
    /// Directory containing SKILL.md (relative to submission root)
    #[serde(default)]
    pub dir: Option<String>,
    /// Explicit path to a single SKILL.md file
    #[serde(default)]
    pub path: Option<String>,
    /// External repo (for plugins that host skills in their own repo).
    /// When set, SKILL.md + scripts + assets are fetched from this repo.
    /// Compatible with Claude marketplace format repos.
    #[serde(default)]
    pub repo: Option<String>,
    /// Pinned commit SHA for external repo (full 40-char hex).
    /// Required when `repo` points to an external repo.
    #[serde(default)]
    pub commit: Option<String>,
}

impl SkillDecl {
    /// Returns true if this skill references an external repo (not community repo).
    pub fn is_external(&self) -> bool {
        self.repo.is_some() && self.dir.is_none()
    }
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

/// Build configuration — tells our CI how to compile the plugin's source code.
///
/// Source code lives in the developer's own GitHub repo, referenced by
/// `source_repo` + `source_commit`. Our CI clones at the exact commit SHA,
/// compiles, and publishes the artifact. We never store source code in our repo.
#[derive(Debug, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Programming language: rust, go, typescript, node, python
    pub lang: String,
    /// GitHub repo containing source code (e.g. "developer/my-tool").
    /// Optional: when absent, source code is in the submission directory (local build).
    #[serde(default)]
    pub source_repo: String,
    /// Git commit SHA (full 40-char hex) — pinned for integrity.
    /// Required when source_repo is set, ignored for local builds.
    #[serde(default)]
    pub source_commit: String,
    /// Path within the repo to the source root (default: ".")
    #[serde(default = "default_source_dir")]
    pub source_dir: String,
    /// Language-specific entry file (Cargo.toml, go.mod, package.json, pyproject.toml)
    #[serde(default)]
    pub entry: Option<String>,
    /// Name of the compiled binary
    #[serde(default)]
    pub binary_name: Option<String>,
    /// Entry point file for TypeScript/Node.js/Python (e.g. src/index.ts, src/index.js, src/main.py)
    #[serde(default)]
    pub main: Option<String>,
    /// Target platforms to build for (optional, defaults to all supported)
    #[serde(default)]
    pub targets: Vec<String>,
}

fn default_source_dir() -> String {
    ".".to_string()
}

impl PluginYaml {
    pub fn from_str(s: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }

    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let parsed = Self::from_str(&content)?;
        Ok(parsed)
    }

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

/// Valid build languages (all compile to binary via our CI).
pub const VALID_BUILD_LANGS: &[&str] = &["rust", "go", "typescript", "node", "python"];

/// Expected entry files per language.
pub const LANG_ENTRY_FILES: &[(&str, &str)] = &[
    ("rust", "Cargo.toml"),
    ("go", "go.mod"),
    ("typescript", "package.json"),
    ("node", "package.json"),
    ("python", "pyproject.toml"),
];

/// File extensions that must NOT appear in source submissions.
pub const FORBIDDEN_BINARY_EXTENSIONS: &[&str] = &[
    "exe", "dll", "com", "cmd", "bat", "scr", "msi",
    "so", "a", "o", "elf",
    "dylib", "app",
    "lib", "obj", "wasm",
    "class", "jar", "jmod",
    "pyc", "pyd", "pyo",
    "node",
];

/// Valid license identifiers (common SPDX).
pub const VALID_LICENSES: &[&str] = &[
    "MIT", "Apache-2.0", "GPL-2.0", "GPL-3.0",
    "BSD-2-Clause", "BSD-3-Clause", "ISC", "MPL-2.0",
    "LGPL-2.1", "LGPL-3.0", "Unlicense", "CC0-1.0",
];
