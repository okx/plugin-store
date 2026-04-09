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
    api_key: Option<&str>,
) -> Result<Value> {
    let wallet = from
        .map(|s| s.to_string())
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or ensure onchainos is logged in.");
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

    let mut approve_hashes: Vec<String> = Vec::new();
    for (token_addr, spender) in &approvals {
        let approve_result = onchainos::erc20_approve(
            chain_id,
            token_addr,
            spender,
            u128::MAX,
            Some(&wallet),
            dry_run,
        )
        .await?;
        approve_hashes.push(onchainos::extract_tx_hash(&approve_result).to_string());
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

    let tx_hash = onchainos::extract_tx_hash(&result).to_string();

    Ok(serde_json::json!({
        "ok": true,
        "operation": "remove-liquidity",
        "chain_id": chain_id,
        "lp_address": lp_address,
        "lp_amount_in": lp_amount_in,
        "token_out": token_out,
        "min_token_out": min_token_out,
        "router": router_to,
        "calldata": calldata,
        "wallet": wallet,
        "approve_txs": approve_hashes,
        "tx_hash": tx_hash,
        "dry_run": dry_run
    }))
}
