use anyhow::Result;
use colored::Colorize;
use plugin_store::registry::RegistryManager;

pub async fn execute(name: &str) -> Result<()> {
    let manager = RegistryManager::new();
    let plugin = manager.find_by_name(name).await?;

    match plugin {
        None => {
            eprintln!(
                "Plugin '{}' not found. Run `plugin-store search <keyword>` to find plugins.",
                name
            );
        }
        Some(p) => {
            println!("{}: {}", "Name".bold(), p.name);
            println!("{}: {}", "Version".bold(), p.version);
            println!("{}: {}", "Description".bold(), p.description);
            if let Some(ref link) = p.link {
                println!("{}: {} ({})", "Author".bold(), p.author.name, link);
            } else {
                println!("{}: {}", "Author".bold(), p.author.name);
            }
            println!("{}: {}", "Category".bold(), p.category);
            println!("{}: {}", "Source".bold(), p.source);
            println!("{}: {}", "Tags".bold(), p.tags.join(", "));

            println!("\n{}:", "Components".bold());
            if p.components.skill.is_some() {
                println!("  {} Skill", "✔".green());
            }
            if let Some(ref mcp) = p.components.mcp {
                println!("  {} MCP (type: {})", "✔".green(), mcp.mcp_type);
            }
            if p.components.binary.is_some() {
                println!("  {} Binary", "✔".green());
            }

            if let Some(ref defi) = p.extra {
                println!("\n{}:", "DeFi Info".bold());
                println!("  Chains: {}", defi.chains.join(", "));
                println!("  Protocols: {}", defi.protocols.join(", "));
                println!("  Risk Level: {}", defi.risk_level);
            }
        }
    }
    Ok(())
}
