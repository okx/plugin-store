// src/commands/claim_rewards.rs — claim LP fee rewards from ClankerFeeLocker
use crate::config;
use crate::onchainos;
use crate::rpc;
use anyhow::{bail, Result};
use alloy_sol_types::{sol, SolCall};

sol! {
    function collectRewards(address token) external;
}

pub async fn run(
    chain_id: u64,
    token_address: &str,
    from: Option<&str>,
    dry_run: bool,
    confirm: bool,
) -> Result<()> {
    // Require explicit --confirm for live claims
    if !dry_run && !confirm {
        bail!(
            "Claiming rewards requires explicit confirmation. Run with --dry-run first to preview, \
             then re-run with --confirm to execute."
        );
    }

    // ── 1. Resolve wallet address ─────────────────────────────────────────
    let wallet = from
        .map(|s| s.to_string())
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        bail!("Cannot determine wallet address — pass --from or ensure onchainos is logged in");
    }

    // ── 2. Security scan ─────────────────────────────────────────────────
    let scan = onchainos::security_token_scan(chain_id, token_address)?;
    let scan_ok = scan["ok"].as_bool().unwrap_or(false)
        || scan["data"].is_object();
    if !scan_ok {
        bail!(
            "Security scan failed for token {} on chain {}. Aborting to protect funds.",
            token_address,
            chain_id
        );
    }
    // Check for block-level risk indicators
    let risk_level = scan["data"]["riskLevel"]
        .as_str()
        .or_else(|| scan["data"]["risk_level"].as_str())
        .unwrap_or("");
    if risk_level.to_lowercase() == "block" {
        bail!(
            "Token {} flagged as HIGH RISK (block). Refusing to proceed.",
            token_address
        );
    }

    // ── 3. Resolve fee locker address ────────────────────────────────────
    // Clanker V4 lockers are resolved via the factory's feeLockerForToken().
    // If factory lookup fails (token not registered in this factory or call reverts),
    // fall back to the well-known V4 locker address from config.
    let rpc_url = config::rpc_url(chain_id);
    let fee_locker_addr = if let Some(factory) = config::factory_address(chain_id) {
        match rpc::resolve_fee_locker(rpc_url, factory, token_address).await {
            Ok(resolved) if resolved.len() == 42
                && resolved.starts_with("0x")
                && resolved != "0x0000000000000000000000000000000000000000" =>
            {
                resolved
            }
            _ => {
                // Factory lookup failed or returned zero address — use fallback
                config::fallback_fee_locker(chain_id)
                    .ok_or_else(|| anyhow::anyhow!("No fallback fee locker for chain {}", chain_id))?
                    .to_string()
            }
        }
    } else {
        config::fallback_fee_locker(chain_id)
            .ok_or_else(|| anyhow::anyhow!("No fee locker configured for chain {}", chain_id))?
            .to_string()
    };

    // ── 4. Check pending rewards via tokenRewards(address token) ─────────
    // The ClankerFeeLocker exposes tokenRewards(address) for querying and
    // collectRewards(address) for claiming.
    let has_rewards = rpc::has_pending_rewards(rpc_url, &fee_locker_addr, token_address).await;
    if let Ok(false) = has_rewards {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "status": "no_rewards",
                "message": "No claimable rewards at this time for this token.",
                "token_address": token_address,
                "wallet": wallet,
                "fee_locker": fee_locker_addr,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // ── 5. Encode collectRewards(address token) calldata ─────────────────
    let token_addr_parsed: alloy_primitives::Address = token_address
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid token address: {}", token_address))?;

    let call = collectRewardsCall {
        token: token_addr_parsed,
    };
    let calldata = format!("0x{}", hex::encode(call.abi_encode()));

    // ── 6. Dry-run preview ────────────────────────────────────────────────
    if dry_run {
        let preview = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "action": "claim_rewards",
                "chain_id": chain_id,
                "fee_locker": fee_locker_addr,
                "input_data": calldata,
                "from": wallet,
                "token_address": token_address,
                "onchainos_command": format!(
                    "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {} --force",
                    chain_id, fee_locker_addr, calldata, wallet
                ),
                "note": "Run without --dry-run after user confirmation to execute on-chain"
            }
        });
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    // ── 7. Execute on-chain (after user confirmation by agent) ────────────
    // The agent MUST ask user to confirm before reaching this point.
    // --force is only passed when confirm=true (i.e. not a dry-run preview).
    let result = onchainos::wallet_contract_call(
        chain_id,
        &fee_locker_addr,
        &calldata,
        Some(&wallet),
        None,
        confirm, // --force only when user has confirmed
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "action": "claim_rewards",
            "token_address": token_address,
            "fee_locker": fee_locker_addr,
            "from": wallet,
            "chain_id": chain_id,
            "tx_hash": tx_hash,
            "explorer_url": format!(
                "https://{}/tx/{}",
                if chain_id == 42161 { "arbiscan.io" } else { "basescan.org" },
                tx_hash
            )
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
