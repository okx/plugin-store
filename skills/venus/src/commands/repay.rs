// Venus — repay command (repayBorrow)

use crate::{config, onchainos};
use alloy_primitives::U256;
use alloy_sol_types::{sol, SolCall};
use anyhow::Result;

sol! {
    function repayBorrow(uint256 repayAmount) external returns (uint256);
}

pub async fn execute(
    chain_id: u64,
    asset: &str,
    amount: f64,
    dry_run: bool,
) -> Result<()> {
    config::get_rpc(chain_id)?;
    let (vtoken_addr, underlying_addr, decimals, is_native) = config::resolve_asset(asset)?;

    let amount_raw = (amount * 10f64.powi(decimals as i32)) as u128;

    let calldata = format!(
        "0x{}",
        hex::encode(
            repayBorrowCall {
                repayAmount: U256::from(amount_raw),
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
                "action": "repay",
                "asset": asset,
                "amount": amount,
                "amount_raw": amount_raw.to_string(),
                "vtoken": vtoken_addr,
                "calldata": calldata
            })
        );
        return Ok(());
    }

    // Resolve wallet after dry_run guard
    let wallet = onchainos::resolve_wallet(chain_id)?;
    let _ = wallet;

    // For ERC-20: approve vToken to spend repay amount
    // ask user to confirm before executing on-chain
    if !is_native {
        let approve_result =
            onchainos::erc20_approve(chain_id, underlying_addr, vtoken_addr, amount_raw, false)
                .await?;
        if !approve_result["ok"].as_bool().unwrap_or(false) {
            anyhow::bail!("ERC-20 approve failed: {}", approve_result);
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    let result =
        onchainos::wallet_contract_call(chain_id, vtoken_addr, &calldata, None, false).await?;
    let tx_hash = onchainos::extract_tx_hash(&result);

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "action": "repay",
            "asset": asset,
            "amount": amount,
            "vtoken": vtoken_addr,
            "tx_hash": tx_hash
        })
    );

    Ok(())
}
