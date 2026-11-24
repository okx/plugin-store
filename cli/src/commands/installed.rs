use anyhow::Result;
use colored::Colorize;
use plugin_store::state::StateManager;

pub async fn execute() -> Result<()> {
    let state_mgr = StateManager::new();
    let state = state_mgr.load()?;

    if state.plugins.is_empty() {
        println!("No plugins installed.");
        return Ok(());
    }

    println!(
        "{:<35} {:<10} {:<20} {}",
        "Name".bold(),
        "Version".bold(),
        "Agents".bold(),
        "Components".bold()
    );
    println!("{}", "-".repeat(85));

    for plugin in &state.plugins {
        let agents: Vec<String> = plugin.agents.iter().map(|a| a.agent.clone()).collect();
        println!(
            "{:<35} {:<10} {:<20} {}",
            plugin.name,
            plugin.version,
            agents.join(", "),
            plugin.components_installed.join(", ")
        );
    }

    println!("\n{} plugins installed.", state.plugins.len());
    Ok(())
}
