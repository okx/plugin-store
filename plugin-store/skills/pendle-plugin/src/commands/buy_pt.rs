use anyhow::Result;
use serde_json::Value;

use crate::api::{self, SdkTokenAmount};
use crate::onchainos;

pub async fn run(
    chain_id: u64,
    token_in: &str,
    amount_in: &str,
    pt_address: &str,
    min_pt_out: &str,
    from: Option<&str>,
    slippage: f64,
    dry_run: bool,
    confirm: bool,
    api_key: Option<&str>,
) -> Result<Value> {
    // Validate inputs
    onchainos::validate_evm_address(token_in)?;
    onchainos::validate_evm_address(pt_address)?;
    onchainos::validate_amount(amount_in, "--amount-in")?;

    // Resolve receiver/sender wallet
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

    // Call Pendle Hosted SDK to generate calldata
    let sdk_resp = api::sdk_convert(
        chain_id,
        &wallet,
        vec![SdkTokenAmount {
            token: token_in.to_string(),
            amount: amount_in.to_string(),
        }],
        vec![SdkTokenAmount {
            token: pt_address.to_string(),
            amount: min_pt_out.to_string(),
        }],
        slippage,
        api_key,
    )
    .await?;

    let (calldata, router_to) = api::extract_sdk_calldata(&sdk_resp)?;
    let approvals = api::extract_required_approvals(&sdk_resp);
    let expected_pt_out = api::extract_amount_out(&sdk_resp);
    api::check_min_out(&expected_pt_out, min_pt_out, "pt")?;

    // Preview gate: show SDK quote without executing
    if !confirm && !dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "note": "Preview — add --confirm to execute on-chain.",
            "operation": "buy-pt",
            "chain_id": chain_id,
            "token_in": token_in,
            "amount_in": amount_in,
            "pt_address": pt_address,
            "expected_pt_out": expected_pt_out,
            "router": router_to,
            "calldata": calldata,
            "wallet": wallet,
            "required_approvals": approvals.len(),
        }));
    }

    let amount_in_wei: u128 = amount_in.parse().map_err(|_| anyhow::anyhow!("Failed to parse amount-in: '{}'", amount_in))?;

    let mut approve_hashes: Vec<String> = Vec::new();

    // Submit ERC-20 approvals if needed
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

    // Submit main buy-PT transaction
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
        "operation": "buy-pt",
        "chain_id": chain_id,
        "token_in": token_in,
        "amount_in": amount_in,
        "pt_address": pt_address,
        "min_pt_out": min_pt_out,
        "expected_pt_out": expected_pt_out,
        "router": router_to,
        "calldata": calldata,
        "wallet": wallet,
        "approve_txs": approve_hashes,
        "tx_hash": tx_hash,
        "dry_run": dry_run
    }))
}
