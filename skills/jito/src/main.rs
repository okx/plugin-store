mod config;
mod onchainos;
mod rpc;
mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "jito", about = "Jito MEV-enhanced liquid staking on Solana")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Query current SOL ↔ JitoSOL exchange rate and approximate APY
    Rates(commands::rates::RatesArgs),
    /// Query your JitoSOL balance and SOL equivalent value
    Positions(commands::positions::PositionsArgs),
    /// Stake SOL to receive JitoSOL (MEV-enhanced liquid staking)
    Stake(commands::stake::StakeArgs),
    /// Unstake JitoSOL back to SOL (creates stake account, unlocks after current epoch ~2-3 days)
    Unstake(commands::unstake::UnstakeArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Rates(args) => commands::rates::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Stake(args) => commands::stake::run(args).await,
        Commands::Unstake(args) => commands::unstake::run(args).await,
    };
    match result {
        Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap()),
        Err(e) => {
            let err = serde_json::json!({"ok": false, "error": e.to_string()});
            println!("{}", serde_json::to_string_pretty(&err).unwrap());
            std::process::exit(1);
        }
    }
}
