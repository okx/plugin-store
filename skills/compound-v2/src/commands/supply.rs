// src/commands/supply.rs — Supply assets to Compound V2 (mint cTokens)
use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

use crate::config::{find_market, RPC_URL, to_raw};
use crate::onchainos::{resolve_wallet, wallet_contract_call, erc20_approve, extract_tx_hash};
use crate::rpc::balance_of;

pub async fn run(
    chain_id: u64,
    asset: String,
    amount: f64,
    from: Option<String>,
    dry_run: bool,
) -> Result<Value> {
    if chain_id != 1 {
        anyhow::bail!("Compound V2 is only supported on Ethereum mainnet (chain 1). Got chain {}.", chain_id);
    }

    let market = find_market(&asset)
        .ok_or_else(|| anyhow::anyhow!("Unknown asset '{}'. Supported: ETH, USDT, USDC, DAI", asset))?;

    let wallet = match from {
        Some(ref w) => w.clone(),
        None => {
            if dry_run {
                "0x0000000000000000000000000000000000000000".to_string()
            } else {
                resolve_wallet(chain_id)?
            }
        }
    };

    let raw_amount = to_raw(amount, market.underlying_decimals);
    if raw_amount == 0 {
        anyhow::bail!("Amount too small to represent in base units.");
    }

    let rpc = RPC_URL;

    if market.is_eth {
        // cETH: mint() payable — send ETH as value
        // selector: 0x1249c58b
        let calldata = "0x1249c58b".to_string();

        if dry_run {
            return Ok(json!({
                "ok": true,
                "dry_run": true,
                "action": "supply ETH",
                "ctoken": market.ctoken,
                "amount_eth": amount,
                "amount_wei": raw_amount.to_string(),
                "calldata": calldata,
                "steps": [
                    { "step": 1, "action": "cETH.mint() payable", "to": market.ctoken, "value_wei": raw_amount.to_string() }
                ]
            }));
        }

        let result = wallet_contract_call(chain_id, market.ctoken, &calldata, Some(&wallet), Some(raw_amount), false).await?;
        let tx_hash = extract_tx_hash(&result);

        // Read updated cToken balance
        let new_ctoken_bal = balance_of(market.ctoken, &wallet, rpc).await.unwrap_or(0);
        let new_ctoken_human = (new_ctoken_bal as f64) / 1e8;

        return Ok(json!({
            "ok": true,
            "action": "supply ETH",
            "txHash": tx_hash,
            "amount_eth": amount,
            "amount_wei": raw_amount.to_string(),
            "new_cETH_balance": format!("{:.8}", new_ctoken_human),
            "ctoken_address": market.ctoken
        }));
    }

    // ERC20 path: approve + mint(uint256)
    let underlying = market.underlying.expect("ERC20 market must have underlying");

    if dry_run {
        // selector: 0xa0712d68 (mint(uint256))
        let mint_calldata = format!("0xa0712d68{:064x}", raw_amount);
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "action": format!("supply {}", asset),
            "ctoken": market.ctoken,
            "underlying": underlying,
            "amount": amount,
            "raw_amount": raw_amount.to_string(),
            "steps": [
                {
                    "step": 1,
                    "action": format!("{}.approve(cToken, amount)", asset),
                    "to": underlying,
                    "calldata": format!("0x095ea7b3{:0>64}{:064x}", market.ctoken.trim_start_matches("0x"), raw_amount)
                },
                {
                    "step": 2,
                    "action": "cToken.mint(amount)",
                    "to": market.ctoken,
                    "calldata": mint_calldata
                }
            ]
        }));
    }

    // Step 1: ERC20 approve
    let approve_result = erc20_approve(chain_id, underlying, market.ctoken, raw_amount, Some(&wallet), false).await?;
    let approve_hash = extract_tx_hash(&approve_result);
    eprintln!("[supply] approve txHash: {}", approve_hash);

    // Wait for nonce safety
    sleep(Duration::from_secs(3)).await;

    // Step 2: mint(uint256) — selector: 0xa0712d68
    let mint_calldata = format!("0xa0712d68{:064x}", raw_amount);
    let mint_result = wallet_contract_call(chain_id, market.ctoken, &mint_calldata, Some(&wallet), None, false).await?;
    let mint_hash = extract_tx_hash(&mint_result);

    // Read updated cToken balance
    let new_ctoken_bal = balance_of(market.ctoken, &wallet, rpc).await.unwrap_or(0);
    let new_ctoken_human = (new_ctoken_bal as f64) / 1e8;

    Ok(json!({
        "ok": true,
        "action": format!("supply {}", asset),
        "approveTxHash": approve_hash,
        "mintTxHash": mint_hash,
        "amount": amount,
        "raw_amount": raw_amount.to_string(),
        "asset": asset,
        "ctoken": market.ctoken,
        "new_ctoken_balance": format!("{:.8}", new_ctoken_human)
    }))
}
