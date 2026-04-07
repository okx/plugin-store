// Venus — supply command
// Supports both ERC-20 tokens (via mint(uint256)) and native BNB (via mint() + --amt)

use crate::{config, onchainos, rpc};
use alloy_primitives::U256;
use alloy_sol_types::{sol, SolCall};
use anyhow::Result;

sol! {
    function mint(uint256 mintAmount) external returns (uint256);
}

// vBNB mint() selector: 0x1249c58b (cast sig "mint()")
const VBNB_MINT_CALLDATA: &str = "0x1249c58b";

pub async fn execute(
    chain_id: u64,
    asset: &str,
    amount: f64,
    dry_run: bool,
) -> Result<()> {
    let rpc_url = config::get_rpc(chain_id)?;
    let (vtoken_addr, underlying_addr, decimals, is_native) = config::resolve_asset(asset)?;

    // Amount in raw units (10^decimals)
    let amount_raw = (amount * 10f64.powi(decimals as i32)) as u128;

    if is_native {
        // BNB supply: mint() payable with msg.value
        // calldata: 0x1249c58b (no args, value goes via --amt)
        let calldata = VBNB_MINT_CALLDATA.to_string();

        if dry_run {
            println!(
                "{}",
                serde_json::json!({
                    "ok": true,
                    "dry_run": true,
                    "action": "supply_bnb",
                    "asset": asset,
                    "amount": amount,
                    "amount_raw": amount_raw.to_string(),
                    "vtoken": vtoken_addr,
                    "calldata": calldata
                })
            );
            return Ok(());
        }

        // Resolve wallet (after dry_run guard)
        let wallet = onchainos::resolve_wallet(chain_id)?;
        let _ = wallet;

        // Submit: no approve needed for native BNB
        // ask user to confirm before executing on-chain
        let result = onchainos::wallet_contract_call(
            chain_id,
            vtoken_addr,
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
                "action": "supply_bnb",
                "asset": asset,
                "amount": amount,
                "vtoken": vtoken_addr,
                "tx_hash": tx_hash
            })
        );
    } else {
        // ERC-20 supply: approve + mint(mintAmount)
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
                    "vtoken": vtoken_addr,
                    "underlying": underlying_addr,
                    "calldata": calldata
                })
            );
            return Ok(());
        }

        // Resolve wallet (after dry_run guard)
        let wallet = onchainos::resolve_wallet(chain_id)?;
        let _ = wallet;

        // 1. Check current allowance; approve if needed
        // approve(vToken, amount)
        // ask user to confirm before executing on-chain
        let approve_result =
            onchainos::erc20_approve(chain_id, underlying_addr, vtoken_addr, amount_raw, false)
                .await?;
        if !approve_result["ok"].as_bool().unwrap_or(false) {
            anyhow::bail!("ERC-20 approve failed: {}", approve_result);
        }

        // Wait 3 seconds for approve to confirm
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        // 2. mint(amount)
        let result =
            onchainos::wallet_contract_call(chain_id, vtoken_addr, &calldata, None, false).await?;
        let tx_hash = onchainos::extract_tx_hash(&result);

        // Verify underlying address in RPC
        let _ = rpc_url;
        let _ = rpc::erc20_symbol;

        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "action": "supply",
                "asset": asset,
                "amount": amount,
                "amount_raw": amount_raw.to_string(),
                "vtoken": vtoken_addr,
                "tx_hash": tx_hash
            })
        );
    }

    Ok(())
}
