mod commands;

use clap::Parser;
use commands::{Commands, RegistryCommands};

#[derive(Parser)]
#[command(name = "plugin-store", version, about = "A plugin marketplace for Skills and MCP servers")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => commands::list::execute().await,
        Commands::Search { keyword } => commands::search::execute(&keyword).await,
        Commands::Info { name } => commands::info::execute(&name).await,
        Commands::Install {
            name,
            skill_only,
            mcp_only,
            agent,
            yes,
        } => commands::install::execute(&name, skill_only, mcp_only, agent.as_deref(), yes).await,
        Commands::Uninstall { name, agent } => {
            commands::uninstall::execute(&name, agent.as_deref()).await
        }
        Commands::Update { name, all } => commands::update::execute(name.as_deref(), all).await,
        Commands::Installed => commands::installed::execute().await,
        Commands::SelfUpdate => commands::self_update::execute().await,
        Commands::Registry { command } => match command {
            RegistryCommands::Update => commands::registry_update::execute().await,
        },
    }
}
