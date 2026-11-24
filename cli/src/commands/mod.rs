pub mod list;
pub mod search;
pub mod info;
pub mod install;
pub mod uninstall;
pub mod update;
pub mod installed;
pub mod registry_update;
pub mod self_update;
pub mod init;
pub mod lint;
pub mod import;

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
    /// Scaffold a new plugin submission
    Init {
        /// Plugin name (lowercase, hyphens, 2-40 chars)
        name: String,
    },
    /// Validate a plugin submission before submitting
    Lint {
        /// Path to the plugin submission directory
        path: String,
    },
    /// Import a Claude marketplace-compatible repo as a plugin submission
    Import {
        /// GitHub repo (owner/repo format)
        repo: String,
        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum RegistryCommands {
    /// Force refresh registry cache
    Update,
}
