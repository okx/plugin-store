pub const GITHUB_OWNER: &str = "okx";
pub const CLI_REPO: &str = "plugin-store";
pub const REGISTRY_REPO: &str = "plugin-store";
pub const COMMUNITY_REPO: &str = "okx/plugin-store";

/// Full GitHub `owner/repo` path for the registry (and CLI binary) repo.
pub fn registry_repo() -> String {
    format!("{}/{}", GITHUB_OWNER, REGISTRY_REPO)
}

/// Base URL of the stats API.
/// Override with env var PLUGIN_STORE_STATS_URL.
/// GET  {url}/counts  → {"plugin-name": 123, ...}
/// POST {url}/install → {"name": "...", "version": "..."}
pub fn stats_url() -> Option<String> {
    std::env::var("PLUGIN_STORE_STATS_URL").ok()
}
