mod api;
mod auth;
mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "fourmeme-plugin",
    version,
    about = "Trade Four.meme bonding-curve memecoins on BNB Chain — buy/sell pre-graduate tokens via TokenManager V2"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check wallet status and routing (no auth required)
    Quickstart(commands::quickstart::QuickstartArgs),

    /// Sign in to four.meme via wallet signature; saves token for create-token
    Login(commands::login::LoginArgs),

    /// Get full on-chain state for a Four.meme token (read-only)
    GetToken(commands::get_token::GetTokenArgs),

    /// Preview a buy: tokens out, fee, msg.value (read-only)
    QuoteBuy(commands::quote_buy::QuoteBuyArgs),

    /// Preview a sell: BNB out, fee (read-only)
    QuoteSell(commands::quote_sell::QuoteSellArgs),

    /// Show wallet's balance for one or more Four.meme tokens
    Positions(commands::positions::PositionsArgs),

    /// Buy a Four.meme token (BNB-quoted; ERC-20 quotes deferred to v0.2)
    Buy(commands::buy::BuyArgs),

    /// Sell a Four.meme token back to BNB
    Sell(commands::sell::SellArgs),

    /// Launch a new Four.meme memecoin (requires four.meme login cookie)
    CreateToken(commands::create_token::CreateTokenArgs),

    /// Get four.meme public sys/config
    Config(commands::config::ConfigArgs),

    /// Discover Four.meme tokens via ranking or keyword search (no auth)
    ListTokens(commands::list_tokens::ListTokensArgs),

    /// Read TaxToken fee/dispatch config (only for tokens with creatorType=5)
    TaxInfo(commands::tax_info::TaxInfoArgs),

    /// Count of ERC-8004 agent identity NFTs owned by a wallet
    AgentBalance(commands::agent_balance::AgentBalanceArgs),

    /// Send BNB or ERC-20 token to another wallet
    Send(commands::send::SendArgs),

    /// Register as an Agent (mint ERC-8004 identity NFT)
    AgentRegister(commands::agent_register::AgentRegisterArgs),

    /// Fetch TokenManager V2 events (TokenCreate / Purchase / Sale / LiquidityAdded)
    Events(commands::events::EventsArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quickstart(a) => commands::quickstart::run(a).await,
        Commands::Login(a)      => commands::login::run(a).await,
        Commands::GetToken(a)   => commands::get_token::run(a).await,
        Commands::QuoteBuy(a)   => commands::quote_buy::run(a).await,
        Commands::QuoteSell(a)  => commands::quote_sell::run(a).await,
        Commands::Positions(a)  => commands::positions::run(a).await,
        Commands::Buy(a)        => commands::buy::run(a).await,
        Commands::Sell(a)       => commands::sell::run(a).await,
        Commands::CreateToken(a) => commands::create_token::run(a).await,
        Commands::Config(a)      => commands::config::run(a).await,
        Commands::ListTokens(a)  => commands::list_tokens::run(a).await,
        Commands::TaxInfo(a)     => commands::tax_info::run(a).await,
        Commands::AgentBalance(a)=> commands::agent_balance::run(a).await,
        Commands::Send(a)        => commands::send::run(a).await,
        Commands::AgentRegister(a) => commands::agent_register::run(a).await,
        Commands::Events(a)      => commands::events::run(a).await,
    }
}
