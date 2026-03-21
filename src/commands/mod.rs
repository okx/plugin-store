pub mod list;
pub mod search;
pub mod info;
pub mod install;
pub mod uninstall;
pub mod update;
pub mod installed;
pub mod registry_update;
pub mod self_update;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// List all available plugins
    List,
    /// Search plugins by keyword
    Search {
        /// Keyword to search for
        keyword: String,
    },
    /// Show plugin details
    Info {
        /// Plugin name
        name: String,
    },
    /// Install a plugin
    Install {
        /// Plugin name
        name: String,
        /// Install skill component only
        #[arg(long)]
        skill_only: bool,
        /// Install MCP component only
        #[arg(long)]
        mcp_only: bool,
        /// Target agent (skip interactive selection)
        #[arg(long)]
        agent: Option<String>,
        /// Skip confirmation prompts (e.g. community plugin warning)
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Uninstall a plugin
    Uninstall {
        /// Plugin name
        name: String,
        /// Target agent (only remove from specific agent)
        #[arg(long)]
        agent: Option<String>,
    },
    /// Update a plugin or all plugins
    Update {
        /// Plugin name (omit for --all)
        name: Option<String>,
        /// Update all installed plugins
        #[arg(long)]
        all: bool,
    },
    /// Show installed plugins
    Installed,
    /// Update plugin-store itself to the latest version
    SelfUpdate,
    /// Registry management
    Registry {
        #[command(subcommand)]
        command: RegistryCommands,
    },
}

#[derive(Subcommand)]
pub enum RegistryCommands {
    /// Force refresh registry cache
    Update,
}
