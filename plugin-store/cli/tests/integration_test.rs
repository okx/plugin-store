use plugin_store::state::StateManager;
use plugin_store::state::models::{InstalledPlugin, InstalledAgent};
use plugin_store::installer::skill::SkillInstaller;
use tempfile::TempDir;

#[test]
fn test_full_install_uninstall_cycle() {
    let tmp = TempDir::new().unwrap();

    // Simulate skill install
    let skill_dir = tmp.path().join("skills").join("test-plugin");
    SkillInstaller::write_skill(&skill_dir, "# Test Skill").unwrap();
    assert!(skill_dir.join("SKILL.md").exists());

    // Record in state
    let state_path = tmp.path().join("installed.json");
    let mut state_mgr = StateManager::with_path(state_path);
    state_mgr
        .add(InstalledPlugin {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            installed_at: "2026-03-19T10:00:00Z".to_string(),
            agents: vec![InstalledAgent {
                agent: "claude-code".to_string(),
                skill_path: Some(skill_dir.join("SKILL.md").display().to_string()),
                mcp_key: None,
                binary_path: None,
                skill_names: vec![],
                mcp_keys: vec![],
            }],
            components_installed: vec!["skill".to_string()],
        })
        .unwrap();

    // Verify installed
    let state = state_mgr.load().unwrap();
    assert_eq!(state.plugins.len(), 1);
    assert_eq!(state.plugins[0].name, "test-plugin");
    assert_eq!(state.plugins[0].version, "1.0.0");

    // Simulate uninstall
    std::fs::remove_dir_all(&skill_dir).unwrap();
    state_mgr.remove("test-plugin").unwrap();

    // Verify clean
    assert!(!skill_dir.exists());
    let state = state_mgr.load().unwrap();
    assert_eq!(state.plugins.len(), 0);
}
