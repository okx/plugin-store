use reqwest::Client;
use serde_json::Value;

const SUBGRAPH_URL: &str =
    "https://api.thegraph.com/subgraphs/name/sameepsi/quickswap-v3";

const GQL_QUERY: &str = r#"{
  "query": "{ pools(first: 20, orderBy: totalValueLockedUSD, orderDirection: desc) { id token0 { symbol } token1 { symbol } totalValueLockedUSD volumeUSD feesUSD } }"
}"#;

/// Known top QuickSwap V3 pools (fallback when subgraph is unavailable)
fn fallback_pools() -> Vec<Value> {
    vec![
        serde_json::json!({
            "pair": "WMATIC/USDC.e",
            "token0": "WMATIC",
            "token1": "USDC.e",
            "source": "hardcoded"
        }),
        serde_json::json!({
            "pair": "WETH/USDC.e",
            "token0": "WETH",
            "token1": "USDC.e",
            "source": "hardcoded"
        }),
        serde_json::json!({
            "pair": "WBTC/WETH",
            "token0": "WBTC",
            "token1": "WETH",
            "source": "hardcoded"
        }),
        serde_json::json!({
            "pair": "USDC.e/USDT",
            "token0": "USDC.e",
            "token1": "USDT",
            "source": "hardcoded"
        }),
        serde_json::json!({
            "pair": "QUICK/WMATIC",
            "token0": "QUICK",
            "token1": "WMATIC",
            "source": "hardcoded"
        }),
    ]
}

pub async fn run(limit: usize) -> anyhow::Result<Value> {
    let client = Client::new();

    let response = client
        .post(SUBGRAPH_URL)
        .header("Content-Type", "application/json")
        .body(GQL_QUERY)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let json: Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => {
                    return Ok(subgraph_unavailable_response(limit));
                }
            };

            if let Some(errors) = json.get("errors") {
                return Ok(serde_json::json!({
                    "ok": true,
                    "note": "subgraph returned errors",
                    "errors": errors,
                    "fallbackPools": &fallback_pools()[..fallback_pools().len().min(limit)]
                }));
            }

            let pools_raw = json
                .get("data")
                .and_then(|d| d.get("pools"))
                .and_then(|p| p.as_array());

            match pools_raw {
                Some(pools) => {
                    let limited: Vec<Value> = pools
                        .iter()
                        .take(limit)
                        .map(|p| {
                            let token0 = p["token0"]["symbol"].as_str().unwrap_or("?");
                            let token1 = p["token1"]["symbol"].as_str().unwrap_or("?");
                            let tvl = p["totalValueLockedUSD"].as_str().unwrap_or("0");
                            let vol = p["volumeUSD"].as_str().unwrap_or("0");
                            let fees = p["feesUSD"].as_str().unwrap_or("0");
                            let id = p["id"].as_str().unwrap_or("");

                            serde_json::json!({
                                "pair": format!("{}/{}", token0, token1),
                                "token0": token0,
                                "token1": token1,
                                "address": id,
                                "tvlUSD": format_usd(tvl),
                                "volumeUSD": format_usd(vol),
                                "feesUSD": format_usd(fees)
                            })
                        })
                        .collect();

                    Ok(serde_json::json!({
                        "ok": true,
                        "source": "subgraph",
                        "count": limited.len(),
                        "pools": limited
                    }))
                }
                None => Ok(subgraph_unavailable_response(limit)),
            }
        }
        Err(_) => Ok(subgraph_unavailable_response(limit)),
    }
}

fn subgraph_unavailable_response(limit: usize) -> Value {
    let fallback = fallback_pools();
    let limited: Vec<Value> = fallback.into_iter().take(limit).collect();
    serde_json::json!({
        "ok": true,
        "note": "subgraph unavailable — showing hardcoded top pools",
        "source": "fallback",
        "count": limited.len(),
        "pools": limited,
        "subgraphUrl": SUBGRAPH_URL
    })
}

fn format_usd(raw: &str) -> String {
    let val: f64 = raw.parse().unwrap_or(0.0);
    if val >= 1_000_000.0 {
        format!("${:.2}M", val / 1_000_000.0)
    } else if val >= 1_000.0 {
        format!("${:.2}K", val / 1_000.0)
    } else {
        format!("${:.2}", val)
    }
}
