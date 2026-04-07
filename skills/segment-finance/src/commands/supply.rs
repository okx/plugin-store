// Segment Finance — supply command
// Compound V2 fork: ERC-20 assets use approve + mint(uint256)
// Native BNB uses mint() payable (selector 0x1249c58b)

use crate::{config, onchainos};
use alloy_primitives::U256;
use alloy_sol_types::{sol, SolCall};
use anyhow::Result;

sol! {
    function mint(uint256 mintAmount) external returns (uint256);
}

pub async fn execute(
    chain_id: u64,
    asset: &str,
    amount: f64,
    dry_run: bool,
) -> Result<()> {
    config::get_rpc(chain_id)?;
    let (setoken_addr, underlying_addr, decimals, is_native) = config::resolve_asset(asset)?;

    // Amount in raw units (10^decimals)
    let amount_raw = (amount * 10f64.powi(decimals as i32)) as u128;

    if is_native {
        // BNB supply: seBNB.mint() payable
        // Selector: 0x1249c58b (mint() with no args; value = msg.value)
        let calldata = "0x1249c58b".to_string();

        if dry_run {
            println!(
                "{}",
                serde_json::json!({
                    "ok": true,
                    "dry_run": true,
                    "action": "supply",
                    "asset": asset,
                    "amount": amount,
                    "amount_raw": amount_raw.to_string(),
                    "setoken": setoken_addr,
                    "calldata": calldata,
                    "note": "BNB supply: mint() payable with --amt <wei>"
                })
            );
            return Ok(());
        }

        // Resolve wallet after dry_run guard
        let _wallet = onchainos::resolve_wallet(chain_id)?;

        // ask user to confirm before executing on-chain
        let result = onchainos::wallet_contract_call(
            chain_id,
            setoken_addr,
            &calldata,
            Some(amount_raw as u64),
            false,
        )
        .await?;
        let tx_hash = onchainos::extract_tx_hash(&result);

        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "action": "supply",
                "asset": asset,
                "amount": amount,
                "amount_raw": amount_raw.to_string(),
                "setoken": setoken_addr,
                "tx_hash": tx_hash
            })
        );
    } else {
        // ERC-20 supply: approve(seToken, amount) + seToken.mint(amount)
        let calldata = format!(
            "0x{}",
            hex::encode(
                mintCall {
                    mintAmount: U256::from(amount_raw),
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
                    "action": "supply",
                    "asset": asset,
                    "amount": amount,
                    "amount_raw": amount_raw.to_string(),
                    "setoken": setoken_addr,
                    "underlying": underlying_addr,
                    "calldata": calldata,
                    "note": "ERC-20 supply: approve then mint"
                })
            );
            return Ok(());
        }

        // Resolve wallet after dry_run guard
        let _wallet = onchainos::resolve_wallet(chain_id)?;

        // 1. Approve seToken to spend underlying
        // ask user to confirm before executing on-chain
        let approve_result =
            onchainos::erc20_approve(chain_id, underlying_addr, setoken_addr, amount_raw, false)
                .await?;
        if !approve_result["ok"].as_bool().unwrap_or(false) {
            anyhow::bail!("ERC-20 approve failed: {}", approve_result);
        }

        // Wait 3 seconds for approve to confirm before mint
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        // 2. mint(amount)
        let result =
            onchainos::wallet_contract_call(chain_id, setoken_addr, &calldata, None, false).await?;
        let tx_hash = onchainos::extract_tx_hash(&result);

        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "action": "supply",
                "asset": asset,
                "amount": amount,
                "amount_raw": amount_raw.to_string(),
                "setoken": setoken_addr,
                "underlying": underlying_addr,
                "tx_hash": tx_hash
            })
        );
    }

    Ok(())
}
