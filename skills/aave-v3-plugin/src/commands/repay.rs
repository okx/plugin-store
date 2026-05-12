use anyhow::Context;
use serde_json::{json, Value};

use crate::calldata;
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

    // Resolve token contract address and decimals (handles both symbol and 0x address)
    let (token_addr, decimals) = onchainos::resolve_token(asset, chain_id)
        .with_context(|| format!("Could not resolve token address for '{}'", asset))?;

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

    // Compute repay amount in minimal units.
    // For --all: use u128::MAX, which encode_repay maps to type(uint256).max.
    // Aave interprets uint256.max as "repay full debt including all accrued interest",
    // pulling the exact outstanding amount from the wallet — no dust risk.
    let (amount_minimal, amount_display) = if all {
        (u128::MAX, "all".to_string())
    } else {
        let v = amount.unwrap();
        let minimal = (v * 10u128.pow(decimals as u32) as f64) as u128;
        (minimal, v.to_string())
    };

    // Pre-flight: check wallet ERC-20 balance can cover the repay amount.
    // Skip in dry-run (no real submission), and for --all (Aave only pulls the actual
    // outstanding debt, which we cannot precisely price-check off-chain — let the on-chain
    // call surface insufficient-balance reverts in that edge case).
    if !dry_run && !all {
        let token_balance = rpc::get_erc20_balance(&token_addr, &from_addr, cfg.rpc_url)
            .await
            .context("Failed to fetch token balance")?;
        if token_balance < amount_minimal {
            anyhow::bail!(
                "Insufficient {} balance: need {:.6}, have {:.6}. Add funds to your wallet before repaying.",
                asset,
                amount_minimal as f64 / 10f64.powi(decimals as i32),
                token_balance as f64 / 10f64.powi(decimals as i32),
            );
        }
    }

    // Step 4: Check ERC-20 allowance for token → pool.
    // For --all (amount_minimal == u128::MAX), always approve with u128::MAX (unlimited)
    // so Aave can pull the full debt amount including last-second interest.
    let needs_approval = if all {
        true
    } else {
        let allowance = rpc::get_allowance(&token_addr, &from_addr, &pool_addr, cfg.rpc_url)
            .await
            .context("Failed to fetch token allowance")?;
        allowance < amount_minimal
    };

    let mut approval_result: Option<Value> = None;
    if needs_approval {
        let approve_amount = if all { u128::MAX } else { amount_minimal };
        let approve_calldata = calldata::encode_erc20_approve(&pool_addr, approve_amount)
            .context("Failed to encode approve calldata")?;
        let approve_res = onchainos::wallet_contract_call(
            chain_id,
            &token_addr,
            &approve_calldata,
            Some(&from_addr),
            dry_run,
        )
        .context("ERC-20 approve failed")?;
        // Wait for approve tx to be mined before submitting repay.
        // Bail early if approve was not broadcast — proceeding with a "pending" hash
        // would submit repay before allowance is on-chain, causing STF revert.
        if !dry_run {
            let approve_tx = approve_res["data"]["txHash"]
                .as_str()
                .or_else(|| approve_res["txHash"].as_str())
                .unwrap_or("");
            if approve_tx.is_empty() || !approve_tx.starts_with("0x") {
                anyhow::bail!(
                    "Approve tx was not broadcast (tx hash: '{}'). Check wallet connection and retry.",
                    approve_tx
                );
            }
            rpc::wait_for_tx(cfg.rpc_url, approve_tx)
                .await
                .context("Approve tx did not confirm in time")?;
        }
        approval_result = Some(approve_res);
    }

    // Step 5: encode and submit repay
    let calldata = calldata::encode_repay(&token_addr, amount_minimal, &from_addr)
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

    let amount_display_fmt = if all {
        "all".to_string()
    } else {
        format!("{:.2}", amount.unwrap_or(0.0))
    };

    Ok(json!({
        "ok": true,
        "txHash": tx_hash,
        "asset": asset,
        "repayAmount": amount_display,
        "repayAmountDisplay": amount_display_fmt,
        "poolAddress": pool_addr,
        "totalDebtBefore": format!("{:.2}", account_data.total_debt_usd()),
        "healthFactorBefore": if account_data.health_factor >= u128::MAX / 2 {
            "no_debt".to_string()
        } else {
            format!("{:.4}", account_data.health_factor_f64())
        },
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
