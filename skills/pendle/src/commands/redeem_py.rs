use anyhow::Result;
use serde_json::Value;

use crate::api::{self, SdkTokenAmount};
use crate::onchainos;

pub async fn run(
    chain_id: u64,
    pt_address: &str,
    pt_amount: &str,
    yt_address: &str,
    yt_amount: &str,
    token_out: &str,
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

    // Both PT and YT as inputs; Hosted SDK routes to redeemPyToToken
    let sdk_resp = api::sdk_convert(
        chain_id,
        &wallet,
        vec![
            SdkTokenAmount {
                token: pt_address.to_string(),
                amount: pt_amount.to_string(),
            },
            SdkTokenAmount {
                token: yt_address.to_string(),
                amount: yt_amount.to_string(),
            },
        ],
        vec![SdkTokenAmount {
            token: token_out.to_string(),
            amount: "0".to_string(),
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
        approve_hashes.push(onchainos::extract_tx_hash(&approve_result)?);
    }

    let result = onchainos::wallet_contract_call(
        chain_id,
        &router_to,
        &calldata,
        Some(&wallet),
        None,
        true,
        dry_run,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result)?;

    Ok(serde_json::json!({
        "ok": true,
        "operation": "redeem-py",
        "chain_id": chain_id,
        "pt_address": pt_address,
        "pt_amount": pt_amount,
        "yt_address": yt_address,
        "yt_amount": yt_amount,
        "token_out": token_out,
        "wallet": wallet,
        "approve_txs": approve_hashes,
        "tx_hash": tx_hash,
        "dry_run": dry_run
    }))
}
