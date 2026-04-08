use crate::calldata;
use crate::config::{get_chain_config, get_dex_pool};

/// DexReservesResolver addresses per chain.
fn get_dex_reserves_resolver(chain_id: u64) -> &'static str {
    match chain_id {
        8453 => "0x05Bd8269A20C472b148246De20E6852091BF16Ff",
        1 => "0x05Bd8269A20C472b148246De20E6852091BF16Ff",
        42161 => "0x05Bd8269A20C472b148246De20E6852091BF16Ff",
        _ => "0x05Bd8269A20C472b148246De20E6852091BF16Ff",
    }
}

/// Encode estimateSwapIn(address dex_, bool swap0to1_, uint256 amountIn_, uint256 amountOutMin_)
/// Selector: 0xbb39e3a1
fn encode_estimate_swap_in(dex: &str, swap0to1: bool, amount_in: u128) -> String {
    let dex_clean = dex.trim_start_matches("0x");
    format!(
        "0xbb39e3a1{:0>64}{:064x}{:064x}{:064x}",
        dex_clean,
        if swap0to1 { 1u128 } else { 0u128 },
        amount_in,
        0u128, // amountOutMin = 0
    )
}

/// Get a swap quote from Fluid DEX (read-only, no wallet needed).
/// Uses DexReservesResolver.estimateSwapIn() for accurate quotes.
pub async fn run(
    token_in: &str,
    token_out: &str,
    amount_in: &str,
    chain_id: u64,
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let (pool, swap0to1) = get_dex_pool(token_in, token_out, chain_id)?;

    let in_decimals = if swap0to1 { pool.token0_decimals } else { pool.token1_decimals };
    let out_decimals = if swap0to1 { pool.token1_decimals } else { pool.token0_decimals };

    let raw_in = calldata::parse_amount(amount_in, in_decimals)?;

    // Use DexReservesResolver.estimateSwapIn for quote
    let resolver = get_dex_reserves_resolver(chain_id);
    let calldata_hex = encode_estimate_swap_in(pool.address, swap0to1, raw_in);

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": resolver, "data": calldata_hex },
            "latest"
        ],
        "id": 1
    });

    let resp: serde_json::Value = client
        .post(cfg.rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    let amount_out_raw = if let Some(result) = resp["result"].as_str() {
        crate::rpc::parse_u128_from_hex(result).unwrap_or(0)
    } else {
        eprintln!("[fluid] Quote estimation failed: {:?}", resp.get("error"));
        0u128
    };

    let amount_in_display = calldata::format_amount(raw_in, in_decimals);
    let amount_out_display = calldata::format_amount(amount_out_raw, out_decimals);

    let note = if amount_out_raw == 0 {
        "Estimation returned 0 — pool may have insufficient liquidity for this amount, or use a larger amount."
    } else {
        "Quote is an estimate via DexReservesResolver.estimateSwapIn. Actual output may vary due to price impact."
    };

    let output = serde_json::json!({
        "ok": true,
        "operation": "quote",
        "pool": pool.address,
        "dexReservesResolver": resolver,
        "tokenIn": token_in,
        "tokenOut": token_out,
        "amountIn": amount_in_display,
        "amountOut": amount_out_display,
        "amountOutRaw": amount_out_raw.to_string(),
        "swap0to1": swap0to1,
        "chainId": chain_id,
        "note": note,
        "swapInstruction": format!("fluid --chain {} swap --token-in {} --token-out {} --amount-in {}", chain_id, token_in, token_out, amount_in)
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
