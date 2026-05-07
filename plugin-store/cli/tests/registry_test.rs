use plugin_store::registry::models::Registry;

#[test]
fn test_fetcher_cache_write_and_read() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cache_path = tmp.path().join("registry.json");
    let json = r#"{"schema_version":1,"plugins":[]}"#;

    plugin_store::utils::cache::write_cache(&cache_path, json).unwrap();
    let content = plugin_store::utils::cache::read_cache(&cache_path).unwrap();
    let registry: plugin_store::registry::models::Registry = serde_json::from_str(&content).unwrap();
    assert_eq!(registry.plugins.len(), 0);
}

#[test]
fn test_deserialize_registry() {
    let json = r#"{
        "schema_version": 1,
        "plugins": [
            {
                "name": "test-plugin",
                "version": "1.0.0",
                "description": "A test plugin",
                "author": { "name": "Test", "github": "https://github.com/test" },
                "category": "trading-strategy",
                "tags": ["test"],
                "type": "official",
                "components": {
                    "skill": {
                        "repo": "test/repo",
                        "path": "skills/test/SKILL.md"
                    }
                }
            }
        ]
    }"#;
    let registry: Registry = serde_json::from_str(json).unwrap();
    assert_eq!(registry.schema_version, 1);
    assert_eq!(registry.plugins.len(), 1);
    assert_eq!(registry.plugins[0].name, "test-plugin");
    assert!(registry.plugins[0].components.skill.is_some());
    assert!(registry.plugins[0].components.mcp.is_none());
    assert!(registry.plugins[0].components.binary.is_none());
}

#[test]
fn test_deserialize_full_components() {
    let json = r#"{
        "schema_version": 1,
        "plugins": [
            {
                "name": "full-plugin",
                "version": "1.2.0",
                "description": "Full plugin",
                "author": { "name": "OKX", "github": "https://github.com/okx" },
                "category": "trading-strategy",
                "tags": ["grid"],
                "type": "community",
                "components": {
                    "skill": { "repo": "okx/repo", "path": "skills/grid/SKILL.md" },
                    "mcp": {
                        "type": "npm",
                        "package": "@okx/grid-mcp",
                        "command": "npx -y @okx/grid-mcp",
                        "args": [],
                        "env": ["PRIVATE_KEY"]
                    },
                    "binary": {
                        "repo": "okx/repo",
                        "asset_pattern": "grid-{target}",
                        "checksums_asset": "checksums.txt",
                        "install_dir": "~/.plugin-store/bin/"
                    }
                },
                "extra": {
                    "chains": ["ethereum"],
                    "protocols": ["uniswap-v3"],
                    "risk_level": "medium"
                }
            }
        ]
    }"#;
    let registry: Registry = serde_json::from_str(json).unwrap();
    let plugin = &registry.plugins[0];
    assert!(plugin.components.mcp.is_some());
    assert!(plugin.components.binary.is_some());
    assert_eq!(plugin.source, "community");
    let defi = plugin.extra.as_ref().unwrap();
    assert_eq!(defi.chains, vec!["ethereum"]);
}
