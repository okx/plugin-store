mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;
mod signing;

use clap::{Parser, Subcommand};
use commands::{
    address::AddressArgs,
    cancel::CancelArgs,
    cancel_batch::CancelBatchArgs,
    close::CloseArgs,
    deposit::DepositArgs,
    dex_list::DexListArgs,
    dex_transfer::DexTransferArgs,
    markets::MarketsArgs,
    evm_send::EvmSendArgs,
    get_gas::GetGasArgs,
    order::OrderArgs,
    order_batch::OrderBatchArgs,
    orders::OrdersArgs,
    positions::PositionsArgs,
    prices::PricesArgs,
    register::RegisterArgs,
    spot_balances::SpotBalancesArgs,
    spot_cancel::SpotCancelArgs,
    spot_order::SpotOrderArgs,
    spot_prices::SpotPricesArgs,
    tpsl::TpslArgs,
    transfer::TransferArgs,
    withdraw::WithdrawArgs,
    quickstart::QuickstartArgs,
};

#[derive(Parser)]
#[command(
    name = "hyperliquid",
    version,
    about = "Hyperliquid DEX plugin — trade perps, deposit from Arbitrum, manage gas on HyperEVM"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show open perpetual positions, unrealized PnL, and margin summary
    Positions(PositionsArgs),
    /// List open orders (limit, TP/SL); optionally filter by coin
    Orders(OrdersArgs),
    /// Get current mid prices for all perp markets or a specific coin
    Prices(PricesArgs),
    /// Place a market or limit perp order; optionally attach TP/SL bracket (requires --confirm)
    Order(OrderArgs),
    /// Market-close an open perp position in one command (requires --confirm)
    Close(CloseArgs),
    /// Set stop-loss and/or take-profit on an existing perp position (requires --confirm)
    Tpsl(TpslArgs),
    /// Cancel an open perp order by order ID (requires --confirm)
    Cancel(CancelArgs),
    /// Place multiple perp orders in a single signed request (requires --confirm)
    #[command(name = "order-batch")]
    OrderBatch(OrderBatchArgs),
    /// Cancel multiple perp orders in a single signed request (requires --confirm)
    #[command(name = "cancel-batch")]
    CancelBatch(CancelBatchArgs),
    /// Deposit USDC to Hyperliquid perp account via Arbitrum bridge (minimum $5)
    Deposit(DepositArgs),
    /// Detect your onchainos signing address on Hyperliquid and show setup instructions
    Register(RegisterArgs),
    /// Show wallet address with QR code for easy transfers (HyperEVM by default)
    Address(AddressArgs),
    /// Swap Arbitrum USDC to HYPE on HyperEVM via relay.link (for gas bootstrap)
    GetGas(GetGasArgs),
    /// Send USDC from HyperCore perp account to a HyperEVM address via CoreWriter
    EvmSend(EvmSendArgs),
    /// Transfer USDC between perp and spot accounts (requires --confirm)
    Transfer(TransferArgs),
    /// Withdraw USDC from Hyperliquid perp account to Arbitrum (requires --confirm)
    Withdraw(WithdrawArgs),
    /// Show spot token balances (HYPE, PURR, USDC, etc.)
    SpotBalances(SpotBalancesArgs),
    /// Get current prices for spot markets or a specific token
    SpotPrices(SpotPricesArgs),
    /// Place a market or limit spot order (requires --confirm)
    SpotOrder(SpotOrderArgs),
    /// Cancel an open spot order by order ID or cancel all for a token (requires --confirm)
    SpotCancel(SpotCancelArgs),
    /// Check wallet assets and get a recommended next step for Hyperliquid
    Quickstart(QuickstartArgs),
    /// HIP-3: List all perp DEXs (default + builder DEXs like xyz/flx/vntl) with per-DEX user balance + asset count
    #[command(name = "dex-list")]
    DexList(DexListArgs),
    /// HIP-3: Move USDC between perp DEXs (default <-> builder DEX). Required to fund RWA trading on builder DEXs (requires --confirm)
    #[command(name = "dex-transfer")]
    DexTransfer(DexTransferArgs),
    /// List tradeable markets across Hyperliquid (--type crypto|tradfi|hip3|spot, --dex, --coin, filters)
    Markets(MarketsArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Orders(args) => commands::orders::run(args).await,
        Commands::Prices(args) => commands::prices::run(args).await,
        Commands::Order(args) => commands::order::run(args).await,
        Commands::Close(args) => commands::close::run(args).await,
        Commands::Tpsl(args) => commands::tpsl::run(args).await,
        Commands::Cancel(args) => commands::cancel::run(args).await,
        Commands::OrderBatch(args) => commands::order_batch::run(args).await,
        Commands::CancelBatch(args) => commands::cancel_batch::run(args).await,
        Commands::Deposit(args) => commands::deposit::run(args).await,
        Commands::Register(args) => commands::register::run(args).await,
        Commands::Address(args) => commands::address::run(args).await,
        Commands::GetGas(args) => commands::get_gas::run(args).await,
        Commands::EvmSend(args) => commands::evm_send::run(args).await,
        Commands::Transfer(args) => commands::transfer::run(args).await,
        Commands::Withdraw(args) => commands::withdraw::run(args).await,
        Commands::SpotBalances(args) => commands::spot_balances::run(args).await,
        Commands::SpotPrices(args) => commands::spot_prices::run(args).await,
        Commands::SpotOrder(args) => commands::spot_order::run(args).await,
        Commands::SpotCancel(args) => commands::spot_cancel::run(args).await,
        Commands::Quickstart(args) => commands::quickstart::run(args).await,
        Commands::DexList(args) => commands::dex_list::run(args).await,
        Commands::DexTransfer(args) => commands::dex_transfer::run(args).await,
        Commands::Markets(args) => commands::markets::run(args).await,
    }
}
