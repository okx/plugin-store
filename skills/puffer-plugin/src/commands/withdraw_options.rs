use clap::Args;
use serde_json::json;

use crate::config::{
    format_units, parse_units, pufeth_address, puffer_vault_address, rpc_url,
    CHAIN_ID, MIN_WITHDRAWAL_AMOUNT_WEI,
};
use crate::onchainos::resolve_wallet;
use crate::rpc::{convert_to_assets, get_balance, get_total_exit_fee_bps, preview_redeem};

#[derive(Args)]
pub struct WithdrawOptionsArgs {
    /// Amount of pufETH to preview withdrawing (e.g. "0.1"). If omitted, uses current balance.
    #[arg(long)]
    pub amount: Option<String>,
    /// Override wallet address (defaults to onchainos wallet for chain 1).
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn run(args: WithdrawOptionsArgs) -> anyhow::Result<()> {
    if let Err(e) = run_inner(args).await {
        println!("{}", super::error_response(&e, Some("withdraw-options")));
    }
    Ok(())
}

async fn run_inner(args: WithdrawOptionsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let vault = puffer_vault_address();
    let wallet = match args.wallet {
        Some(w) => w,
        None => resolve_wallet(CHAIN_ID)?,
    };

    let pufeth_raw = get_balance(pufeth_address(), &wallet, rpc).await?;
    let amount_raw = match args.amount.as_deref() {
        Some(s) => parse_units(s, 18)?,
        None => pufeth_raw,
    };

    if amount_raw == 0 {
        anyhow::bail!("No pufETH balance and no --amount specified.");
    }
    // Note: we intentionally DO NOT bail if amount > balance — this is a preview command
    // and an external agent may want to see costs for hypothetical sizes. The output
    // flags `amount_exceeds_balance` for the agent to decide.
    let amount_exceeds_balance = amount_raw > pufeth_raw;

    let exit_fee_bps = get_total_exit_fee_bps(vault, rpc).await?;

    // 1-step preview: previewRedeem already subtracts the exit fee.
    let instant_weth_raw = preview_redeem(vault, amount_raw, rpc).await?;
    // Full convertToAssets (no fee) for reference / 2-step WETH estimate.
    let gross_eth_raw = convert_to_assets(vault, amount_raw, rpc).await?;
    let instant_fee_raw = gross_eth_raw.saturating_sub(instant_weth_raw);

    let eligible_for_2step = amount_raw >= MIN_WITHDRAWAL_AMOUNT_WEI;

    let out = json!({
        "ok": true,
        "chain": "ethereum",
        "chain_id": CHAIN_ID,
        "wallet": wallet,
        "wallet_pufeth_balance": format_units(pufeth_raw, 18),
        "wallet_pufeth_balance_raw": pufeth_raw.to_string(),
        "amount_exceeds_balance": amount_exceeds_balance,
        "pufeth_amount": format_units(amount_raw, 18),
        "pufeth_amount_raw": amount_raw.to_string(),
        "options": [
            {
                "method": "instant",
                "description": "1-step withdraw: burns pufETH, sends WETH in the same tx. Always available if vault has liquidity.",
                "fee_bps": exit_fee_bps,
                "fee_pct": (exit_fee_bps as f64) / 100.0,
                "fee_weth": format_units(instant_fee_raw, 18),
                "fee_weth_raw": instant_fee_raw.to_string(),
                "estimated_weth_out": format_units(instant_weth_raw, 18),
                "estimated_weth_out_raw": instant_weth_raw.to_string(),
                "delivery": "immediate (single tx)",
                "command": format!("puffer-plugin instant-withdraw --amount {}", format_units(amount_raw, 18)),
            },
            {
                "method": "queued-2-step",
                "description": "Fee-free queued withdraw. Step 1 submits a request; step 2 claims after batch finalization (~14d).",
                "fee_bps": 0,
                "fee_pct": 0.0,
                "estimated_weth_out": format_units(gross_eth_raw, 18),
                "estimated_weth_out_raw": gross_eth_raw.to_string(),
                "estimated_finalization_days": 14,
                "min_amount_pufeth": format_units(MIN_WITHDRAWAL_AMOUNT_WEI, 18),
                "eligible": eligible_for_2step,
                "delivery": "~14 days (two txs: request-withdraw, then claim-withdraw)",
                "command_step1": format!("puffer-plugin request-withdraw --amount {}", format_units(amount_raw, 18)),
                "command_step2": "puffer-plugin claim-withdraw --id <withdrawalId>",
            }
        ],
        "recommendation": if eligible_for_2step {
            "If you need WETH immediately, use instant (pays exit fee). If you can wait ~14 days, use queued-2-step (no fee)."
        } else {
            "Amount below 0.01 pufETH minimum for queued path — only instant is available."
        },
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
