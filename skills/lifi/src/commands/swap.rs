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
        137 => "https://polygon-mainnet-rpc.publicnode.com",
        10 => "https://optimism-rpc.publicnode.com",
        56 => "https://bsc-rpc.publicnode.com",
        43114 => "https://avalanche-c-chain-rpc.publicnode.com",
        _ => "https://ethereum.publicnode.com",
    }
}

/// Chain-specific wait time after an approval tx, based on avg block time.
fn approval_wait_secs(chain_id: u64) -> u64 {
    match chain_id {
        8453 | 42161 | 10 => 3,  // Base, Arbitrum, Optimism — ~2s blocks
        137 | 56 | 43114 => 6,   // Polygon, BSC, Avalanche — ~2-3s blocks
        1 => 20,                  // Ethereum — ~12s blocks, wait for 1-2 confirms
        _ => 10,
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
    confirm: bool,
    force: bool,
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

    // Preview gate: fetch quote and show details without broadcasting unless --confirm
    if !confirm {
        let placeholder = "0x87fb0647faabea33113eaf1d80d67acb1c491b90";
        let wallet_addr = from.unwrap_or(placeholder);
        let quote_resp = api::get_quote(from_chain, to_chain, from_token, to_token, amount, wallet_addr, slippage).await;
        if let Ok(ref quote) = quote_resp {
            return Ok(serde_json::json!({
                "ok": true,
                "preview": true,
                "message": "Transaction preview — add --confirm to broadcast",
                "fromChain": from_chain,
                "toChain": to_chain,
                "fromToken": quote["action"]["fromToken"]["symbol"],
                "toToken": quote["action"]["toToken"]["symbol"],
                "fromAmount": quote["estimate"]["fromAmount"],
                "toAmount": quote["estimate"]["toAmount"],
                "tool": quote["toolDetails"]["key"],
                "toolName": quote["toolDetails"]["name"],
                "feeCosts": quote["estimate"]["feeCosts"],
                "gasCosts": quote["estimate"]["gasCosts"]
            }));
        }
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "message": "Transaction preview — add --confirm to broadcast (quote fetch failed)",
            "fromChain": from_chain,
            "toChain": to_chain,
            "fromToken": from_token,
            "toToken": to_token,
            "amount": amount
        }));
    }

    // Resolve wallet address (after dry_run and confirm guards)
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
    let to_addr = tx_req["to"].as_str().unwrap_or(LIFI_DIAMOND);

    // Validate that the target contract is the expected LiFiDiamond address
    if !to_addr.eq_ignore_ascii_case(LIFI_DIAMOND) {
        anyhow::bail!(
            "Security check failed: LI.FI API returned unexpected contract address '{}'. \
             Expected LiFiDiamond ({}). Aborting to protect your funds.",
            to_addr, LIFI_DIAMOND
        );
    }

    // Parse native ETH value from hex — use u128 to avoid overflow on large values
    let value_hex = tx_req["value"].as_str().unwrap_or("0x0");
    let value_clean = value_hex.trim_start_matches("0x");
    let value_wei = u128::from_str_radix(value_clean, 16).unwrap_or(0);

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
            let approve_result = onchainos::erc20_approve(
                from_chain,
                from_token_addr,
                approval_address,
                required_amount,
                Some(&wallet),
                false,
            ).await?;

            let approve_hash = onchainos::extract_tx_hash(&approve_result);
            eprintln!("Approve tx: {}", approve_hash);

            // Wait for approve to confirm — use chain-specific block time
            let wait_secs = approval_wait_secs(from_chain);
            tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
        }
    }

    // Step 3: Submit bridge/swap tx
    // Pass --force only when the user explicitly passed --force to lifi.
    // Without --force, onchainos will run its own risk checks; if it returns a
    // risk warning the call will fail and the user must re-run with --force after
    // reviewing the warning. Never auto-escalate silently.
    let amt = if value_wei > 0 { Some(value_wei) } else { None };

    let result = onchainos::wallet_contract_call(
        from_chain,
        to_addr,
        calldata,
        Some(&wallet),
        amt,
        false,
        force, // user must pass --force explicitly to bypass onchainos risk checks
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
