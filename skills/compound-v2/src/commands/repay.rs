// src/commands/repay.rs — Repay Compound V2 borrow (DRY-RUN ONLY for safety)
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

    // Safety: repay is dry-run only
    if !dry_run {
        return Ok(json!({
            "ok": false,
            "error": "repay is only available in dry-run mode (--dry-run) for safety. Run with --dry-run to preview the transaction."
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

    if market.is_eth {
        // repayBorrow() payable — selector: 0x4e4d9fea
        let calldata = "0x4e4d9fea".to_string();
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "action": "repay ETH borrow",
            "warning": "Repay is dry-run only.",
            "ctoken": market.ctoken,
            "wallet": wallet,
            "amount_eth": amount,
            "amount_wei": raw_amount.to_string(),
            "calldata": calldata,
            "steps": [
                {
                    "step": 1,
                    "action": "cETH.repayBorrow() payable",
                    "to": market.ctoken,
                    "value_wei": raw_amount.to_string(),
                    "calldata": calldata
                }
            ]
        }));
    }

    // ERC20 path: approve + repayBorrow(uint256)
    let underlying = market.underlying.expect("ERC20 market must have underlying");

    // repayBorrow(uint256) selector: 0x0e752702
    let repay_calldata = format!("0x0e752702{:064x}", raw_amount);
    let approve_calldata = format!(
        "0x095ea7b3{:0>64}{:064x}",
        market.ctoken.trim_start_matches("0x"),
        raw_amount
    );

    Ok(json!({
        "ok": true,
        "dry_run": true,
        "action": format!("repay {} borrow", asset),
        "warning": "Repay is dry-run only.",
        "ctoken": market.ctoken,
        "underlying": underlying,
        "wallet": wallet,
        "amount": amount,
        "raw_amount": raw_amount.to_string(),
        "steps": [
            {
                "step": 1,
                "action": format!("{}.approve(cToken, amount)", asset),
                "to": underlying,
                "calldata": approve_calldata
            },
            {
                "step": 2,
                "action": format!("c{}.repayBorrow(amount)", asset),
                "to": market.ctoken,
                "calldata": repay_calldata
            }
        ]
    }))
}
