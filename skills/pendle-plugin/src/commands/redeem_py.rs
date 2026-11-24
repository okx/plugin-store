use anyhow::Result;
use serde_json::Value;

use crate::api;
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
    confirm: bool,
    api_key: Option<&str>,
) -> Result<Value> {
    // Validate inputs
    onchainos::validate_evm_address(pt_address)?;
    onchainos::validate_evm_address(yt_address)?;
    onchainos::validate_evm_address(token_out)?;
    onchainos::validate_amount(pt_amount, "--pt-amount")?;
    onchainos::validate_amount(yt_amount, "--yt-amount")?;

    let wallet = from
        .map(|s| s.to_string())
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Pre-flight balance checks: verify wallet holds enough PT and YT before calling the SDK
    if !dry_run {
        let pt_required: u128 = pt_amount.parse().unwrap_or(0);
        let pt_balance = onchainos::erc20_balance_of(chain_id, pt_address, &wallet).await.unwrap_or(0);
        if pt_balance < pt_required {
            anyhow::bail!(
                "Insufficient PT balance: wallet {} holds {} wei of PT {} but {} wei is required. \
                 Acquire more before retrying.",
                wallet, pt_balance, pt_address, pt_required
            );
        }
        let yt_required: u128 = yt_amount.parse().unwrap_or(0);
        let yt_balance = onchainos::erc20_balance_of(chain_id, yt_address, &wallet).await.unwrap_or(0);
        if yt_balance < yt_required {
            anyhow::bail!(
                "Insufficient YT balance: wallet {} holds {} wei of YT {} but {} wei is required. \
                 Acquire more before retrying.",
                wallet, yt_balance, yt_address, yt_required
            );
        }
    }

    // Use the v2 GET endpoint with comma-separated tokensIn — the v3 POST endpoint cannot
    // classify redeemPyToToken when inputs contains both PT and YT addresses.
    // Ref: pendle-finance/pendle-examples-public hosted-sdk-demo/src/redeem-py.ts
    let sdk_resp = api::sdk_convert_v2_get(
        chain_id,
        &wallet,
        &format!("{},{}", pt_address, yt_address),
        &format!("{},{}", pt_amount, yt_amount),
        token_out,
        slippage,
        api_key,
    )
    .await?;

    let (calldata, router_to) = api::extract_sdk_calldata(&sdk_resp)?;
    let approvals = api::extract_required_approvals(&sdk_resp);
    let expected_token_out = api::extract_amount_out(&sdk_resp);

    // Preview gate: show SDK quote without executing
    if !confirm && !dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "note": "Preview — add --confirm to execute on-chain.",
            "operation": "redeem-py",
            "chain_id": chain_id,
            "pt_address": pt_address,
            "pt_amount": pt_amount,
            "yt_address": yt_address,
            "yt_amount": yt_amount,
            "token_out": token_out,
            "expected_token_out": expected_token_out,
            "router": router_to,
            "calldata": calldata,
            "wallet": wallet,
            "required_approvals": approvals.len(),
        }));
    }

    let pt_wei: u128 = pt_amount.parse().map_err(|_| anyhow::anyhow!("Failed to parse pt-amount: '{}'", pt_amount))?;
    let yt_wei: u128 = yt_amount.parse().map_err(|_| anyhow::anyhow!("Failed to parse yt-amount: '{}'", yt_amount))?;

    // redeemPyToToken always requires approval for both PT and YT.
    // The Pendle SDK v2 requiredApprovals field only lists PT (incomplete); we always approve
    // both explicitly. The spender is taken from the SDK response when present, otherwise
    // falls back to PENDLE_ROUTER.
    let spender = approvals.first()
        .map(|(_, s)| s.as_str())
        .unwrap_or(crate::config::PENDLE_ROUTER);
    let tokens_to_approve = [(pt_address, pt_wei), (yt_address, yt_wei)];

    let mut approve_hashes: Vec<String> = Vec::new();
    for (token_addr, approve_amount) in &tokens_to_approve {
        let approve_result = onchainos::erc20_approve(
            chain_id,
            token_addr,
            spender,
            *approve_amount,
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
        "operation": "redeem-py",
        "chain_id": chain_id,
        "pt_address": pt_address,
        "pt_amount": pt_amount,
        "yt_address": yt_address,
        "yt_amount": yt_amount,
        "token_out": token_out,
        "expected_token_out": expected_token_out,
        "router": router_to,
        "calldata": calldata,
        "wallet": wallet,
        "approve_txs": approve_hashes,
        "tx_hash": tx_hash,
        "dry_run": dry_run
    }))
}
