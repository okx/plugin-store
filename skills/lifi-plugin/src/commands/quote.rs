use clap::Args;
use serde_json::{json, Value};

use crate::api::{self, QuoteParams};
use crate::config::{is_native_token, parse_chain, supported_chains_help, NATIVE_TOKEN_SENTINEL};
use crate::onchainos::resolve_wallet;
use crate::rpc::fmt_token_amount;

#[derive(Args)]
pub struct QuoteArgs {
    /// Source chain (id or key)
    #[arg(long)]
    pub from_chain: String,
    /// Destination chain (id or key)
    #[arg(long)]
    pub to_chain: String,
    /// Source token (symbol like USDC, or 0x… address). For native ETH/BNB/MATIC pass "ETH"/"BNB"/"MATIC" or the sentinel.
    #[arg(long)]
    pub from_token: String,
    /// Destination token (symbol or 0x… address)
    #[arg(long)]
    pub to_token: String,
    /// Human-readable amount (e.g. 100 = 100 USDC). Decimals are resolved automatically.
    /// `allow_hyphen_values` so `--amount -5` reaches our validator (instead of clap eating `-5` as a flag).
    #[arg(long, allow_hyphen_values = true)]
    pub amount: String,
    /// Override sender address (defaults to onchainos wallet on the source chain)
    #[arg(long)]
    pub from_address: Option<String>,
    /// Receiver address (defaults to from_address)
    #[arg(long)]
    pub to_address: Option<String>,
    /// Slippage as a percent (default 0.5 = 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage_pct: f64,
    /// Route preference: "FASTEST" (default) or "CHEAPEST"
    #[arg(long, default_value = "FASTEST")]
    pub order: String,
    /// Bridges to exclude (comma-separated, e.g. "stargate,across")
    #[arg(long, value_delimiter = ',')]
    pub deny_bridges: Vec<String>,
}

