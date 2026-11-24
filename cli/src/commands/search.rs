use anyhow::Result;
use colored::Colorize;
use plugin_store::registry::RegistryManager;

pub async fn execute(keyword: &str) -> Result<()> {
    let manager = RegistryManager::new();
    let results = manager.search(keyword).await?;

    if results.is_empty() {
        println!("No plugins found matching '{}'.", keyword);
        return Ok(());
    }

    println!(
        "{:<32} {:<10} {:<15} {}",
        "Name".bold(),
        "Version".bold(),
        "Source".bold(),
        "Description".bold()
    );
    println!("{}", "-".repeat(90));

    for plugin in &results {
        println!(
            "{:<32} {:<10} {:<15} {}",
            plugin.name, plugin.version, plugin.source, plugin.description
        );
    }

    println!("\n{} plugins found.", results.len());
    Ok(())
}
