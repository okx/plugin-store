use anyhow::Result;
use crate::agent::{AgentKind, get_adapter};

pub struct McpInstaller;

impl McpInstaller {
    pub fn install(
        agent: &AgentKind,
        name: &str,
        command: &str,
        args: &[String],
        env: &[String],
    ) -> Result<()> {
        let adapter = get_adapter(agent);
        adapter.install_mcp_config(name, command, args, env)
    }

    pub fn uninstall(agent: &AgentKind, name: &str) -> Result<()> {
        let adapter = get_adapter(agent);
        adapter.remove_mcp_config(name)
    }
}
