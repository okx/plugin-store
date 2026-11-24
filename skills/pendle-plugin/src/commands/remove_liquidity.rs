use anyhow::Result;
use serde_json::Value;

use crate::api::{self, SdkTokenAmount};
use crate::onchainos;

pub async fn run(
    chain_id: u64,
    lp_address: &str,
    lp_amount_in: &str,
    token_out: &str,
    min_token_out: &str,
    from: Option<&str>,
    slippage: f64,
    dry_run: bool,
    confirm: bool,
    api_key: Option<&str>,
) -> Result<Value> {
    // Validate inputs
    onchainos::validate_evm_address(lp_address)?;
    onchainos::validate_evm_address(token_out)?;
    onchainos::validate_amount(lp_amount_in, "--lp-amount-in")?;

    let wallet = from
        .map(|s| s.to_string())
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Pre-flight balance check: verify wallet holds enough LP tokens before calling the SDK
    if !dry_run {
        let balance = onchainos::erc20_balance_of(chain_id, lp_address, &wallet).await.unwrap_or(0);
        let required: u128 = lp_amount_in.parse().unwrap_or(0);
        if balance < required {
            anyhow::bail!(
                "Insufficient LP balance: wallet {} holds {} wei of LP token {} but {} wei is required. \
                 Acquire more before retrying.",
                wallet, balance, lp_address, required
            );
        }
    }

    // Hosted SDK routes automatically to removeLiquiditySingleToken
    let sdk_resp = api::sdk_convert(
        chain_id,
        &wallet,
        vec![SdkTokenAmount {
            token: lp_address.to_string(),
            amount: lp_amount_in.to_string(),
        }],
        vec![SdkTokenAmount {
            token: token_out.to_string(),
            amount: min_token_out.to_string(),
        }],
        slippage,
        api_key,
    )
    .await?;

    let (calldata, router_to) = api::extract_sdk_calldata(&sdk_resp)?;
    let approvals = api::extract_required_approvals(&sdk_resp);
    let expected_token_out = api::extract_amount_out(&sdk_resp);
    api::check_min_out(&expected_token_out, min_token_out, "token")?;

    // Preview gate: show SDK quote without executing
    if !confirm && !dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "note": "Preview — add --confirm to execute on-chain.",
            "operation": "remove-liquidity",
            "chain_id": chain_id,
            "lp_address": lp_address,
            "lp_amount_in": lp_amount_in,
            "token_out": token_out,
            "expected_token_out": expected_token_out,
            "router": router_to,
            "calldata": calldata,
            "wallet": wallet,
            "required_approvals": approvals.len(),
        }));
    }

    let lp_amount_wei: u128 = lp_amount_in.parse().map_err(|_| anyhow::anyhow!("Failed to parse lp-amount-in: '{}'", lp_amount_in))?;

    let mut approve_hashes: Vec<String> = Vec::new();
    for (token_addr, spender) in &approvals {
        let approve_result = onchainos::erc20_approve(
            chain_id,
            token_addr,
            spender,
            lp_amount_wei,
            Some(&wallet),
            dry_run,
        )
        .await?;
        let approve_hash = onchainos::extract_tx_hash(&approve_result)?;
        if !dry_run { onchainos::wait_for_tx(&approve_hash, onchainos::default_rpc_url(chain_id)).await; }
        approve_hashes.push(approve_hash);
    }

    let result = onchainos::wallet_contract_call(
        chain_id,
        &router_to,
        &calldata,
        Some(&wallet),
        None,
        dry_run,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result)?;

    Ok(serde_json::json!({
        "ok": true,
        "operation": "remove-liquidity",
        "chain_id": chain_id,
        "lp_address": lp_address,
        "lp_amount_in": lp_amount_in,
        "token_out": token_out,
        "min_token_out": min_token_out,
        "expected_token_out": expected_token_out,
        "router": router_to,
        "calldata": calldata,
        "wallet": wallet,
        "approve_txs": approve_hashes,
        "tx_hash": tx_hash,
        "dry_run": dry_run
    }))
}
