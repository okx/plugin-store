use anyhow::Context;
use serde_json::{json, Value};

use crate::calldata;
use crate::commands::supply::infer_decimals;
use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Repay borrowed assets on Aave V3 via Pool.repay() ABI calldata.
///
/// Flow:
/// 1. Resolve from address
/// 2. Resolve Pool address at runtime
/// 3. Check user has outstanding debt
/// 4. Check ERC-20 allowance; approve if insufficient
/// 5. Encode repay calldata and submit
pub async fn run(
    chain_id: u64,
    asset: &str,
    amount: Option<f64>,
    all: bool,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if amount.is_none() && !all {
        anyhow::bail!("Specify either --amount <value> or --all for full repayment");
    }

    let cfg = get_chain_config(chain_id)?;

    // Resolve caller address
    let from_addr = resolve_from(from, chain_id)?;

    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(cfg.pool_addresses_provider, cfg.rpc_url)
        .await
        .context("Failed to resolve Pool address")?;

    // Pre-flight: check debt
    let account_data = rpc::get_user_account_data(&pool_addr, &from_addr, cfg.rpc_url)
        .await
        .context("Failed to fetch user account data")?;

    if account_data.total_debt_base == 0 && !dry_run {
        return Ok(json!({
            "ok": true,
            "message": "No outstanding debt to repay.",
            "totalDebtUSD": "0.00"
        }));
    }
    let zero_debt_warning = if account_data.total_debt_base == 0 {
        Some("No outstanding debt detected. Repay calldata shown for simulation only — tx would revert on-chain.")
    } else {
        None
    };

    // Compute repay amount in minimal units
    // For --all: query the wallet's actual token balance and use that as the repay amount.
    // Using uint256.max reverts when wallet balance < accrued dust interest.
    let (amount_minimal, amount_display) = if all {
        let balance = rpc::get_erc20_balance(asset, &from_addr, cfg.rpc_url)
            .await
            .context("Failed to fetch token balance for full repay")?;
        if balance == 0 {
            anyhow::bail!("No {} balance in wallet to repay with", asset);
        }
        (balance, format!("all ({})", balance))
    } else {
        let decimals = onchainos::resolve_token(asset, chain_id)
            .map(|(_, d)| d as u64)
            .unwrap_or_else(|_| infer_decimals(asset));
        let v = amount.unwrap();
        let minimal = (v * 10u128.pow(decimals as u32) as f64) as u128;
        (minimal, v.to_string())
    };

    // Step 4: Check ERC-20 allowance for asset → pool
    // For full repay (u128::MAX) we approve unconditionally since we can't know exact debt
    let needs_approval = if all {
        true
    } else {
        let allowance = rpc::get_allowance(asset, &from_addr, &pool_addr, cfg.rpc_url)
            .await
            .unwrap_or(0);
        allowance < amount_minimal
    };

    let mut approval_result: Option<Value> = None;
    if needs_approval {
        eprintln!(
            "Insufficient ERC-20 allowance for {} → {}. Approving...",
            asset, pool_addr
        );
        let approve_res = onchainos::dex_approve(chain_id, asset, &pool_addr, dry_run)
            .context("onchainos dex approve failed")?;
        // Wait for approve tx to be mined before submitting repay
        if !dry_run {
            let approve_tx = approve_res["data"]["txHash"]
                .as_str()
                .or_else(|| approve_res["txHash"].as_str())
                .unwrap_or("");
            if !approve_tx.is_empty() && approve_tx.starts_with("0x") {
                rpc::wait_for_tx(cfg.rpc_url, approve_tx)
                    .await
                    .context("Approve tx did not confirm in time")?;
            }
        }
        approval_result = Some(approve_res);
    }

    // Step 5: encode and submit repay
    let calldata = calldata::encode_repay(asset, amount_minimal, &from_addr)
        .context("Failed to encode repay calldata")?;

    let result = onchainos::wallet_contract_call(
        chain_id,
        &pool_addr,
        &calldata,
        Some(&from_addr),
        dry_run,
    )
    .context("onchainos wallet contract-call failed")?;

    let tx_hash = result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .or_else(|| result["hash"].as_str())
        .unwrap_or("pending");

    Ok(json!({
        "ok": true,
        "txHash": tx_hash,
        "asset": asset,
        "repayAmount": amount_display,
        "poolAddress": pool_addr,
        "totalDebtBefore": format!("{:.2}", account_data.total_debt_usd()),
        "healthFactorBefore": format!("{:.4}", account_data.health_factor_f64()),
        "approvalExecuted": approval_result.is_some(),
        "approvalResult": approval_result,
        "dryRun": dry_run,
        "warning": zero_debt_warning,
        "raw": result
    }))
}

fn resolve_from(from: Option<&str>, chain_id: u64) -> anyhow::Result<String> {
    if let Some(addr) = from {
        return Ok(addr.to_string());
    }
    onchainos::wallet_address(chain_id).context(
        "No --from address specified and could not resolve active wallet.",
    )
}
