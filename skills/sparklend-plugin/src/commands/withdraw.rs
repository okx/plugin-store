use anyhow::Context;
use serde_json::{json, Value};

use crate::calldata;
use crate::config;
use crate::onchainos;
use crate::rpc;

/// Withdraw assets from SparkLend Pool via Pool.withdraw().
///
/// Flow:
/// 1. Resolve token contract address
/// 2. Resolve Pool address via PoolAddressesProvider
/// 3. Check outstanding debt and warn if health factor may be affected
/// 4. Call Pool.withdraw(asset, amount, to)
///    - For --all: amount = type(uint256).max
///    - For --amount X: amount = X in minimal units (auto-capped to aToken balance)
pub async fn run(
    chain_id: u64,
    asset: &str,
    amount: Option<f64>,
    all: bool,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if amount.is_none() && !all {
        anyhow::bail!("Specify either --amount <value> or --all for full withdrawal");
    }
    if let Some(amt) = amount {
        if amt <= 0.0 {
            anyhow::bail!("--amount must be greater than 0");
        }
    }

    let from_addr = resolve_from(from, chain_id)?;

    // Resolve token address and decimals
    let (token_addr, decimals) = onchainos::resolve_token(asset, chain_id)
        .with_context(|| format!("Could not resolve token address for '{}'", asset))?;

    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(config::POOL_ADDRESSES_PROVIDER, config::RPC_URL)
        .await
        .context("Failed to resolve SparkLend Pool address")?;

    // Pre-flight: check outstanding debt
    let account_data = rpc::get_user_account_data(&pool_addr, &from_addr, config::RPC_URL)
        .await
        .context("Failed to fetch user account data")?;

    if account_data.total_debt_usd() >= 0.005 {
        eprintln!(
            "[sparklend] WARNING: You have outstanding debt (${:.4}). Withdrawing collateral reduces \
             your health factor (currently {:.2}). If HF drops below 1.0, the transaction will revert. \
             Repay debt first, or withdraw a smaller amount to keep HF above 1.0.",
            account_data.total_debt_usd(),
            account_data.health_factor_f64(),
        );
    }

    let (amount_minimal, amount_display) = if all {
        (u128::MAX, "all".to_string())
    } else {
        let amt = amount.unwrap();
        let mut minimal = super::supply::human_to_minimal(amt, decimals as u64);

        // Pre-flight: cap --amount to actual aToken (spToken) balance to prevent precision-mismatch revert.
        let actual_atoken_balance: Option<u128> = async {
            let pdp = rpc::get_pool_data_provider(config::POOL_ADDRESSES_PROVIDER, config::RPC_URL)
                .await
                .ok()?;
            let atoken_addr = rpc::get_atoken_address(&pdp, &token_addr, config::RPC_URL)
                .await
                .ok()?;
            rpc::get_erc20_balance(&atoken_addr, &from_addr, config::RPC_URL)
                .await
                .ok()
        }
        .await;

        if let Some(bal) = actual_atoken_balance {
            if bal == 0 && !dry_run {
                anyhow::bail!(
                    "No {} supplied to SparkLend. Nothing to withdraw.",
                    asset
                );
            } else if bal > 0 && minimal > bal {
                eprintln!(
                    "[sparklend] NOTE: Requested {:.6} {} but spToken balance is {:.6}. \
                     Adjusting withdrawal amount down to actual balance.",
                    minimal as f64 / 10f64.powi(decimals as i32),
                    asset,
                    bal as f64 / 10f64.powi(decimals as i32),
                );
                minimal = bal;
            }
        }

        let display_amt = minimal as f64 / 10f64.powi(decimals as i32);
        (minimal, format!("{:.6}", display_amt))
    };

    // Encode calldata
    let calldata = calldata::encode_withdraw(&token_addr, amount_minimal, &from_addr)
        .context("Failed to encode withdraw calldata")?;

    if dry_run {
        let cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, pool_addr, calldata, from_addr
        );
        eprintln!("[dry-run] would execute: {}", cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "asset": asset,
            "tokenAddress": token_addr,
            "amount": amount_display,
            "amountDisplay": amount_display,
            "poolAddress": pool_addr,
            "chain": config::CHAIN_NAME,
            "simulatedCommand": cmd
        }));
    }

    let result = onchainos::wallet_contract_call(
        chain_id,
        &pool_addr,
        &calldata,
        Some(&from_addr),
        false,
    )
    .context("Pool.withdraw() failed")?;

    let tx_hash = result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending");

    Ok(json!({
        "ok": true,
        "txHash": tx_hash,
        "explorer": format!("https://etherscan.io/tx/{}", tx_hash),
        "asset": asset,
        "tokenAddress": token_addr,
        "amount": amount_display,
        "amountDisplay": amount_display,
        "poolAddress": pool_addr,
        "chain": config::CHAIN_NAME,
        "dryRun": false,
        "raw": result
    }))
}

fn resolve_from(from: Option<&str>, chain_id: u64) -> anyhow::Result<String> {
    if let Some(addr) = from {
        return Ok(addr.to_string());
    }
    onchainos::wallet_address(chain_id).context("No --from address and could not resolve active wallet.")
}
