use anyhow::Result;
use colored::Colorize;
use dialoguer::Confirm;
use plugin_store::registry::RegistryManager;
use plugin_store::state::StateManager;

pub async fn execute(name: Option<&str>, all: bool) -> Result<()> {
    let registry_mgr = RegistryManager::new();
    let registry = registry_mgr.get_registry(true).await?;
    let state_mgr = StateManager::new();
    let state = state_mgr.load()?;

    if state.plugins.is_empty() {
        println!("No plugins installed.");
        return Ok(());
    }

    let plugins_to_check: Vec<_> = if all {
        state.plugins.clone()
    } else if let Some(name) = name {
        state
            .plugins
            .iter()
            .filter(|p| p.name == name)
            .cloned()
            .collect()
    } else {
        println!("Specify a plugin name or use --all.");
        return Ok(());
    };

    let mut updates_available = Vec::new();

    for installed in &plugins_to_check {
        if let Some(registry_plugin) = registry.plugins.iter().find(|p| p.name == installed.name) {
            if registry_plugin.version != installed.version {
                updates_available.push((installed.clone(), registry_plugin.clone()));
            } else {
                println!("  {} {} (up to date)", installed.name, installed.version);
            }
        }
    }

    if updates_available.is_empty() {
        println!("All plugins are up to date.");
        return Ok(());
    }

    println!("\nUpdates available:");
    for (installed, latest) in &updates_available {
        println!(
            "  {}: {} -> {}",
            installed.name,
            installed.version.yellow(),
            latest.version.green()
        );
    }

    if all && updates_available.len() > 1 {
        let confirm = Confirm::new()
            .with_prompt(format!("Update {} plugins?", updates_available.len()))
            .default(true)
            .interact()?;
        if !confirm {
            return Ok(());
        }
    }

    for (installed, _) in &updates_available {
        let agents: Vec<String> = installed.agents.iter().map(|a| a.agent.clone()).collect();
        // Uninstall old version first (cleans up renamed/removed skills & MCPs)
        super::uninstall::execute(&installed.name, None).await?;
        // Re-install to same agents
        for agent_id in &agents {
            super::install::execute(&installed.name, false, false, Some(agent_id), true).await?;
        }
    }

    Ok(())
}
