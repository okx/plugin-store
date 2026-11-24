use clap::Args;
use serde_json::json;

use crate::api::{fetch_eth_price, fetch_pufeth_apy};
use crate::config::{format_units, pufeth_address, puffer_vault_address, rpc_url, CHAIN_ID};
use crate::onchainos::resolve_wallet;
use crate::rpc::{convert_to_assets, get_balance, get_total_exit_fee_bps};

#[derive(Args)]
pub struct PositionsArgs {
    /// Override wallet address (defaults to onchainos wallet for chain 1).
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    if let Err(e) = run_inner(args).await {
        println!("{}", super::error_response(&e, Some("positions")));
    }
    Ok(())
}

async fn run_inner(args: PositionsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let vault = puffer_vault_address();
    let wallet = match args.wallet {
        Some(w) => w,
        None => resolve_wallet(CHAIN_ID)?,
    };

    // Balances and rate — fail loudly on RPC errors (→ EVM-012, no unwrap_or(0))
    let pufeth_raw = get_balance(pufeth_address(), &wallet, rpc).await?;
    let eth_value_raw = convert_to_assets(vault, pufeth_raw, rpc).await?;
    let one_share_assets = convert_to_assets(vault, 1_000_000_000_000_000_000, rpc).await?;
    let exit_fee_bps = get_total_exit_fee_bps(vault, rpc).await?;

    // Best-effort external data
    let eth_price = fetch_eth_price().await;
    let apy = fetch_pufeth_apy().await;

    let eth_value_human = format_units(eth_value_raw, 18);
    let usd_value = eth_price.map(|p| {
        let eth_f = eth_value_raw as f64 / 1e18;
        eth_f * p
    });

    let out = json!({
        "ok": true,
        "chain": "ethereum",
        "chain_id": CHAIN_ID,
        "wallet": wallet,
        "pufeth_balance": format_units(pufeth_raw, 18),
        "pufeth_balance_raw": pufeth_raw.to_string(),
        "eth_equivalent": eth_value_human,
        "eth_equivalent_raw": eth_value_raw.to_string(),
        "usd_value": usd_value,
        "pufeth_to_eth_rate": format_units(one_share_assets, 18),
        "exit_fee_bps": exit_fee_bps,
        "exit_fee_pct": (exit_fee_bps as f64) / 100.0,
        "apy_pct": apy,
        "hints": {
            "instant_withdraw": "1-step instant withdraw applies the exit fee (see exit_fee_pct).",
            "queued_withdraw": "2-step queued withdraw is fee-free but finalizes in ~14 days (min 0.01 pufETH).",
        },
        "next_actions": [
            "puffer-plugin withdraw-options --amount <pufETH>",
            "puffer-plugin rate",
        ],
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
