use clap::Args;
use serde_json::{json, Value};

use crate::api::{self, QuoteParams};
use crate::config::{is_native_token, parse_chain, supported_chains_help, NATIVE_TOKEN_SENTINEL};
use crate::onchainos::resolve_wallet;
use crate::rpc::fmt_token_amount;

#[derive(Args)]
pub struct RoutesArgs {
    /// Source chain (id or key)
    #[arg(long)]
    pub from_chain: String,
    /// Destination chain (id or key)
    #[arg(long)]
    pub to_chain: String,
    /// Source token (symbol or 0x address)
    #[arg(long)]
    pub from_token: String,
    /// Destination token (symbol or 0x address)
    #[arg(long)]
    pub to_token: String,
    /// Human-readable amount
    #[arg(long, allow_hyphen_values = true)]
    pub amount: String,
    /// Override sender (default: onchainos wallet)
    #[arg(long)]
    pub from_address: Option<String>,
    /// Receiver (default: sender)
    #[arg(long)]
    pub to_address: Option<String>,
    /// Slippage as percent (default 0.5)
    #[arg(long, default_value = "0.5")]
    pub slippage_pct: f64,
    /// Route preference: FASTEST or CHEAPEST
    #[arg(long, default_value = "FASTEST")]
    pub order: String,
    /// Limit how many routes to display (default 5)
    #[arg(long, default_value = "5")]
    pub limit: usize,
}

