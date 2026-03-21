use anyhow::Result;
use colored::Colorize;
use plugin_store::agent::{get_adapter, AgentKind};
use plugin_store::installer::mcp::McpInstaller;
use plugin_store::state::StateManager;
use plugin_store::utils::ui;

pub async fn execute(name: &str, agent_filter: Option<&str>) -> Result<()> {
    let mut state_mgr = StateManager::new();
    let plugin = state_mgr
        .find(name)?
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' is not installed.", name))?;

    println!("Uninstalling {}...", plugin.name.bold());

    for agent_record in &plugin.agents {
        if let Some(filter) = agent_filter {
            if agent_record.agent != filter {
                continue;
            }
        }

        let kind = AgentKind::from_id(&agent_record.agent);
        if let Some(kind) = kind {
            let adapter = get_adapter(&kind);

            // Remove all discovered skills (multi-skill plugins)
            if !agent_record.skill_names.is_empty() {
                for skill_name in &agent_record.skill_names {
                    adapter.remove_skill(skill_name)?;
                }
                ui::print_success(&format!(
                    "{} skills removed from {}",
                    agent_record.skill_names.len(),
                    kind.name()
                ));
            } else if agent_record.skill_path.is_some() {
                // Legacy single-skill fallback
                adapter.remove_skill(&plugin.name)?;
                ui::print_success(&format!("Skill removed from {}", kind.name()));
            }

            // Remove all discovered MCP servers
            if !agent_record.mcp_keys.is_empty() {
                for mcp_key in &agent_record.mcp_keys {
                    McpInstaller::uninstall(&kind, mcp_key)?;
                }
                ui::print_success(&format!(
                    "{} MCP entries removed from {}",
                    agent_record.mcp_keys.len(),
                    kind.name()
                ));
            } else if let Some(ref mcp_key) = agent_record.mcp_key {
                // Legacy single-mcp fallback
                McpInstaller::uninstall(&kind, mcp_key)?;
                ui::print_success(&format!("MCP entry removed from {}", kind.name()));
            }

            if let Some(ref binary_path) = agent_record.binary_path {
                let path = std::path::Path::new(binary_path);
                if path.exists() {
                    std::fs::remove_file(path)?;
                    ui::print_success(&format!("Binary removed <- {}", binary_path));
                }
            }
        }
    }

    if agent_filter.is_some() {
        let mut state = state_mgr.load()?;
        if let Some(p) = state.plugins.iter_mut().find(|p| p.name == name) {
            p.agents.retain(|a| Some(a.agent.as_str()) != agent_filter);
            if p.agents.is_empty() {
                state.plugins.retain(|p| p.name != name);
            }
        }
        state_mgr.save(&state)?;
    } else {
        state_mgr.remove(name)?;
    }

    ui::print_success("State updated");
    println!("{}", "Done!".green().bold());
    Ok(())
}
