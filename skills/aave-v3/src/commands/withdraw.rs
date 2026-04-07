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
/// 2. Fetch the user's current aToken balance
/// 3. Resolve Pool address via PoolAddressesProvider
/// 4. Call Pool.withdraw(asset, amount, to)
///    - For --all: amount = type(uint256).max
///    - For --amount X: amount = X in minimal units, but if X is within 0.01% of
///      the full aToken balance we use type(uint256).max instead. This is necessary
///      because aTokens accrue interest continuously; by the time the tx is mined
///      the balance may be slightly higher than the encoded amount, causing Aave to
///      revert. Passing uint256.max tells Aave to redeem the full balance, avoiding
///      the race condition.
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

    let from_addr = resolve_from(from)?;

    // Resolve token address and decimals
    let (token_addr, decimals) = onchainos::resolve_token(asset, chain_id)
        .with_context(|| format!("Could not resolve token address for '{}'", asset))?;

    let (amount_minimal, amount_display) = if all {
        (u128::MAX, "all".to_string())
    } else {
        let amt = amount.unwrap();
        let minimal = super::supply::human_to_minimal(amt, decimals as u64);

        // aTokens accrue interest every block.  If the requested amount is within
        // 0.01% of the current aToken balance, use uint256::MAX so the tx does not
        // revert when the balance has grown by a few wei between encoding and mining.
        let use_max = match rpc::get_erc20_balance(&token_addr, &from_addr, cfg.rpc_url).await {
            Ok(atoken_balance) if atoken_balance > 0 => {
                // threshold = 0.01% of balance (1 part in 10_000)
                let threshold = atoken_balance / 10_000;
                minimal >= atoken_balance.saturating_sub(threshold)
            }
            _ => false,
        };

        if use_max {
            (u128::MAX, amt.to_string())
        } else {
            (minimal, amt.to_string())
        }
    };

    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(cfg.pool_addresses_provider, cfg.rpc_url)
        .await
        .context("Failed to resolve Pool address")?;

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
    .with_context(|| {
        format!(
            "Pool.withdraw() failed (asset={}, amount={}, pool={}). \
             Check the RPC revert reason above for details.",
            asset, amount_display, pool_addr
        )
    })?;

    let tx_hash = onchainos::extract_tx_hash(&result)?;

    Ok(json!({
        "ok": true,
        "txHash": tx_hash,
        "asset": asset,
        "tokenAddress": token_addr,
        "amount": amount_display,
        "poolAddress": pool_addr,
        "dryRun": false,
        "raw": result
    }))
}

fn resolve_from(from: Option<&str>) -> anyhow::Result<String> {
    if let Some(addr) = from {
        return Ok(addr.to_string());
    }
    onchainos::wallet_address().context("No --from address and could not resolve active wallet.")
}
