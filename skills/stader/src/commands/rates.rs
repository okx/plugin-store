// rates — Query current Stader ETHx exchange rate and protocol stats
// Pure read operation — no wallet needed, no confirmation required.

use anyhow::Result;
use clap::Args;
use serde_json::json;
use crate::config;
use crate::rpc;

#[derive(Args)]
pub struct RatesArgs {
    /// ETH amount in wei to preview (optional, default 1 ETH = 1000000000000000000)
    #[arg(long, default_value = "1000000000000000000")]
    pub preview_amount: u128,
}

pub async fn execute(args: &RatesArgs, rpc_url: &str) -> Result<()> {
    let manager = config::STADER_MANAGER;

    // Fetch all rate data in parallel-style sequential calls
    let exchange_rate = rpc::get_exchange_rate(rpc_url, manager).await?;
    let total_assets = rpc::get_total_assets(rpc_url, manager).await?;
    let min_deposit = rpc::get_min_deposit(rpc_url, manager).await?;
    let max_deposit = rpc::get_max_deposit(rpc_url, manager).await?;
    let vault_healthy = rpc::is_vault_healthy(rpc_url, manager).await?;

    // Preview deposit for requested amount
    let ethx_preview = rpc::preview_deposit(rpc_url, manager, args.preview_amount).await?;

    // exchange_rate: 1 ETHx = N wei ETH (scaled by 1e18 from the contract)
    // The contract getExchangeRate() returns ETH per ETHx share (1e18 scaled)
    let rate_eth = exchange_rate as f64 / 1e18_f64;

    let output = json!({
        "ok": true,
        "data": {
            "exchange_rate": {
                "ethx_to_eth": format!("{:.6}", rate_eth),
                "description": "1 ETHx is worth this many ETH",
                "raw_wei": exchange_rate.to_string()
            },
            "total_eth_staked": {
                "eth": rpc::format_eth(total_assets),
                "wei": total_assets.to_string()
            },
            "deposit_limits": {
                "min_eth": rpc::format_eth(min_deposit),
                "max_eth": rpc::format_eth(max_deposit),
                "min_wei": min_deposit.to_string(),
                "max_wei": max_deposit.to_string()
            },
            "preview": {
                "eth_in_wei": args.preview_amount.to_string(),
                "eth_in": rpc::format_eth(args.preview_amount),
                "ethx_out_wei": ethx_preview.to_string(),
                "ethx_out": rpc::format_eth(ethx_preview)
            },
            "vault_healthy": vault_healthy,
            "protocol": "Stader ETHx",
            "chain": "Ethereum Mainnet (1)"
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
