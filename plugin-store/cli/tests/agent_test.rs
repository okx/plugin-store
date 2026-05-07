use plugin_store::agent::{AgentKind, detect_agents};

#[test]
fn test_agent_kind_display() {
    assert_eq!(AgentKind::ClaudeCode.name(), "Claude Code");
    assert_eq!(AgentKind::Cursor.name(), "Cursor");
    assert_eq!(AgentKind::OpenClaw.name(), "OpenClaw");
}

#[test]
fn test_detect_agents_returns_list() {
    let agents = detect_agents();
    assert!(agents.len() <= 3);
}
