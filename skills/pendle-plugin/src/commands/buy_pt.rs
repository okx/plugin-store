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
    confirm: bool,
    dry_run: bool,
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
        true, // enableAggregator: true — buy/sell/liquidity operations may need DEX routing
        api_key,
    )
    .await?;

    let (calldata, router_to) = api::extract_sdk_calldata(&sdk_resp)?;
    let approvals = api::extract_required_approvals(&sdk_resp);

    // Preview gate: show what would be executed without --confirm
    if !confirm {
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "operation": "buy-pt",
            "chain_id": chain_id,
            "token_in": token_in,
            "amount_in": amount_in,
            "pt_address": pt_address,
            "min_pt_out": min_pt_out,
            "router": router_to,
            "calldata": calldata,
            "wallet": wallet,
            "required_approvals": approvals.iter().map(|(t, s)| serde_json::json!({"token": t, "spender": s})).collect::<Vec<_>>(),
            "dry_run": dry_run,
            "note": "Re-run with --confirm to execute"
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
        approve_hashes.push(onchainos::extract_tx_hash(&approve_result)?);
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
        "router": router_to,
        "calldata": calldata,
        "wallet": wallet,
        "approve_txs": approve_hashes,
        "tx_hash": tx_hash,
        "dry_run": dry_run
    }))
}
