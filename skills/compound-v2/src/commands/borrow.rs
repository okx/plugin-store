// src/commands/borrow.rs — Borrow from Compound V2 (DRY-RUN ONLY for safety)
use anyhow::Result;
use serde_json::{json, Value};

use crate::config::{find_market, to_raw};
use crate::onchainos::resolve_wallet;

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

    // Safety: borrow is dry-run only
    if !dry_run {
        return Ok(json!({
            "ok": false,
            "error": "borrow is only available in dry-run mode (--dry-run) for safety. Run with --dry-run to preview the transaction."
        }));
    }

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
        anyhow::bail!("Amount too small.");
    }

    // borrow(uint256) selector: 0xc5ebeaec
    let calldata = format!("0xc5ebeaec{:064x}", raw_amount);

    Ok(json!({
        "ok": true,
        "dry_run": true,
        "action": format!("borrow {}", asset),
        "warning": "Borrow is dry-run only. Requires sufficient collateral supplied first.",
        "ctoken": market.ctoken,
        "wallet": wallet,
        "amount": amount,
        "raw_amount": raw_amount.to_string(),
        "calldata": calldata,
        "steps": [
            {
                "step": 1,
                "action": format!("c{}.borrow(amount)", asset),
                "to": market.ctoken,
                "calldata": calldata,
                "note": "Requires: collateral factor * collateral value >= borrow value"
            }
        ]
    }))
}
