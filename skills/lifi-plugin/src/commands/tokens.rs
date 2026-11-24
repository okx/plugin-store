use clap::Args;
use serde_json::{json, Value};

use crate::api;
use crate::config::{parse_chain, supported_chains_help};

#[derive(Args)]
pub struct TokensArgs {
    /// Chain id (1, 42161, ...) or key (ETH, ARB, BAS, OPT, BSC, POL).
    #[arg(long)]
    pub chain: String,

    /// Filter by symbol (e.g. USDC). Returns first matching token.
    #[arg(long)]
    pub symbol: Option<String>,

    /// Limit number of returned tokens (default 50; LI.FI returns hundreds otherwise).
    #[arg(long, default_value = "50")]
    pub limit: usize,
}

pub async fn run(args: TokensArgs) -> anyhow::Result<()> {
    let chain = match parse_chain(&args.chain) {
        Some(c) => c,
        None => {
            println!(
                "{}",
                super::error_response(
                    &format!("Unsupported chain '{}'", args.chain),
                    "UNSUPPORTED_CHAIN",
                    &format!("Use one of: {}", supported_chains_help()),
                )
            );
            return Ok(());
        }
    };

    // Specific token lookup
    if let Some(sym) = &args.symbol {
        let token = match api::get_token(chain.id, sym).await {
            Ok(v) => v,
            Err(e) => {
                let msg = format!("{:#}", e);
                // LI.FI returns "404" or "1003" (code) or "Could not find token" body
                // for unknown symbols. Map all of those to TOKEN_NOT_FOUND.
                // LI.FI returns either 404 with code 1003 ("Could not find token"),
                // or 400 with code 1011 ("Unknown token symbol") for unknown inputs.
                let code = if msg.contains("404")
                    || msg.contains("Could not find token")
                    || msg.contains("Unknown token symbol")
                    || msg.contains("\"code\":1003")
                    || msg.contains("\"code\":1011")
                {
                    "TOKEN_NOT_FOUND"
                } else {
                    "API_ERROR"
                };
                println!(
                    "{}",
                    super::error_response(
                        &msg,
                        code,
                        "Verify the symbol or pass the contract address (0x…)."
                    )
                );
                return Ok(());
            }
        };
        println!("{}", serde_json::to_string_pretty(&json!({
            "ok": true,
            "chain": { "id": chain.id, "key": chain.key, "name": chain.name },
            "token": token,
        }))?);
        return Ok(());
    }

    // Full list
    let resp = match api::get_tokens(chain.id).await {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("{:#}", e),
                    "API_ERROR",
                    "LI.FI /v1/tokens unreachable. Retry."
                )
            );
            return Ok(());
        }
    };

    let chain_id_str = chain.id.to_string();
    let token_list = resp["tokens"][&chain_id_str].as_array().cloned().unwrap_or_default();
    let total = token_list.len();
    let trimmed: Vec<Value> = token_list
        .into_iter()
        .take(args.limit)
        .map(|t| {
            json!({
                "address": t.get("address").cloned().unwrap_or(Value::Null),
                "symbol": t.get("symbol").cloned().unwrap_or(Value::Null),
                "name": t.get("name").cloned().unwrap_or(Value::Null),
                "decimals": t.get("decimals").cloned().unwrap_or(Value::Null),
                "priceUSD": t.get("priceUSD").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "chain": { "id": chain.id, "key": chain.key, "name": chain.name },
        "total": total,
        "shown": trimmed.len(),
        "tokens": trimmed,
        "note": if total > trimmed.len() { format!("Showing {} of {}; pass --limit to widen.", trimmed.len(), total) } else { "All tokens shown.".to_string() },
    }))?);
    Ok(())
}