pub async fn run(args: QuoteArgs) -> anyhow::Result<()> {
    let from_chain = match parse_chain(&args.from_chain) {
        Some(c) => c,
        None => {
            println!("{}", super::error_response(
                &format!("Unsupported source chain '{}'", args.from_chain),
                "UNSUPPORTED_CHAIN",
                &format!("Use one of: {}", supported_chains_help()),
            ));
            return Ok(());
        }
    };
    let to_chain = match parse_chain(&args.to_chain) {
        Some(c) => c,
        None => {
            println!("{}", super::error_response(
                &format!("Unsupported destination chain '{}'", args.to_chain),
                "UNSUPPORTED_CHAIN",
                &format!("Use one of: {}", supported_chains_help()),
            ));
            return Ok(());
        }
    };

    let order = args.order.to_uppercase();
    if order != "FASTEST" && order != "CHEAPEST" {
        println!("{}", super::error_response(
            &format!("--order must be FASTEST or CHEAPEST (got '{}')", args.order),
            "INVALID_ARGUMENT",
            "Use --order FASTEST or --order CHEAPEST",
        ));
        return Ok(());
    }

    if args.slippage_pct < 0.0 || args.slippage_pct > 50.0 {
        println!("{}", super::error_response(
            &format!("Slippage {}% out of range (0–50)", args.slippage_pct),
            "INVALID_ARGUMENT",
            "Pass slippage in percent (0.5 = 0.5%, not 0.005).",
        ));
        return Ok(());
    }

    // Resolve from_address (onchainos wallet on from_chain by default).
    let from_addr = match args.from_address.clone() {
        Some(a) => a,
        None => match resolve_wallet(from_chain.id) {
            Ok(a) => a,
            Err(e) => {
                println!("{}", super::error_response(
                    &format!("Could not resolve wallet on chain {}: {:#}", from_chain.id, e),
                    "WALLET_NOT_FOUND",
                    "Pass --from-address explicitly or run `onchainos wallet addresses` to verify login.",
                ));
                return Ok(());
            }
        },
    };

    // Resolve from_token → contract address + decimals (so we can convert human amount to atomic)
    let (from_token_addr, from_token_decimals, from_token_symbol) =
        match resolve_token(from_chain.id, &args.from_token, from_chain.native_symbol).await {
            Ok(t) => t,
            Err(e) => {
                println!("{}", super::error_response(
                    &format!("from_token '{}' on chain {}: {:#}", args.from_token, from_chain.key, e),
                    "TOKEN_NOT_FOUND",
                    "Pass the 0x… contract address or verify the symbol via `tokens --chain X --symbol Y`.",
                ));
                return Ok(());
            }
        };
    let (to_token_addr, to_token_decimals, to_token_symbol) =
        match resolve_token(to_chain.id, &args.to_token, to_chain.native_symbol).await {
            Ok(t) => t,
            Err(e) => {
                println!("{}", super::error_response(
                    &format!("to_token '{}' on chain {}: {:#}", args.to_token, to_chain.key, e),
                    "TOKEN_NOT_FOUND",
                    "Pass the 0x… contract address or verify the symbol via `tokens --chain X --symbol Y`.",
                ));
                return Ok(());
            }
        };

    // Convert human amount → atomic (handles decimals)
    let amount_raw = match human_to_atomic(&args.amount, from_token_decimals) {
        Ok(s) => s,
        Err(e) => {
            println!("{}", super::error_response(
                &format!("Invalid amount '{}': {}", args.amount, e),
                "INVALID_ARGUMENT",
                "Pass a positive number, e.g. --amount 100 or --amount 0.001",
            ));
            return Ok(());
        }
    };

    // Convert percent slippage to LI.FI decimal slippage
    let slippage_dec = args.slippage_pct / 100.0;

    let deny: Vec<&str> = args.deny_bridges.iter().map(|s| s.as_str()).collect();

    let qp = QuoteParams {
        from_chain: from_chain.id,
        to_chain: to_chain.id,
        from_token: &from_token_addr,
        to_token: &to_token_addr,
        from_address: &from_addr,
        to_address: args.to_address.as_deref(),
        from_amount: &amount_raw,
        slippage: Some(slippage_dec),
        order: Some(&order),
        deny_bridges: deny,
        integrator: Some("lifi-plugin"),
    };

    let resp = match api::get_quote(&qp).await {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("{:#}", e);
            let (code, suggestion) = classify_quote_error(&msg);
            println!("{}", super::error_response(&msg, code, suggestion));
            return Ok(());
        }
    };

    println!("{}", serde_json::to_string_pretty(&summarize_quote(
        &resp,
        from_chain.id,
        to_chain.id,
        from_chain.key,
        to_chain.key,
        &from_token_symbol,
        &to_token_symbol,
        from_token_decimals,
        to_token_decimals,
    ))?);
    Ok(())
}

/// Convert "100.5" + decimals=6 → "100500000".
/// Errors if not a positive number or has more than `decimals` fractional digits.
fn human_to_atomic(s: &str, decimals: u32) -> Result<String, String> {
    let f: f64 = s.parse().map_err(|_| "not a number".to_string())?;
    if f <= 0.0 || !f.is_finite() {
        return Err("must be a positive finite number".to_string());
    }
    let scaled = f * 10f64.powi(decimals as i32);
    if scaled > u128::MAX as f64 {
        return Err("amount exceeds u128".to_string());
    }
    let atomic = scaled.round() as u128;
    if atomic == 0 {
        return Err(format!("amount too small for {} decimals", decimals));
    }
    Ok(atomic.to_string())
}

/// Resolve a user-provided token expression (symbol, key, or address) to (address, decimals, symbol).
/// Native gas token shorthand handled locally to avoid an LI.FI call.
async fn resolve_token(
    chain_id: u64,
    user_input: &str,
    native_symbol: &str,
) -> anyhow::Result<(String, u32, String)> {
    let trimmed = user_input.trim();
    let upper = trimmed.to_uppercase();

    // Native gas-token shorthand: "ETH" / "BNB" / "MATIC" / "POL" / sentinel address.
    if is_native_token(trimmed)
        || upper == native_symbol
        || upper == "ETH" || upper == "BNB" || upper == "MATIC" || upper == "POL"
        || upper == "NATIVE"
    {
        // Native ETH/BNB/MATIC: 18 decimals on every chain we support
        return Ok((NATIVE_TOKEN_SENTINEL.to_string(), 18, native_symbol.to_string()));
    }

    // Any other input: ask LI.FI to resolve.
    let info = api::get_token(chain_id, trimmed).await?;
    let address = info["address"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("LI.FI did not return an address for '{}'", trimmed))?
        .to_string();
    let decimals = info["decimals"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("LI.FI did not return decimals for '{}'", trimmed))?
        as u32;
    let symbol = info["symbol"].as_str().unwrap_or(trimmed).to_string();
    Ok((address, decimals, symbol))
}

