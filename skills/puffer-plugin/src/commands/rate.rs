use clap::Args;
use serde_json::json;

use crate::config::{format_units, puffer_vault_address, rpc_url, withdrawal_manager_address, CHAIN_ID, MIN_WITHDRAWAL_AMOUNT_WEI};
use crate::rpc::{convert_to_assets, get_finalized_batch, get_total_exit_fee_bps, get_withdrawals_length, total_assets};

#[derive(Args)]
pub struct RateArgs {}

pub async fn run(_args: RateArgs) -> anyhow::Result<()> {
    if let Err(e) = run_inner().await {
        println!("{}", super::error_response(&e, Some("rate")));
    }
    Ok(())
}

async fn run_inner() -> anyhow::Result<()> {
    let rpc = rpc_url();
    let vault = puffer_vault_address();
    let manager = withdrawal_manager_address();

    let one_share_assets = convert_to_assets(vault, 1_000_000_000_000_000_000, rpc).await?;
    let total_assets_raw = total_assets(vault, rpc).await?;
    let exit_fee_bps = get_total_exit_fee_bps(vault, rpc).await?;
    let finalized_batch = get_finalized_batch(manager, rpc).await?;
    let queue_len = get_withdrawals_length(manager, rpc).await?;

    let out = json!({
        "ok": true,
        "chain": "ethereum",
        "chain_id": CHAIN_ID,
        "vault": vault,
        "withdrawal_manager": manager,
        "pufeth_to_eth_rate": format_units(one_share_assets, 18),
        "pufeth_to_eth_rate_raw": one_share_assets.to_string(),
        "total_assets_eth": format_units(total_assets_raw, 18),
        "total_assets_eth_raw": total_assets_raw.to_string(),
        "exit_fee_bps": exit_fee_bps,
        "exit_fee_pct": (exit_fee_bps as f64) / 100.0,
        "queued_withdraw": {
            "latest_finalized_batch_index": finalized_batch,
            "total_withdrawal_requests": queue_len,
            "min_amount_pufeth": format_units(MIN_WITHDRAWAL_AMOUNT_WEI, 18),
            "estimated_finalization_days": 14,
        },
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
