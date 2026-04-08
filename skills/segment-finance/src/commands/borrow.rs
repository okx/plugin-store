// Segment Finance — borrow command
// Dry-run only per GUARDRAILS (liquidation risk with test wallet)
// Selector: 0xc5ebeaec

use crate::{config, onchainos};
use alloy_primitives::U256;
use alloy_sol_types::{sol, SolCall};
use anyhow::Result;

sol! {
    function borrow(uint256 borrowAmount) external returns (uint256);
}

pub async fn execute(
    chain_id: u64,
    asset: &str,
    amount: f64,
    dry_run: bool,
) -> Result<()> {
    config::get_rpc(chain_id)?;
    let (setoken_addr, _, decimals, _) = config::resolve_asset(asset)?;

    let amount_raw = (amount * 10f64.powi(decimals as i32)) as u128;

    let calldata = format!(
        "0x{}",
        hex::encode(
            borrowCall {
                borrowAmount: U256::from(amount_raw),
            }
            .abi_encode()
        )
    );

    if dry_run {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "action": "borrow",
                "asset": asset,
                "amount": amount,
                "amount_raw": amount_raw.to_string(),
                "setoken": setoken_addr,
                "calldata": calldata,
                "note": "Ensure collateral is supplied and entered as market first via enter-market"
            })
        );
        return Ok(());
    }

    // Resolve wallet after dry_run guard
    let _wallet = onchainos::resolve_wallet(chain_id)?;

    // ask user to confirm before executing on-chain
    let result =
        onchainos::wallet_contract_call(chain_id, setoken_addr, &calldata, None, false).await?;
    let tx_hash = onchainos::extract_tx_hash(&result);

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "action": "borrow",
            "asset": asset,
            "amount": amount,
            "amount_raw": amount_raw.to_string(),
            "setoken": setoken_addr,
            "tx_hash": tx_hash
        })
    );

    Ok(())
}