pub async fn run(args: RoutesArgs) -> anyhow::Result<()> {
    let from_chain = match parse_chain(&args.from_chain) {
        Some(c) => c,
        None => return print_err(
            &format!("Unsupported source chain '{}'", args.from_chain),
            "UNSUPPORTED_CHAIN",
            &format!("Use one of: {}", supported_chains_help()),
        ),
    };
    let to_chain = match parse_chain(&args.to_chain) {
        Some(c) => c,
        None => return print_err(
            &format!("Unsupported destination chain '{}'", args.to_chain),
            "UNSUPPORTED_CHAIN",
            &format!("Use one of: {}", supported_chains_help()),
        ),
    };

    let order = args.order.to_uppercase();
    if order != "FASTEST" && order != "CHEAPEST" {
        return print_err(
            &format!("--order must be FASTEST or CHEAPEST (got '{}')", args.order),
            "INVALID_ARGUMENT",
            "Use --order FASTEST or --order CHEAPEST",
        );
    }

    let from_addr = match args.from_address.clone() {
        Some(a) => a,
        None => match resolve_wallet(from_chain.id) {
            Ok(a) => a,
            Err(e) => return print_err(
                &format!("Could not resolve wallet on chain {}: {:#}", from_chain.id, e),
                "WALLET_NOT_FOUND",
                "Pass --from-address or run `onchainos wallet addresses`.",
            ),
        },
    };

    let (from_token_addr, from_decimals, from_symbol) =
        match resolve_token(from_chain.id, &args.from_token, from_chain.native_symbol).await {
            Ok(t) => t,
            Err(e) => return print_err(
                &format!("from_token '{}' on chain {}: {:#}", args.from_token, from_chain.key, e),
                "TOKEN_NOT_FOUND",
                "Pass the 0x… contract address or use `tokens --chain X --symbol Y`.",
            ),
        };
    let (to_token_addr, to_decimals, to_symbol) =
        match resolve_token(to_chain.id, &args.to_token, to_chain.native_symbol).await {
            Ok(t) => t,
            Err(e) => return print_err(
                &format!("to_token '{}' on chain {}: {:#}", args.to_token, to_chain.key, e),
                "TOKEN_NOT_FOUND",
                "Pass the 0x… contract address or use `tokens --chain X --symbol Y`.",
            ),
        };

    let amount_raw = match human_to_atomic(&args.amount, from_decimals) {
        Ok(s) => s,
        Err(e) => return print_err(
            &format!("Invalid amount '{}': {}", args.amount, e),
            "INVALID_ARGUMENT",
            "Pass a positive number, e.g. --amount 100 or --amount 0.001",
        ),
    };

    let qp = QuoteParams {
        from_chain: from_chain.id,
        to_chain: to_chain.id,
        from_token: &from_token_addr,
        to_token: &to_token_addr,
        from_address: &from_addr,
        to_address: args.to_address.as_deref(),
        from_amount: &amount_raw,
        slippage: Some(args.slippage_pct / 100.0),
        order: Some(&order),
        deny_bridges: vec![],
        integrator: Some("lifi-plugin"),
    };

    let resp = match api::post_routes(&qp).await {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("{:#}", e);
            let code = if msg.contains("404") { "NO_ROUTE_AVAILABLE" }
                else if msg.contains("400") { "INVALID_ROUTES_REQUEST" }
                else { "API_ERROR" };
            return print_err(&msg, code, "Verify chain/token/amount; some pairs may have no route.");
        }
    };

    let routes = resp["routes"].as_array().cloned().unwrap_or_default();
    let total = routes.len();
    let trimmed: Vec<Value> = routes.into_iter().take(args.limit).enumerate().map(|(i, r)| {
        let from_amt: u128 = r["fromAmount"].as_str().unwrap_or("0").parse().unwrap_or(0);
        let to_amt: u128 = r["toAmount"].as_str().unwrap_or("0").parse().unwrap_or(0);
        let to_amt_min: u128 = r["toAmountMin"].as_str().unwrap_or("0").parse().unwrap_or(0);
        let steps = r["steps"].as_array().map(|s| s.len()).unwrap_or(0);
        let tools: Vec<String> = r["steps"].as_array().map(|arr| {
            arr.iter().filter_map(|s| s.get("tool").and_then(|t| t.as_str()).map(|s| s.to_string())).collect()
        }).unwrap_or_default();
        json!({
            "rank": i,
            "from_amount": fmt_token_amount(from_amt, from_decimals),
            "from_amount_raw": from_amt.to_string(),
            "from_amount_usd": r.get("fromAmountUSD").cloned().unwrap_or(Value::Null),
            "to_amount": fmt_token_amount(to_amt, to_decimals),
            "to_amount_raw": to_amt.to_string(),
            "to_amount_usd": r.get("toAmountUSD").cloned().unwrap_or(Value::Null),
            "to_amount_min": fmt_token_amount(to_amt_min, to_decimals),
            "to_amount_min_raw": to_amt_min.to_string(),
            "execution_duration_seconds": r.get("executionDuration").cloned().unwrap_or(Value::Null),
            "gas_cost_usd": r.get("gasCostUSD").cloned().unwrap_or(Value::Null),
            "step_count": steps,
            "tools": tools,
            "id": r.get("id").cloned().unwrap_or(Value::Null),
        })
    }).collect();

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "from": { "chain": from_chain.key, "token": from_symbol },
        "to":   { "chain": to_chain.key,   "token": to_symbol   },
        "total_routes": total,
        "shown": trimmed.len(),
        "routes": trimmed,
        "tip": "The first route is the optimal pick under the given --order. Use `quote` for ready-to-execute calldata.",
    }))?);
    Ok(())
}

// ── shared mini-helpers (copied to keep each command self-contained) ──────────

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}

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

async fn resolve_token(
    chain_id: u64,
    user_input: &str,
    native_symbol: &str,
) -> anyhow::Result<(String, u32, String)> {
    let trimmed = user_input.trim();
    let upper = trimmed.to_uppercase();
    if is_native_token(trimmed)
        || upper == native_symbol
        || upper == "ETH" || upper == "BNB" || upper == "MATIC" || upper == "POL"
        || upper == "NATIVE"
    {
        return Ok((NATIVE_TOKEN_SENTINEL.to_string(), 18, native_symbol.to_string()));
    }
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
