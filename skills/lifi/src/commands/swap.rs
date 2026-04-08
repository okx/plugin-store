use anyhow::Result;
use serde_json::Value;

use crate::api;
use crate::config::{ETH_ADDRESS, ETH_ADDRESS2, LIFI_DIAMOND};
use crate::onchainos;

/// RPC endpoints by chain ID
fn rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "https://ethereum.publicnode.com",
        8453 => "https://base-rpc.publicnode.com",
        42161 => "https://arbitrum-one-rpc.publicnode.com",
        137 => "https://polygon-bsc-rpc.publicnode.com",
        10 => "https://optimism-rpc.publicnode.com",
        56 => "https://bsc-rpc.publicnode.com",
        43114 => "https://avalanche-c-chain-rpc.publicnode.com",
        _ => "https://ethereum.publicnode.com",
    }
}

fn is_native_token(address: &str) -> bool {
    address.eq_ignore_ascii_case(ETH_ADDRESS)
        || address.eq_ignore_ascii_case(ETH_ADDRESS2)
        || address.is_empty()
}

/// Execute a cross-chain swap or bridge via LI.FI.
/// 1. Get quote from LI.FI API
/// 2. If token is ERC-20 and allowance insufficient, send approve tx
/// 3. Submit bridge/swap tx via onchainos wallet contract-call
pub async fn execute(
    from_chain: u64,
    to_chain: u64,
    from_token: &str,
    to_token: &str,
    amount: &str,
    slippage: f64,
    from: Option<&str>,
    dry_run: bool,
) -> Result<Value> {
    // dry_run early return — show what would be sent
    if dry_run {
        // Still fetch quote to show preview
        // Use a placeholder address that LI.FI API accepts for quote estimation
        let placeholder = "0x87fb0647faabea33113eaf1d80d67acb1c491b90";
        let wallet_addr = from.unwrap_or(placeholder);
        let quote_resp = api::get_quote(from_chain, to_chain, from_token, to_token, amount, wallet_addr, slippage).await;

        if let Ok(ref quote) = quote_resp {
            let tx_req = &quote["transactionRequest"];
            let calldata = tx_req["data"].as_str().unwrap_or("");
            let selector = if calldata.len() >= 10 { &calldata[..10] } else { calldata };
            return Ok(serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "preview": {
                    "fromChain": from_chain,
                    "toChain": to_chain,
                    "fromToken": quote["action"]["fromToken"]["symbol"],
                    "toToken": quote["action"]["toToken"]["symbol"],
                    "fromAmount": quote["estimate"]["fromAmount"],
                    "toAmount": quote["estimate"]["toAmount"],
                    "tool": quote["toolDetails"]["key"],
                    "calldata_selector": selector,
                    "to": tx_req["to"],
                    "value": tx_req["value"]
                }
            }));
        }

        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "note": "Quote fetch failed in dry-run, but transaction would be submitted to LiFiDiamond"
        }));
    }

    // Resolve wallet address (after dry_run guard)
    let wallet = if let Some(f) = from {
        f.to_string()
    } else {
        onchainos::resolve_wallet(from_chain)?
    };

    // Step 1: Get quote
    let quote = api::get_quote(from_chain, to_chain, from_token, to_token, amount, &wallet, slippage).await?;

    let tx_req = &quote["transactionRequest"];
    let calldata = tx_req["data"].as_str()
        .ok_or_else(|| anyhow::anyhow!("No transactionRequest.data in quote response"))?;
    let to_addr = tx_req["to"].as_str()
        .unwrap_or(LIFI_DIAMOND);

    // Parse native ETH value from hex
    let value_hex = tx_req["value"].as_str().unwrap_or("0x0");
    let value_clean = value_hex.trim_start_matches("0x");
    let value_wei = u64::from_str_radix(value_clean, 16).unwrap_or(0);

    // Approval address from estimate (may differ from LiFiDiamond on some routes)
    let approval_address = quote["estimate"]["approvalAddress"]
        .as_str()
        .unwrap_or(LIFI_DIAMOND);

    // Get from token address for allowance check
    let from_token_addr = quote["action"]["fromToken"]["address"]
        .as_str()
        .unwrap_or("");

    // Step 2: ERC-20 approve if needed
    if !is_native_token(from_token_addr) && value_wei == 0 {
        let rpc = rpc_url(from_chain);
        let from_amount_str = quote["estimate"]["fromAmount"].as_str().unwrap_or(amount);
        let required_amount: u128 = from_amount_str.parse().unwrap_or(0);

        let current_allowance = onchainos::erc20_allowance(
            from_chain,
            from_token_addr,
            &wallet,
            approval_address,
            rpc,
        ).await;

        if current_allowance < required_amount {
            // Send approve with unlimited allowance (u128::MAX)
            let approve_result = onchainos::erc20_approve(
                from_chain,
                from_token_addr,
                approval_address,
                u128::MAX,
                Some(&wallet),
                false,
            ).await?;

            let approve_hash = onchainos::extract_tx_hash(&approve_result);
            eprintln!("Approve tx: {}", approve_hash);

            // Wait for approve to confirm before submitting main tx
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    }

    // Step 3: Submit bridge/swap tx
    // All LI.FI txs go through LiFiDiamond — use --force for DEX/bridge ops
    let amt = if value_wei > 0 { Some(value_wei) } else { None };

    let result = onchainos::wallet_contract_call(
        from_chain,
        to_addr,
        calldata,
        Some(&wallet),
        amt,
        false,
        true, // --force required for all LI.FI bridge/swap txs
    ).await?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    Ok(serde_json::json!({
        "ok": true,
        "txHash": tx_hash,
        "data": {
            "txHash": tx_hash
        },
        "details": {
            "fromChain": from_chain,
            "toChain": to_chain,
            "fromToken": quote["action"]["fromToken"]["symbol"],
            "toToken": quote["action"]["toToken"]["symbol"],
            "fromAmount": quote["estimate"]["fromAmount"],
            "toAmount": quote["estimate"]["toAmount"],
            "tool": quote["toolDetails"]["key"],
            "toolName": quote["toolDetails"]["name"]
        },
        "tracking": format!("https://scan.li.fi/tx/{}", tx_hash)
    }))
}
