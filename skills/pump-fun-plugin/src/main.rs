mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

use commands::{
    buy::BuyArgs, get_price::GetPriceArgs, get_token_info::GetTokenInfoArgs, sell::SellArgs,
};

#[derive(Parser, Debug)]
#[command(
    name = "pump-fun",
    about = "Plugin for pump.fun — buy and sell tokens on Solana bonding curves via onchainos swap",
    version
)]
struct Cli {
    /// Simulate without broadcasting (no on-chain transaction sent)
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Get on-chain bonding curve info for a token
    GetTokenInfo(GetTokenInfoArgs),

    /// Get current buy or sell price for a token
    GetPrice(GetPriceArgs),

    /// Buy tokens on a pump.fun bonding curve via onchainos swap
    Buy(BuyArgs),

    /// Sell tokens back to a pump.fun bonding curve via onchainos swap
    Sell(SellArgs),

    /// Check wallet state and get guided next steps
    Quickstart,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::GetTokenInfo(args) => commands::get_token_info::execute(args).await,
        Commands::GetPrice(args) => commands::get_price::execute(args).await,
        Commands::Buy(args) => commands::buy::execute(args, cli.dry_run).await,
        Commands::Sell(args) => commands::sell::execute(args, cli.dry_run).await,
        Commands::Quickstart => commands::quickstart::run().await,
    };

    if let Err(e) = result {
        eprintln!("{}", serde_json::json!({"ok": false, "error": e.to_string()}));
        std::process::exit(1);
    }
}