fn classify_quote_error(msg: &str) -> (&'static str, &'static str) {
    if msg.contains("404") || msg.contains("No quote available") || msg.contains("No available quote") {
        ("NO_ROUTE_AVAILABLE", "No bridge/swap route exists for this pair. Try a different token, smaller amount, or another chain.")
    } else if msg.contains("400") || msg.contains("Invalid") {
        ("INVALID_QUOTE_REQUEST", "Quote parameters rejected. Verify chain/token and amount.")
    } else if msg.contains("INSUFFICIENT_LIQUIDITY") {
        ("INSUFFICIENT_LIQUIDITY", "Pool depth is too thin for this size. Try a smaller amount.")
    } else {
        ("API_ERROR", "LI.FI quote API failed. Retry, or check connectivity.")
    }
}

fn summarize_quote(
    resp: &Value,
    from_chain_id: u64,
    to_chain_id: u64,
    from_chain_key: &str,
    to_chain_key: &str,
    from_symbol: &str,
    to_symbol: &str,
    from_decimals: u32,
    to_decimals: u32,
) -> Value {
    let estimate = &resp["estimate"];
    let tx_req = &resp["transactionRequest"];

    let from_amount_raw = estimate["fromAmount"].as_str().unwrap_or("0").to_string();
    let to_amount_raw = estimate["toAmount"].as_str().unwrap_or("0").to_string();
    let to_amount_min_raw = estimate["toAmountMin"].as_str().unwrap_or("0").to_string();

    let from_atomic = from_amount_raw.parse::<u128>().unwrap_or(0);
    let to_atomic = to_amount_raw.parse::<u128>().unwrap_or(0);
    let to_min_atomic = to_amount_min_raw.parse::<u128>().unwrap_or(0);

    let exec_seconds = estimate["executionDuration"].as_u64();

    json!({
        "ok": true,
        "tool": resp.get("tool").cloned().unwrap_or(Value::Null),
        "type": resp.get("type").cloned().unwrap_or(Value::Null),
        "from": {
            "chain": from_chain_key,
            "chain_id": from_chain_id,
            "token": from_symbol,
            "amount": fmt_token_amount(from_atomic, from_decimals),
            "amount_raw": from_amount_raw,
            "amount_usd": estimate.get("fromAmountUSD").cloned().unwrap_or(Value::Null),
        },
        "to": {
            "chain": to_chain_key,
            "chain_id": to_chain_id,
            "token": to_symbol,
            "amount": fmt_token_amount(to_atomic, to_decimals),
            "amount_raw": to_amount_raw,
            "amount_min": fmt_token_amount(to_min_atomic, to_decimals),
            "amount_min_raw": to_amount_min_raw,
            "amount_usd": estimate.get("toAmountUSD").cloned().unwrap_or(Value::Null),
        },
        "execution_duration_seconds": exec_seconds,
        "approval_address": estimate.get("approvalAddress").cloned().unwrap_or(Value::Null),
        "fee_costs": estimate.get("feeCosts").cloned().unwrap_or(Value::Array(vec![])),
        "gas_costs": estimate.get("gasCosts").cloned().unwrap_or(Value::Array(vec![])),
        "transaction_request": {
            "to": tx_req.get("to").cloned().unwrap_or(Value::Null),
            "value_hex": tx_req.get("value").cloned().unwrap_or(Value::Null),
            "chainId": tx_req.get("chainId").cloned().unwrap_or(Value::Null),
            "gas_limit_hex": tx_req.get("gasLimit").cloned().unwrap_or(Value::Null),
            "data_preview": tx_req.get("data").and_then(|d| d.as_str()).map(|s| {
                if s.len() > 20 { format!("{}...({} bytes)", &s[..20], (s.len() - 2) / 2) } else { s.to_string() }
            }).unwrap_or_default(),
        },
        "id": resp.get("id").cloned().unwrap_or(Value::Null),
        "tip": "Run `lifi-plugin bridge` with the same args + `--confirm` to execute.",
    })
}
