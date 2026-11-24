use anyhow::Context;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::calldata;
use crate::config::{CHAIN_ID, CHAIN_NAME, DEADLINE_SECS, RPC_URL, SWAP_ROUTER, WMATIC};
use crate::onchainos;
use crate::rpc;

pub async fn run(
    chain_id: u64,
    token_in: &str,
    token_out: &str,
    amount: f64,
    slippage: f64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // --- Validation ---
    if amount <= 0.0 {
        return Err(anyhow::anyhow!("Amount must be greater than 0"));
    }
    if slippage < 0.0 || slippage > 50.0 {
        return Err(anyhow::anyhow!(
            "Slippage must be between 0 and 50 (got {}%)",
            slippage
        ));
    }

    // --- Resolve tokens ---
    let (in_addr, in_decimals) = onchainos::resolve_token(token_in, chain_id)
        .await
        .with_context(|| format!("Failed to resolve tokenIn '{}'", token_in))?;

    let (out_addr, out_decimals) = onchainos::resolve_token(token_out, chain_id)
        .await
        .with_context(|| format!("Failed to resolve tokenOut '{}'", token_out))?;

    let in_symbol = display_symbol(token_in, &in_addr);
    let out_symbol = display_symbol(token_out, &out_addr);

    // --- Resolve wallet address ---
    let wallet = match from {
        Some(f) => f.to_lowercase(),
        None => onchainos::wallet_address(chain_id)
            .with_context(|| "Could not determine wallet address")?,
    };

    // --- Convert amount to raw units ---
    let in_scale = 10u128.pow(in_decimals as u32);
    let amount_in_raw = (amount * in_scale as f64) as u128;

    if amount_in_raw == 0 {
        return Err(anyhow::anyhow!(
            "Amount {} is too small for {} (decimals: {})",
            amount,
            in_symbol,
            in_decimals
        ));
    }

    // --- Get quote ---
    let amount_out_raw =
        rpc::quote_exact_input_single(&in_addr, &out_addr, amount_in_raw, RPC_URL)
            .await
            .with_context(|| {
                format!(
                    "Quote failed for {}/{} — pool may not exist on QuickSwap V3",
                    in_symbol, out_symbol
                )
            })?;

    // Guard: if quote returns 0 output, the amount is too small to route
    if amount_out_raw == 0 {
        return Err(anyhow::anyhow!(
            "Amount {:.6} {} is too small — Quoter returned 0 output. \
             Try a larger amount (minimum ~0.01 {} recommended).",
            amount, in_symbol, in_symbol
        ));
    }

    // --- Compute amountOutMin with slippage ---
    let slippage_factor = 1.0 - (slippage / 100.0);
    let amount_out_min_raw = (amount_out_raw as f64 * slippage_factor) as u128;

    let out_scale = 10u128.pow(out_decimals as u32);
    let amount_out_human = amount_out_raw as f64 / out_scale as f64;
    let amount_out_min_human = amount_out_min_raw as f64 / out_scale as f64;

    // --- Deadline ---
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let deadline = now + DEADLINE_SECS;

    // Is this a native MATIC input? (tokenIn resolves to WMATIC but user supplied MATIC/POL)
    let is_native_matic = {
        let upper = token_in.trim().to_uppercase();
        upper == "MATIC" || upper == "POL"
    };

    // Dry-run preview: show all planned steps without broadcasting
    if dry_run {
        let mut steps = Vec::new();

        if is_native_matic {
            steps.push(serde_json::json!({
                "step": 1,
                "action": "wrap",
                "description": format!("Wrap {:.6} MATIC → WMATIC (deposit)", amount),
                "contract": WMATIC,
                "value_wei": amount_in_raw.to_string()
            }));
        }

        let approve_data = calldata::encode_erc20_approve(SWAP_ROUTER, amount_in_raw);
        steps.push(serde_json::json!({
            "step": if is_native_matic { 2 } else { 1 },
            "action": "approve",
            "description": format!("Approve SwapRouter to spend {:.6} {}", amount, in_symbol),
            "contract": in_addr,
            "spender": SWAP_ROUTER,
            "calldata": &approve_data[..approve_data.len().min(66)]
        }));

        let swap_data = calldata::encode_exact_input_single(
            &in_addr,
            &out_addr,
            &wallet,
            deadline,
            amount_in_raw,
            amount_out_min_raw,
        );
        steps.push(serde_json::json!({
            "step": if is_native_matic { 3 } else { 2 },
            "action": "swap",
            "description": format!("Swap {:.6} {} → min {:.6} {} on QuickSwap V3",
                amount, in_symbol, amount_out_min_human, out_symbol),
            "contract": SWAP_ROUTER,
            "calldata": &swap_data[..swap_data.len().min(66)]
        }));

        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "message": "Dry-run preview. Pass --confirm to execute on-chain.",
            "wallet": wallet,
            "tokenIn": in_symbol,
            "tokenOut": out_symbol,
            "amountIn": format!("{:.6}", amount),
            "amountOutEstimated": format!("{:.6}", amount_out_human),
            "amountOutMinimum": format!("{:.6}", amount_out_min_human),
            "slippage": format!("{}%", slippage),
            "deadline": deadline,
            "chain": CHAIN_NAME,
            "steps": steps
        }));
    }

    // --- Live execution ---
    let mut tx_log: Vec<Value> = Vec::new();

    // Pre-flight: check wallet balance for ERC-20 tokens (not MATIC wraps — native balance checked separately)
    if !is_native_matic {
        let balance = rpc::get_erc20_balance(&in_addr, &wallet, RPC_URL)
            .await
            .unwrap_or(0);
        if balance < amount_in_raw {
            let in_scale = 10u128.pow(in_decimals as u32);
            return Err(anyhow::anyhow!(
                "Insufficient {} balance: need {:.6}, have {:.6}. \
                 Check the token address — you may have USDC.e (0x2791...) instead of native USDC (0x3c49...).",
                in_symbol,
                amount,
                balance as f64 / in_scale as f64
            ));
        }
    }

    // Step 1: Wrap MATIC → WMATIC if needed
    if is_native_matic {
        let wrap_data = calldata::encode_wmatic_deposit();
        eprintln!("[1/3] Wrapping {:.6} MATIC → WMATIC...", amount);

        let wrap_result = onchainos::wallet_contract_call_with_value(
            CHAIN_ID,
            WMATIC,
            &wrap_data,
            Some(&wallet),
            amount_in_raw,
            false,
        )
        .context("WMATIC wrap (deposit) failed")?;

        let wrap_hash = extract_tx_hash(&wrap_result);
        tx_log.push(serde_json::json!({
            "step": "wrap",
            "txHash": wrap_hash
        }));

        if let Some(hash) = &wrap_hash {
            eprintln!("  Waiting for wrap tx {}...", hash);
            rpc::wait_for_tx(RPC_URL, hash)
                .await
                .context("WMATIC wrap tx did not confirm")?;
            eprintln!("  Wrap confirmed.");
        }
    }

    // Step 2: Check and approve allowance
    let current_allowance = rpc::get_allowance(&in_addr, &wallet, SWAP_ROUTER, RPC_URL)
        .await
        .unwrap_or(0);

    if current_allowance < amount_in_raw {
        let step_num = if is_native_matic { 2 } else { 1 };
        eprintln!(
            "[{}/{}] Approving SwapRouter to spend {} {}...",
            step_num,
            if is_native_matic { 3 } else { 2 },
            amount,
            in_symbol
        );
        // Approve exact operation amount (scoped — not unlimited)
        let approve_data = calldata::encode_erc20_approve(SWAP_ROUTER, amount_in_raw);
        let approve_result = onchainos::wallet_contract_call(
            CHAIN_ID,
            &in_addr,
            &approve_data,
            Some(&wallet),
            false,
        )
        .context("ERC-20 approval failed")?;

        let approve_hash = extract_tx_hash(&approve_result);
        tx_log.push(serde_json::json!({
            "step": "approve",
            "txHash": approve_hash
        }));

        if let Some(hash) = &approve_hash {
            eprintln!("  Waiting for approve tx {}...", hash);
            rpc::wait_for_tx(RPC_URL, hash)
                .await
                .context("Approval tx did not confirm")?;
            eprintln!("  Approval confirmed.");
        }
    } else {
        eprintln!("  Allowance sufficient — skipping approve.");
        tx_log.push(serde_json::json!({
            "step": "approve",
            "skipped": true,
            "reason": "existing allowance sufficient"
        }));
    }

    // Step 3: Swap
    let swap_step = if is_native_matic { 3 } else { 2 };
    eprintln!(
        "[{}/{}] Swapping {:.6} {} → {} on QuickSwap V3...",
        swap_step,
        swap_step,
        amount,
        in_symbol,
        out_symbol
    );

    let swap_data = calldata::encode_exact_input_single(
        &in_addr,
        &out_addr,
        &wallet,
        deadline,
        amount_in_raw,
        amount_out_min_raw,
    );

    let swap_result = onchainos::wallet_contract_call(
        CHAIN_ID,
        SWAP_ROUTER,
        &swap_data,
        Some(&wallet),
        false,
    )
    .context("Swap transaction failed")?;

    let swap_hash = extract_tx_hash(&swap_result);
    let explorer_link = swap_hash.as_ref().map(|h| {
        format!("https://polygonscan.com/tx/{}", h)
    });

    tx_log.push(serde_json::json!({
        "step": "swap",
        "txHash": swap_hash,
        "explorerLink": explorer_link
    }));

    Ok(serde_json::json!({
        "ok": true,
        "tokenIn": in_symbol,
        "tokenOut": out_symbol,
        "amountIn": format!("{:.6}", amount),
        "amountOutEstimated": format!("{:.6}", amount_out_human),
        "amountOutMinimum": format!("{:.6}", amount_out_min_human),
        "slippage": format!("{}%", slippage),
        "wallet": wallet,
        "chain": CHAIN_NAME,
        "transactions": tx_log,
        "explorerLink": explorer_link
    }))
}

fn display_symbol(input: &str, addr: &str) -> String {
    if input.starts_with("0x") && input.len() == 42 {
        // Shorten address for display: 0x2791...4174
        return format!("0x{}...{}", &addr[2..6], &addr[addr.len().saturating_sub(4)..]);
    }
    let upper = input.trim().to_uppercase();
    if upper == "MATIC" || upper == "POL" || upper == "WPOL" {
        return "WMATIC".to_string();
    }
    upper
}

fn extract_tx_hash(result: &Value) -> Option<String> {
    // onchainos returns: {"ok":true,"data":{"txHash":"0x...","orderId":"..."}}
    if let Some(h) = result["data"]["txHash"].as_str() {
        if h.starts_with("0x") && h.len() == 66 {
            return Some(h.to_string());
        }
    }
    // Flat fallbacks for other response shapes
    for key in &["txHash", "tx_hash", "hash"] {
        if let Some(h) = result.get(key).and_then(|v| v.as_str()) {
            if h.starts_with("0x") && h.len() == 66 {
                return Some(h.to_string());
            }
        }
    }
    if let Some(h) = result.get("result").and_then(|v| v.as_str()) {
        if h.starts_with("0x") && h.len() == 66 {
            return Some(h.to_string());
        }
    }
    None
}
