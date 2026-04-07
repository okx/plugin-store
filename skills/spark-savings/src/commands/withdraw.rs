use anyhow::Context;
use serde_json::{json, Value};

use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Withdraw sUSDS → USDS.
///
/// On L2 (Base, Arbitrum, Optimism):
///   1. approve(sUSDS → PSM3, amount)
///   2. PSM3.swapExactIn(sUSDS, USDS, amount, 0, receiver, 0)
///
/// On Ethereum (ERC-4626):
///   1. sUSDS.redeem(shares, receiver, owner)
///   (No approve needed — owner is calling)
///
/// `amount_susds`: amount of sUSDS shares to redeem (omit for full balance)
/// `all`: if true, redeem entire sUSDS balance
pub async fn run(
    chain_id: u64,
    amount_susds: Option<f64>,
    all: bool,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    let wallet = match from {
        Some(addr) => addr.to_string(),
        None => onchainos::resolve_wallet(chain_id, dry_run)
            .context("Failed to resolve wallet address")?,
    };

    // Read current sUSDS balance
    let balance_hex = rpc::eth_call(
        cfg.rpc_url,
        cfg.susds,
        &format!("0x70a08231{}", rpc::encode_address(&wallet)),
    )
    .await
    .unwrap_or_default();
    let balance_shares = rpc::decode_u256(&balance_hex).unwrap_or(0);
    let balance_human = rpc::from_minimal(balance_shares, 18);

    let shares_to_redeem = if all {
        // In dry-run with zero balance, use a placeholder amount
        if balance_shares == 0 && dry_run {
            rpc::to_minimal(1.0, 18)
        } else {
            balance_shares
        }
    } else {
        let amt = amount_susds.unwrap_or(0.0);
        if amt <= 0.0 {
            anyhow::bail!("Specify --amount <sUSDS_amount> or --all");
        }
        rpc::to_minimal(amt, 18)
    };

    if !dry_run {
        if shares_to_redeem == 0 {
            anyhow::bail!(
                "No sUSDS balance to withdraw (balance: {:.6} sUSDS)",
                balance_human
            );
        }
        if shares_to_redeem > balance_shares {
            anyhow::bail!(
                "Insufficient sUSDS balance: have {:.6}, requested {:.6}",
                balance_human,
                rpc::from_minimal(shares_to_redeem, 18)
            );
        }
    }

    let shares_human = rpc::from_minimal(shares_to_redeem, 18);

    // Preview: how many USDS will be received?
    let preview_usds = if cfg.use_psm3 {
        let psm3 = cfg.psm3.unwrap();
        // previewSwapExactIn(sUSDS, USDS, shares_amount) = 0x00d8088a
        let data = format!(
            "0x00d8088a{}{}{}",
            rpc::encode_address(cfg.susds),
            rpc::encode_address(cfg.usds),
            rpc::encode_u256(shares_to_redeem)
        );
        match rpc::eth_call(cfg.rpc_url, psm3, &data).await {
            Ok(hex) => rpc::decode_u256(&hex).unwrap_or(shares_to_redeem),
            Err(_) => shares_to_redeem,
        }
    } else {
        // convertToAssets(uint256 shares) = 0x07a2d13a
        let data = format!("0x07a2d13a{}", rpc::encode_u256(shares_to_redeem));
        match rpc::eth_call(cfg.rpc_url, cfg.susds, &data).await {
            Ok(hex) => rpc::decode_u256(&hex).unwrap_or(shares_to_redeem),
            Err(_) => shares_to_redeem,
        }
    };
    let preview_human = rpc::from_minimal(preview_usds, 18);

    if dry_run {
        let withdraw_calldata = build_withdraw_calldata(cfg, shares_to_redeem, &wallet);
        let withdraw_target = if cfg.use_psm3 {
            cfg.psm3.unwrap().to_string()
        } else {
            cfg.susds.to_string()
        };

        let mut steps = vec![];

        if cfg.use_psm3 {
            let psm3 = cfg.psm3.unwrap();
            let approve_calldata = onchainos::encode_approve(psm3, shares_to_redeem);
            steps.push(json!({
                "step": 1,
                "action": "approve sUSDS → PSM3",
                "token": cfg.susds,
                "spender": psm3,
                "simulatedCommand": format!(
                    "onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
                    chain_id, cfg.susds, approve_calldata
                )
            }));
            steps.push(json!({
                "step": 2,
                "action": "swapExactIn (sUSDS→USDS)",
                "contract": psm3,
                "simulatedCommand": format!(
                    "onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
                    chain_id, withdraw_target, withdraw_calldata
                )
            }));
        } else {
            steps.push(json!({
                "step": 1,
                "action": "redeem",
                "contract": cfg.susds,
                "simulatedCommand": format!(
                    "onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
                    chain_id, withdraw_target, withdraw_calldata
                )
            }));
        }

        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "chain": cfg.name,
            "chainId": chain_id,
            "wallet": wallet,
            "sUSDS_balance": format!("{:.6}", balance_human),
            "sUSDS_toRedeem": format!("{:.6}", shares_human),
            "estimatedUSDS": format!("{:.6}", preview_human),
            "steps": steps
        }));
    }

    // Execute withdraw
    let withdraw_calldata = build_withdraw_calldata(cfg, shares_to_redeem, &wallet);
    let withdraw_target = if cfg.use_psm3 {
        cfg.psm3.unwrap().to_string()
    } else {
        cfg.susds.to_string()
    };

    // On L2: need to approve sUSDS to PSM3 first
    let (approve_tx, withdraw_tx) = if cfg.use_psm3 {
        let psm3 = cfg.psm3.unwrap();
        let approve_calldata = onchainos::encode_approve(psm3, shares_to_redeem);
        let approve_result =
            onchainos::wallet_contract_call(chain_id, cfg.susds, &approve_calldata, false)
                .context("sUSDS approve failed")?;
        let approve_tx = onchainos::extract_tx_hash(&approve_result);

        if approve_tx.starts_with("0x") && approve_tx != "0x" {
            let _ = rpc::wait_for_tx(cfg.rpc_url, &approve_tx).await;
        }

        let withdraw_result =
            onchainos::wallet_contract_call(chain_id, &withdraw_target, &withdraw_calldata, false)
                .context("Withdraw swap failed")?;
        let withdraw_tx = onchainos::extract_tx_hash(&withdraw_result);

        (Some(approve_tx), withdraw_tx)
    } else {
        // Ethereum ERC-4626: no approve needed
        let withdraw_result =
            onchainos::wallet_contract_call(chain_id, &withdraw_target, &withdraw_calldata, false)
                .context("Redeem failed")?;
        let withdraw_tx = onchainos::extract_tx_hash(&withdraw_result);
        (None, withdraw_tx)
    };

    let mut result = json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "wallet": wallet,
        "sUSDS_redeemed": format!("{:.6}", shares_human),
        "estimatedUSDS": format!("{:.6}", preview_human),
        "withdrawTxHash": withdraw_tx,
        "message": format!(
            "Withdrawn {:.4} sUSDS → ~{:.4} USDS on {}",
            shares_human, preview_human, cfg.name
        )
    });

    if let Some(tx) = approve_tx {
        result["approveTxHash"] = json!(tx);
    }

    Ok(result)
}

/// Build calldata for the withdraw/redeem operation.
fn build_withdraw_calldata(
    cfg: &crate::config::ChainConfig,
    shares: u128,
    wallet: &str,
) -> String {
    if cfg.use_psm3 {
        // swapExactIn(sUSDS, USDS, shares, 0, receiver, 0)
        // selector: 0x1a019e37
        format!(
            "0x1a019e37{}{}{}{}{}{}",
            rpc::encode_address(cfg.susds),
            rpc::encode_address(cfg.usds),
            rpc::encode_u256(shares),
            rpc::encode_u256(0),
            rpc::encode_address(wallet),
            rpc::encode_u256(0)
        )
    } else {
        // ERC-4626: redeem(uint256 shares, address receiver, address owner)
        // selector: 0xba087652
        format!(
            "0xba087652{}{}{}",
            rpc::encode_u256(shares),
            rpc::encode_address(wallet),
            rpc::encode_address(wallet)
        )
    }
}
