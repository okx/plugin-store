mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sky-lending", about = "Sky Lending (MakerDAO CDP) plugin for onchainos")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List collateral types (ilks) with rates and liquidation parameters
    Ilks(commands::ilks::IlksArgs),
    /// List CDP vaults for an address
    Vaults(commands::vaults::VaultsArgs),
    /// Open a new CDP vault (dry-run only — CDP has liquidation risk)
    OpenVault(commands::open_vault::OpenVaultArgs),
    /// Deposit ETH collateral into a vault (dry-run only)
    DepositCollateral(commands::deposit_collateral::DepositCollateralArgs),
    /// Draw (mint) DAI against deposited collateral (dry-run only)
    DrawDai(commands::draw_dai::DrawDaiArgs),
    /// Repay DAI debt (dry-run only)
    RepayDai(commands::repay_dai::RepayDaiArgs),
    /// Withdraw collateral from a vault (dry-run only)
    WithdrawCollateral(commands::withdraw_collateral::WithdrawCollateralArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Ilks(args) => commands::ilks::run(args).await,
        Commands::Vaults(args) => commands::vaults::run(args).await,
        Commands::OpenVault(args) => commands::open_vault::run(args).await,
        Commands::DepositCollateral(args) => commands::deposit_collateral::run(args).await,
        Commands::DrawDai(args) => commands::draw_dai::run(args).await,
        Commands::RepayDai(args) => commands::repay_dai::run(args).await,
        Commands::WithdrawCollateral(args) => commands::withdraw_collateral::run(args).await,
    }
}
