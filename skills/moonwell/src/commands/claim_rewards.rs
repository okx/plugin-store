// src/commands/claim_rewards.rs — Claim WELL rewards from Comptroller
use anyhow::Result;
use serde_json::{json, Value};

use crate::config::chain_config;
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};

pub async fn run(
    chain_id: u64,
    from: Option<String>,
    dry_run: bool,
) -> Result<Value> {
    let cfg = chain_config(chain_id)?;
    let from_addr = match &from {
        Some(f) => f.clone(),
        None => resolve_wallet(chain_id, dry_run)?,
    };

    eprintln!(
        "[moonwell] claim-rewards for {} on chain {} via Comptroller {}",
        from_addr, chain_id, cfg.comptroller
    );

    // claimReward(address) — selector 0xd279c191
    let addr_padded = format!("{:0>64}", from_addr.trim_start_matches("0x"));
    let calldata = format!("0xd279c191{}", addr_padded);

    let result = wallet_contract_call(
        chain_id,
        cfg.comptroller,
        &calldata,
        Some(&from_addr),
        None,
        dry_run,
    ).await?;
    let tx_hash = extract_tx_hash(&result);

    Ok(json!({
        "ok": true,
        "action": "claim-rewards",
        "wallet": from_addr,
        "comptroller": cfg.comptroller,
        "chain_id": chain_id,
        "dry_run": dry_run,
        "txHash": tx_hash,
        "note": "Claimed all accrued WELL rewards from Moonwell Comptroller."
    }))
}
