use anyhow::Context;
use serde_json::{json, Value};

use crate::calldata;
use crate::config;
use crate::onchainos;
use crate::rpc;

/// Repay borrowed assets on SparkLend via Pool.repay().
///
/// Flow:
/// 1. Resolve from address
/// 2. Resolve Pool address at runtime
/// 3. Check user has outstanding debt
/// 4. Check ERC-20 allowance; approve if insufficient
/// 5. Wait for approve tx confirmation, then submit repay
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
    if let Some(amt) = amount {
        if amt <= 0.0 {
            anyhow::bail!("--amount must be greater than 0");
        }
    }

    let from_addr = resolve_from(from, chain_id)?;

    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(config::POOL_ADDRESSES_PROVIDER, config::RPC_URL)
        .await
        .context("Failed to resolve SparkLend Pool address")?;

    // Resolve token contract address and decimals
    let (token_addr, decimals) = onchainos::resolve_token(asset, chain_id)
        .with_context(|| format!("Could not resolve token address for '{}'", asset))?;

    // Pre-flight: check debt
    let account_data = rpc::get_user_account_data(&pool_addr, &from_addr, config::RPC_URL)
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

    let (amount_minimal, amount_display) = if all {
        (u128::MAX, "all".to_string())
    } else {
        let v = amount.unwrap();
        let minimal = (v * 10u128.pow(decimals as u32) as f64) as u128;
        (minimal, v.to_string())
    };

    // Check ERC-20 allowance; approve if insufficient.
    // For --all: always approve with u128::MAX so Aave can pull full debt + last-second interest.
    let needs_approval = if all {
        true
    } else {
        let allowance = rpc::get_allowance(&token_addr, &from_addr, &pool_addr, config::RPC_URL)
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
            rpc::wait_for_tx(config::RPC_URL, approve_tx)
                .await
                .context("Approve tx did not confirm in time")?;
        }
        approval_result = Some(approve_res);
    }

    // Encode repay calldata
    let calldata = calldata::encode_repay(&token_addr, amount_minimal, &from_addr)
        .context("Failed to encode repay calldata")?;

    // Dry-run: return preview without broadcasting
    if dry_run {
        let amount_display_fmt = if all { "all".to_string() } else { format!("{:.6}", amount.unwrap_or(0.0)) };
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "asset": asset,
            "tokenAddress": token_addr,
            "repayAmount": amount_display,
            "repayAmountDisplay": amount_display_fmt,
            "poolAddress": pool_addr,
            "chain": config::CHAIN_NAME,
            "totalDebtBefore": format!("{:.2}", account_data.total_debt_usd()),
            "healthFactorBefore": if account_data.health_factor >= u128::MAX / 2 {
                "no_debt".to_string()
            } else {
                format!("{:.4}", account_data.health_factor_f64())
            },
            "approvalCalldata": approval_result.as_ref().and_then(|r| r["simulatedCommand"].as_str()),
            "repayCalldata": calldata,
            "warning": zero_debt_warning,
        }));
    }

    let result = onchainos::wallet_contract_call(
        chain_id,
        &pool_addr,
        &calldata,
        Some(&from_addr),
        false,
    )
    .context("Pool.repay() failed")?;

    let tx_hash = result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .or_else(|| result["hash"].as_str())
        .unwrap_or("pending");

    let amount_display_fmt = if all {
        "all".to_string()
    } else {
        format!("{:.6}", amount.unwrap_or(0.0))
    };

    Ok(json!({
        "ok": true,
        "txHash": tx_hash,
        "explorer": format!("https://etherscan.io/tx/{}", tx_hash),
        "asset": asset,
        "tokenAddress": token_addr,
        "repayAmount": amount_display,
        "repayAmountDisplay": amount_display_fmt,
        "poolAddress": pool_addr,
        "chain": config::CHAIN_NAME,
        "totalDebtBefore": format!("{:.2}", account_data.total_debt_usd()),
        "healthFactorBefore": if account_data.health_factor >= u128::MAX / 2 {
            "no_debt".to_string()
        } else {
            format!("{:.4}", account_data.health_factor_f64())
        },
        "approvalExecuted": approval_result.is_some(),
        "dryRun": false,
        "warning": zero_debt_warning,
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
