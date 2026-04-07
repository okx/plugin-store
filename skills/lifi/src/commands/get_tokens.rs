use anyhow::Result;
use serde_json::Value;

use crate::api;

pub async fn execute(chains: &str, symbol_filter: Option<&str>) -> Result<Value> {
    let resp = api::get_tokens(chains).await?;
    let tokens_map = resp["tokens"].as_object().cloned().unwrap_or_default();

    let mut all_tokens: Vec<Value> = Vec::new();
    for (chain_id, token_list) in &tokens_map {
        if let Some(arr) = token_list.as_array() {
            for token in arr {
                let sym = token["symbol"].as_str().unwrap_or("");
                if let Some(filter) = symbol_filter {
                    if !sym.eq_ignore_ascii_case(filter) {
                        continue;
                    }
                }
                all_tokens.push(serde_json::json!({
                    "chainId": chain_id.parse::<u64>().unwrap_or(0),
                    "address": token["address"],
                    "symbol": token["symbol"],
                    "name": token["name"],
                    "decimals": token["decimals"],
                    "priceUSD": token["priceUSD"]
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "ok": true,
        "total": all_tokens.len(),
        "tokens": all_tokens
    }))
}
