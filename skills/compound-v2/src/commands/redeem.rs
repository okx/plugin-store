// src/commands/redeem.rs — Redeem cTokens to get back underlying asset
use anyhow::Result;
use serde_json::{json, Value};

use crate::config::{find_market, RPC_URL, to_raw};
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};
use crate::rpc::balance_of;

pub async fn run(
    chain_id: u64,
    asset: String,
    ctoken_amount: f64,
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

    // cToken has 8 decimals
    let raw_ctoken = to_raw(ctoken_amount, market.ctoken_decimals);
    if raw_ctoken == 0 {
        anyhow::bail!("cToken amount too small.");
    }

    // redeem(uint256) selector: 0xdb006a75
    let calldata = format!("0xdb006a75{:064x}", raw_ctoken);

    if dry_run {
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "action": format!("redeem c{}", asset),
            "ctoken": market.ctoken,
            "ctoken_amount": ctoken_amount,
            "raw_ctoken_amount": raw_ctoken.to_string(),
            "calldata": calldata,
            "steps": [
                {
                    "step": 1,
                    "action": format!("c{}.redeem(cTokenAmount)", asset),
                    "to": market.ctoken,
                    "calldata": calldata
                }
            ]
        }));
    }

    let rpc = RPC_URL;

    // Check current cToken balance
    let current_ctoken = balance_of(market.ctoken, &wallet, rpc).await.unwrap_or(0);
    if raw_ctoken > current_ctoken {
        anyhow::bail!(
            "Insufficient cToken balance. Have: {} c{} (raw: {}), requested: {} (raw: {})",
            (current_ctoken as f64) / 1e8,
            asset,
            current_ctoken,
            ctoken_amount,
            raw_ctoken
        );
    }

    let result = wallet_contract_call(chain_id, market.ctoken, &calldata, Some(&wallet), None, false).await?;
    let tx_hash = extract_tx_hash(&result);

    // Read updated balance
    let new_ctoken_bal = balance_of(market.ctoken, &wallet, rpc).await.unwrap_or(0);
    let new_ctoken_human = (new_ctoken_bal as f64) / 1e8;

    Ok(json!({
        "ok": true,
        "action": format!("redeem c{}", asset),
        "txHash": tx_hash,
        "ctoken_redeemed": ctoken_amount,
        "raw_ctoken": raw_ctoken.to_string(),
        "asset": asset,
        "ctoken": market.ctoken,
        "new_ctoken_balance": format!("{:.8}", new_ctoken_human)
    }))
}
