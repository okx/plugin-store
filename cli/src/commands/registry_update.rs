use anyhow::Result;
use colored::Colorize;
use plugin_store::registry::RegistryManager;

pub async fn execute() -> Result<()> {
    println!("Refreshing registry...");
    let manager = RegistryManager::new();
    let registry = manager.get_registry(true).await?;
    println!(
        "  {} Registry updated. {} plugins available.",
        "OK".green(),
        registry.plugins.len()
    );
    Ok(())
}
