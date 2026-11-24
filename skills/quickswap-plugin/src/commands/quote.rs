use anyhow::Context;
use serde_json::Value;

use crate::config::{CHAIN_NAME, RPC_URL, WMATIC};
use crate::onchainos;
use crate::rpc;

pub async fn run(
    chain_id: u64,
    token_in: &str,
    token_out: &str,
    amount: f64,
) -> anyhow::Result<Value> {
    if amount <= 0.0 {
        return Err(anyhow::anyhow!("Amount must be greater than 0"));
    }

    // Resolve tokens
    let (in_addr, in_decimals) = onchainos::resolve_token(token_in, chain_id)
        .await
        .with_context(|| format!("Failed to resolve tokenIn '{}'", token_in))?;

    let (out_addr, out_decimals) = onchainos::resolve_token(token_out, chain_id)
        .await
        .with_context(|| format!("Failed to resolve tokenOut '{}'", token_out))?;

    // Get symbols for display
    let in_symbol = get_display_symbol(token_in, &in_addr).await;
    let out_symbol = get_display_symbol(token_out, &out_addr).await;

    // Convert amount to smallest units
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

    // Get quote from Quoter contract
    let amount_out_raw = rpc::quote_exact_input_single(&in_addr, &out_addr, amount_in_raw, RPC_URL)
        .await
        .with_context(|| {
            format!(
                "Quote failed for {}/{} — pool may not exist",
                in_symbol, out_symbol
            )
        })?;

    if amount_out_raw == 0 {
        return Err(anyhow::anyhow!(
            "Amount {:.6} {} is too small — Quoter returned 0 output. Try a larger amount.",
            amount, in_symbol
        ));
    }

    // Format output amounts
    let out_scale = 10u128.pow(out_decimals as u32);
    let amount_out = amount_out_raw as f64 / out_scale as f64;
    let price = if amount > 0.0 { amount_out / amount } else { 0.0 };

    Ok(serde_json::json!({
        "ok": true,
        "tokenIn": in_symbol,
        "tokenInAddress": in_addr,
        "tokenInDecimals": in_decimals,
        "tokenOut": out_symbol,
        "tokenOutAddress": out_addr,
        "tokenOutDecimals": out_decimals,
        "amountIn": format!("{:.6}", amount),
        "amountOut": format!("{:.6}", amount_out),
        "price": format!("{:.6}", price),
        "chain": CHAIN_NAME,
        "note": "Price is indicative. Actual swap price may differ due to price impact."
    }))
}

/// Get a display-friendly symbol: use original input if it's a readable symbol,
/// otherwise fall back to RPC symbol lookup.
async fn get_display_symbol(input: &str, addr: &str) -> String {
    let upper = input.trim().to_uppercase();
    // If it's a known symbol name (not an address), use it
    if !input.starts_with("0x") {
        // Normalize MATIC/POL/WPOL to WMATIC for display
        if upper == "MATIC" || upper == "POL" || upper == "WPOL" {
            return "WMATIC".to_string();
        }
        return input.trim().to_uppercase();
    }
    // It's an address — look up symbol from RPC; shorten address if lookup fails or returns empty
    let sym = rpc::get_erc20_symbol(addr, RPC_URL)
        .await
        .unwrap_or_default();
    if sym.is_empty() || sym == "UNKNOWN" {
        if addr.len() > 10 {
            format!("0x{}...{}", &addr[2..6], &addr[addr.len() - 4..])
        } else {
            addr.to_string()
        }
    } else {
        sym
    }
}

/// Normalize MATIC/POL to WMATIC display name
#[allow(dead_code)]
fn normalize_display_name(input: &str) -> String {
    let upper = input.to_uppercase();
    if upper == "MATIC" || upper == "POL" || upper == "WPOL" {
        return "WMATIC".to_string();
    }
    upper
}
