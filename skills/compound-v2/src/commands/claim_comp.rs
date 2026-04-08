// src/commands/claim_comp.rs — Claim accrued COMP rewards from Comptroller
use anyhow::Result;
use serde_json::{json, Value};

use crate::config::COMPTROLLER;
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};

pub async fn run(chain_id: u64, from: Option<String>, dry_run: bool) -> Result<Value> {
    if chain_id != 1 {
        anyhow::bail!("Compound V2 is only supported on Ethereum mainnet (chain 1). Got chain {}.", chain_id);
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

    // claimComp(address) selector: 0xe9af0292
    let wallet_padded = format!("{:0>64}", wallet.trim_start_matches("0x"));
    let calldata = format!("0xe9af0292{}", wallet_padded);

    if dry_run {
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "action": "claim COMP rewards",
            "comptroller": COMPTROLLER,
            "wallet": wallet,
            "calldata": calldata,
            "steps": [
                {
                    "step": 1,
                    "action": "Comptroller.claimComp(wallet)",
                    "to": COMPTROLLER,
                    "calldata": calldata
                }
            ]
        }));
    }

    let result = wallet_contract_call(chain_id, COMPTROLLER, &calldata, Some(&wallet), None, false).await?;
    let tx_hash = extract_tx_hash(&result);

    Ok(json!({
        "ok": true,
        "action": "claim COMP rewards",
        "txHash": tx_hash,
        "wallet": wallet,
        "comptroller": COMPTROLLER,
        "note": "COMP rewards have been claimed and sent to your wallet."
    }))
}
