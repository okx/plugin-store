pub mod claude_code;
pub mod cursor;
pub mod openclaw;

use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentKind {
    ClaudeCode,
    Cursor,
    OpenClaw,
}

impl AgentKind {
    pub fn name(&self) -> &str {
        match self {
            AgentKind::ClaudeCode => "Claude Code",
            AgentKind::Cursor => "Cursor",
            AgentKind::OpenClaw => "OpenClaw",
        }
    }

    pub fn id(&self) -> &str {
        match self {
            AgentKind::ClaudeCode => "claude-code",
            AgentKind::Cursor => "cursor",
            AgentKind::OpenClaw => "openclaw",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "claude-code" => Some(AgentKind::ClaudeCode),
            "cursor" => Some(AgentKind::Cursor),
            "openclaw" => Some(AgentKind::OpenClaw),
            _ => None,
        }
    }
}

pub struct DetectedAgent {
    pub kind: AgentKind,
    pub found: bool,
    pub path_hint: String,
}

pub trait AgentAdapter {
    fn detect(&self) -> DetectedAgent;
    fn skill_dir(&self, plugin_name: &str) -> PathBuf;
    fn install_mcp_config(&self, name: &str, command: &str, args: &[String], env: &[String]) -> Result<()>;
    fn remove_mcp_config(&self, name: &str) -> Result<()>;
    fn remove_skill(&self, plugin_name: &str) -> Result<()>;
}

pub fn detect_agents() -> Vec<DetectedAgent> {
    let adapters: Vec<Box<dyn AgentAdapter>> = vec![
        Box::new(claude_code::ClaudeCodeAdapter::new()),
        Box::new(cursor::CursorAdapter::new()),
        Box::new(openclaw::OpenClawAdapter::new()),
    ];
    adapters.iter().map(|a| a.detect()).collect()
}

pub fn get_adapter(kind: &AgentKind) -> Box<dyn AgentAdapter> {
    match kind {
        AgentKind::ClaudeCode => Box::new(claude_code::ClaudeCodeAdapter::new()),
        AgentKind::Cursor => Box::new(cursor::CursorAdapter::new()),
        AgentKind::OpenClaw => Box::new(openclaw::OpenClawAdapter::new()),
    }
}
