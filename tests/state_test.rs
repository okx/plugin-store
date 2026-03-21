use plugin_store::state::models::{InstalledPlugin, InstalledAgent};
use plugin_store::state::StateManager;
use tempfile::TempDir;

#[test]
fn test_state_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let state_path = tmp.path().join("installed.json");
    let mut manager = StateManager::with_path(state_path.clone());

    let state = manager.load().unwrap();
    assert_eq!(state.plugins.len(), 0);

    let plugin = InstalledPlugin {
        name: "test-plugin".to_string(),
        version: "1.0.0".to_string(),
        installed_at: "2026-03-19T10:00:00Z".to_string(),
        agents: vec![InstalledAgent {
            agent: "claude-code".to_string(),
            skill_path: Some("~/.claude/skills/test-plugin/SKILL.md".to_string()),
            mcp_key: None,
            binary_path: None,
            skill_names: vec![],
            mcp_keys: vec![],
        }],
        components_installed: vec!["skill".to_string()],
    };
    manager.add(plugin).unwrap();

    let state = manager.load().unwrap();
    assert_eq!(state.plugins.len(), 1);
    assert_eq!(state.plugins[0].name, "test-plugin");
}

#[test]
fn test_state_remove() {
    let tmp = TempDir::new().unwrap();
    let state_path = tmp.path().join("installed.json");
    let mut manager = StateManager::with_path(state_path);

    let plugin = InstalledPlugin {
        name: "to-remove".to_string(),
        version: "1.0.0".to_string(),
        installed_at: "2026-03-19T10:00:00Z".to_string(),
        agents: vec![],
        components_installed: vec![],
    };
    manager.add(plugin).unwrap();
    assert_eq!(manager.load().unwrap().plugins.len(), 1);

    manager.remove("to-remove").unwrap();
    assert_eq!(manager.load().unwrap().plugins.len(), 0);
}
