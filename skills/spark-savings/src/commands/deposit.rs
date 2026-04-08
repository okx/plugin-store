use anyhow::Context;
use serde_json::{json, Value};

use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Deposit USDS into sUSDS savings vault.
///
/// On L2 (Base, Arbitrum, Optimism):
///   1. approve(USDS → PSM3, amount)
///   2. PSM3.swapExactIn(USDS, sUSDS, amount, 0, receiver, 0)
///
/// On Ethereum:
///   1. approve(USDS → sUSDS, amount)
///   2. sUSDS.deposit(amount, receiver)
pub async fn run(
    chain_id: u64,
    amount: f64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    let wallet = match from {
        Some(addr) => addr.to_string(),
        None => onchainos::resolve_wallet(chain_id, dry_run)
            .context("Failed to resolve wallet address")?,
    };

    let amount_minimal = rpc::to_minimal(amount, 18);

    // Preview: how many sUSDS will be received?
    let preview_susds = if cfg.use_psm3 {
        let psm3 = cfg.psm3.unwrap();
        // previewSwapExactIn(USDS, sUSDS, amount) = 0x00d8088a
        let data = format!(
            "0x00d8088a{}{}{}",
            rpc::encode_address(cfg.usds),
            rpc::encode_address(cfg.susds),
            rpc::encode_u256(amount_minimal)
        );
        match rpc::eth_call(cfg.rpc_url, psm3, &data).await {
            Ok(hex) => rpc::decode_u256(&hex).unwrap_or(amount_minimal),
            Err(_) => amount_minimal,
        }
    } else {
        // previewDeposit(uint256 assets) = 0xef8b30f7
        let data = format!("0xef8b30f7{}", rpc::encode_u256(amount_minimal));
        match rpc::eth_call(cfg.rpc_url, cfg.susds, &data).await {
            Ok(hex) => rpc::decode_u256(&hex).unwrap_or(amount_minimal),
            Err(_) => amount_minimal,
        }
    };
    let preview_human = rpc::from_minimal(preview_susds, 18);

    if dry_run {
        let (approve_target, deposit_calldata, deposit_target) = build_calldata(cfg, amount_minimal, &wallet);
        let approve_calldata = onchainos::encode_approve(&approve_target, amount_minimal);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "chain": cfg.name,
            "chainId": chain_id,
            "wallet": wallet,
            "amountUSDS": format!("{:.6}", amount),
            "amountMinimal": amount_minimal.to_string(),
            "estimatedSUSDS": format!("{:.6}", preview_human),
            "steps": [
                {
                    "step": 1,
                    "action": "approve",
                    "token": cfg.usds,
                    "spender": approve_target,
                    "simulatedCommand": format!(
                        "onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
                        chain_id, cfg.usds, approve_calldata
                    )
                },
                {
                    "step": 2,
                    "action": if cfg.use_psm3 { "swapExactIn (USDS→sUSDS)" } else { "deposit" },
                    "contract": deposit_target,
                    "simulatedCommand": format!(
                        "onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
                        chain_id, deposit_target, deposit_calldata
                    )
                }
            ]
        }));
    }

    let (approve_target, deposit_calldata, deposit_target) = build_calldata(cfg, amount_minimal, &wallet);

    // Step 1: approve
    let approve_calldata = onchainos::encode_approve(&approve_target, amount_minimal);
    let approve_result =
        onchainos::wallet_contract_call(chain_id, cfg.usds, &approve_calldata, false)
            .context("ERC-20 approve failed")?;
    let approve_tx = onchainos::extract_tx_hash(&approve_result);

    // Wait for approve before deposit
    if approve_tx.starts_with("0x") && approve_tx != "0x" {
        let _ = rpc::wait_for_tx(cfg.rpc_url, &approve_tx).await;
    }

    // Step 2: deposit / swap
    let deposit_result =
        onchainos::wallet_contract_call(chain_id, &deposit_target, &deposit_calldata, false)
            .context("Deposit/swap failed")?;
    let deposit_tx = onchainos::extract_tx_hash(&deposit_result);

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "wallet": wallet,
        "amountUSDS": format!("{:.6}", amount),
        "amountMinimal": amount_minimal.to_string(),
        "estimatedSUSDS": format!("{:.6}", preview_human),
        "approveTxHash": approve_tx,
        "depositTxHash": deposit_tx,
        "message": format!(
            "Deposited {:.4} USDS → ~{:.4} sUSDS on {}",
            amount, preview_human, cfg.name
        )
    }))
}

/// Build calldata for the deposit operation.
/// Returns (approve_target, deposit_calldata, deposit_target).
fn build_calldata(
    cfg: &crate::config::ChainConfig,
    amount_minimal: u128,
    wallet: &str,
) -> (String, String, String) {
    if cfg.use_psm3 {
        let psm3 = cfg.psm3.unwrap();
        // swapExactIn(assetIn, assetOut, amountIn, minAmountOut, receiver, referralCode)
        // selector: 0x1a019e37
        let calldata = format!(
            "0x1a019e37{}{}{}{}{}{}",
            rpc::encode_address(cfg.usds),
            rpc::encode_address(cfg.susds),
            rpc::encode_u256(amount_minimal),
            rpc::encode_u256(0), // minAmountOut = 0 (no slippage protection in plugin)
            rpc::encode_address(wallet),
            rpc::encode_u256(0)  // referralCode = 0
        );
        (psm3.to_string(), calldata, psm3.to_string())
    } else {
        // ERC-4626: deposit(uint256 assets, address receiver)
        // selector: 0x6e553f65
        let calldata = format!(
            "0x6e553f65{}{}",
            rpc::encode_u256(amount_minimal),
            rpc::encode_address(wallet)
        );
        (cfg.susds.to_string(), calldata, cfg.susds.to_string())
    }
}
