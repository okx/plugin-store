use crate::config::get_market_config;
use crate::onchainos;
use crate::rpc;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    market: &str,
    from: Option<String>,
    dry_run: bool,
    confirm: bool,
) -> Result<()> {
    let cfg = get_market_config(chain_id, market)?;

    // Resolve wallet address — must not default to zero address
    let wallet = from
        .clone()
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or log in via onchainos.");
    }

    // Pre-flight: check rewards owed
    let reward_owed = rpc::get_reward_owed(
        cfg.rewards_contract,
        cfg.comet_proxy,
        &wallet,
        cfg.rpc_url,
    )
    .await?;

    if reward_owed == 0 {
        let result = serde_json::json!({
            "ok": true,
            "data": {
                "message": "No claimable COMP rewards at this time.",
                "reward_owed_raw": "0"
            }
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Build CometRewards.claimTo(address comet, address src, address to, bool shouldAccrue)
    // selector: 0x4ff85d94
    let comet_padded = rpc::pad_address(cfg.comet_proxy);
    let wallet_padded = rpc::pad_address(&wallet);
    let bool_true = "0000000000000000000000000000000000000000000000000000000000000001";
    let claim_calldata = format!(
        "0x4ff85d94{}{}{}{}",
        comet_padded, wallet_padded, wallet_padded, bool_true
    );

    // Confirm gate: show preview and exit if --confirm not given (and not dry-run)
    if !dry_run && !confirm {
        let result = serde_json::json!({
            "ok": true,
            "preview": true,
            "operation": "claim-rewards",
            "chain_id": chain_id,
            "market": market,
            "wallet": wallet,
            "reward_owed_raw": reward_owed.to_string(),
            "rewards_contract": cfg.rewards_contract,
            "comet": cfg.comet_proxy,
            "pending_transactions": 1,
            "transactions": [
                {"step": 1, "action": "CometRewards.claimTo", "rewards_contract": cfg.rewards_contract, "comet": cfg.comet_proxy, "src": wallet.clone(), "to": wallet.clone(), "calldata": claim_calldata}
            ],
            "note": "Re-run with --confirm to execute this transaction on-chain."
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    if dry_run {
        let result = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "reward_owed_raw": reward_owed.to_string(),
            "steps": [
                {
                    "step": 1,
                    "action": "CometRewards.claimTo",
                    "rewards_contract": cfg.rewards_contract,
                    "comet": cfg.comet_proxy,
                    "src": wallet,
                    "to": wallet,
                    "should_accrue": true,
                    "calldata": claim_calldata
                }
            ]
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Execute CometRewards.claimTo
    let claim_result = onchainos::wallet_contract_call(
        chain_id,
        cfg.rewards_contract,
        &claim_calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;
    let claim_tx = onchainos::extract_tx_hash_or_err(&claim_result)?;

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "chain_id": chain_id,
            "market": market,
            "wallet": wallet,
            "reward_owed_raw": reward_owed.to_string(),
            "claim_tx_hash": claim_tx,
            "message": "COMP rewards claimed successfully."
        }
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
