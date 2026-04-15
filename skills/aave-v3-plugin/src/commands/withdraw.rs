use anyhow::Context;
use serde_json::{json, Value};

use crate::calldata;
use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Withdraw assets from Aave V3 Pool via direct contract-call.
///
/// Flow:
/// 1. Resolve token contract address
/// 2. Resolve Pool address via PoolAddressesProvider
/// 3. Call Pool.withdraw(asset, amount, to)
///    - For --all: amount = type(uint256).max
///    - For --amount X: amount = X in minimal units
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

    let cfg = get_chain_config(chain_id)?;

    let from_addr = resolve_from(from, chain_id)?;

    // Resolve token address and decimals
    let (token_addr, decimals) = onchainos::resolve_token(asset, chain_id)
        .with_context(|| format!("Could not resolve token address for '{}'", asset))?;

    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(cfg.pool_addresses_provider, cfg.rpc_url)
        .await
        .context("Failed to resolve Pool address")?;

    // Pre-flight: check outstanding debt (Problem 3)
    let account_data = rpc::get_user_account_data(&pool_addr, &from_addr, cfg.rpc_url)
        .await
        .context("Failed to fetch user account data")?;

    if account_data.total_debt_base > 0 && !all {
        // Warn but don't block — let Aave enforce HF constraints on-chain
        eprintln!(
            "[aave-v3] WARNING: You have outstanding debt (${:.2}). Withdrawing collateral reduces \
             your health factor (currently {:.2}). If HF drops below 1.0, the transaction will revert. \
             Use --all to withdraw the maximum safe amount, or repay debt first.",
            account_data.total_debt_usd(),
            account_data.health_factor_f64(),
        );
    }

    let (amount_minimal, amount_display) = if all {
        (u128::MAX, "all".to_string())
    } else {
        let amt = amount.unwrap();
        let mut minimal = super::supply::human_to_minimal(amt, decimals as u64);

        // Pre-flight: cap --amount to actual aToken balance to prevent precision-mismatch revert.
        // aToken balance may differ slightly from the "round" amount the user sees
        // (e.g. 0.999998 USDC when user requests 1.0) due to Aave internal rounding.
        let actual_atoken_balance: Option<u128> = async {
            let pdp = rpc::get_pool_data_provider(cfg.pool_addresses_provider, cfg.rpc_url)
                .await
                .ok()?;
            let atoken_addr = rpc::get_atoken_address(&pdp, &token_addr, cfg.rpc_url)
                .await
                .ok()?;
            rpc::get_erc20_balance(&atoken_addr, &from_addr, cfg.rpc_url)
                .await
                .ok()
        }
        .await;

        if let Some(bal) = actual_atoken_balance {
            if bal == 0 && !dry_run {
                anyhow::bail!(
                    "No {} supplied to Aave on this chain. Nothing to withdraw.",
                    asset
                );
            } else if bal > 0 && minimal > bal {
                // Precision fix: requested amount slightly exceeds aToken balance
                // (e.g. user requests 1.0 but balance is 0.999998 due to Aave rounding)
                eprintln!(
                    "[aave-v3] NOTE: Requested {:.6} {} but aToken balance is {:.6}. \
                     Adjusting withdrawal amount down to actual balance.",
                    minimal as f64 / 10f64.powi(decimals as i32),
                    asset,
                    bal as f64 / 10f64.powi(decimals as i32),
                );
                minimal = bal;
            }
        }

        let display_amt = minimal as f64 / 10f64.powi(decimals as i32);
        (minimal, format!("{:.2}", display_amt))
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
        "asset": asset,
        "tokenAddress": token_addr,
        "amount": amount_display,
        "amountDisplay": amount_display,
        "poolAddress": pool_addr,
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
