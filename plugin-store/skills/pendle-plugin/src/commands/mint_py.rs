use anyhow::Result;
use serde_json::Value;

use crate::api;
use crate::onchainos;

pub async fn run(
    chain_id: u64,
    token_in: &str,
    amount_in: &str,
    pt_address: &str,
    yt_address: &str,
    from: Option<&str>,
    slippage: f64,
    dry_run: bool,
    confirm: bool,
    api_key: Option<&str>,
) -> Result<Value> {
    // Validate inputs
    onchainos::validate_evm_address(token_in)?;
    onchainos::validate_evm_address(pt_address)?;
    onchainos::validate_evm_address(yt_address)?;
    onchainos::validate_amount(amount_in, "--amount-in")?;

    let wallet = from
        .map(|s| s.to_string())
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Pre-flight balance check: verify wallet holds enough token_in before calling the SDK
    if !dry_run {
        let balance = onchainos::erc20_balance_of(chain_id, token_in, &wallet).await.unwrap_or(0);
        let required: u128 = amount_in.parse().unwrap_or(0);
        if balance < required {
            anyhow::bail!(
                "Insufficient balance: wallet {} holds {} wei of token {} but {} wei is required. \
                 Acquire more before retrying.",
                wallet, balance, token_in, required
            );
        }
    }

    // Use the v2 GET endpoint with comma-separated tokensOut — the v3 POST endpoint cannot
    // classify mintPyFromToken when outputs contains both PT and YT addresses.
    // Ref: pendle-finance/pendle-examples-public hosted-sdk-demo/src/mint-py.ts
    let sdk_resp = api::sdk_convert_v2_get(
        chain_id,
        &wallet,
        token_in,
        amount_in,
        &format!("{},{}", pt_address, yt_address),
        slippage,
        api_key,
    )
    .await?;

    let (calldata, router_to) = api::extract_sdk_calldata(&sdk_resp)?;
    let approvals = api::extract_required_approvals(&sdk_resp);
    let expected_py_out = api::extract_amount_out(&sdk_resp);

    // Preview gate: show SDK quote without executing
    if !confirm && !dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "note": "Preview — add --confirm to execute on-chain.",
            "operation": "mint-py",
            "chain_id": chain_id,
            "token_in": token_in,
            "amount_in": amount_in,
            "pt_address": pt_address,
            "yt_address": yt_address,
            "expected_py_out": expected_py_out,
            "router": router_to,
            "calldata": calldata,
            "wallet": wallet,
            "required_approvals": approvals.len(),
        }));
    }

    let amount_in_wei: u128 = amount_in.parse().map_err(|_| anyhow::anyhow!("Failed to parse amount-in: '{}'", amount_in))?;

    let mut approve_hashes: Vec<String> = Vec::new();
    for (token_addr, spender) in &approvals {
        let approve_result = onchainos::erc20_approve(
            chain_id,
            token_addr,
            spender,
            amount_in_wei,
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
        "operation": "mint-py",
        "chain_id": chain_id,
        "token_in": token_in,
        "amount_in": amount_in,
        "pt_address": pt_address,
        "yt_address": yt_address,
        "expected_py_out": expected_py_out,
        "router": router_to,
        "calldata": calldata,
        "wallet": wallet,
        "approve_txs": approve_hashes,
        "tx_hash": tx_hash,
        "dry_run": dry_run
    }))
}
