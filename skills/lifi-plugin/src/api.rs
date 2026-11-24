/// LI.FI public API client. All endpoints under https://li.quest/v1.
///
/// Spec: https://docs.li.fi/api-reference
///
/// Endpoints used:
///   GET /chains                    → list of supported chains (with chainId, key, name, etc.)
///   GET /tokens?chains=<id>        → token registry per chain
///   GET /token?chain=X&token=SYM   → resolve a single token (symbol or address) on a chain
///   GET /quote?...                 → single-step quote with calldata + approvalAddress
///   POST /advanced/routes          → multi-step route alternatives
///   GET /status?txHash=...         → status of an in-flight bridge tx
///   GET /tools                     → list of bridges + DEXs (used to validate `bridge` param)
///   GET /connections?...           → which chains/tokens have routes between them

use serde_json::Value;
use crate::config::LIFI_API_BASE;

const HTTP_TIMEOUT_SECS: u64 = 30;

/// Build a reqwest client with a sane timeout.
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .user_agent("lifi-plugin/0.1.0")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// GET helper that returns the parsed JSON. On non-2xx, the response body is
/// included in the error so the caller (and the user) can see LI.FI's error message.
async fn http_get(url: &str) -> anyhow::Result<Value> {
    let client = http_client();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("HTTP GET {} failed: {}", url, e))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| anyhow::anyhow!("read response body failed: {}", e))?;
    if !status.is_success() {
        anyhow::bail!("LI.FI API {}: {}", status, text);
    }
    serde_json::from_str::<Value>(&text)
        .map_err(|e| anyhow::anyhow!("parse LI.FI JSON failed: {} — body: {}", e, text))
}

/// POST helper.
async fn http_post(url: &str, body: &Value) -> anyhow::Result<Value> {
    let client = http_client();
    let resp = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("HTTP POST {} failed: {}", url, e))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| anyhow::anyhow!("read response body failed: {}", e))?;
    if !status.is_success() {
        anyhow::bail!("LI.FI API {}: {}", status, text);
    }
    serde_json::from_str::<Value>(&text)
        .map_err(|e| anyhow::anyhow!("parse LI.FI JSON failed: {} — body: {}", e, text))
}

/// GET /v1/chains
pub async fn get_chains() -> anyhow::Result<Value> {
    http_get(&format!("{}/chains", LIFI_API_BASE)).await
}

/// GET /v1/tokens?chains=<chainId>
/// Returns the full token map for the given chain (LI.FI returns `{tokens: {chainId: [...]}}`).
pub async fn get_tokens(chain_id: u64) -> anyhow::Result<Value> {
    http_get(&format!("{}/tokens?chains={}", LIFI_API_BASE, chain_id)).await
}

/// GET /v1/token?chain=X&token=Y — resolve a single token.
/// `token` can be a symbol (USDC), key (USDC), or address (0x…).
pub async fn get_token(chain_id: u64, token: &str) -> anyhow::Result<Value> {
    http_get(&format!(
        "{}/token?chain={}&token={}",
        LIFI_API_BASE,
        chain_id,
        urlencode(token)
    ))
    .await
}

/// Parameters for GET /v1/quote.
pub struct QuoteParams<'a> {
    pub from_chain: u64,
    pub to_chain: u64,
    pub from_token: &'a str,
    pub to_token: &'a str,
    pub from_address: &'a str,
    pub to_address: Option<&'a str>,
    pub from_amount: &'a str,    // atomic units (with decimals already applied)
    pub slippage: Option<f64>,   // decimal e.g. 0.005 = 0.5%
    pub order: Option<&'a str>,  // "FASTEST" | "CHEAPEST"
    pub deny_bridges: Vec<&'a str>,
    pub integrator: Option<&'a str>,
}

/// GET /v1/quote
pub async fn get_quote(p: &QuoteParams<'_>) -> anyhow::Result<Value> {
    let mut q: Vec<String> = vec![
        format!("fromChain={}", p.from_chain),
        format!("toChain={}", p.to_chain),
        format!("fromToken={}", urlencode(p.from_token)),
        format!("toToken={}", urlencode(p.to_token)),
        format!("fromAddress={}", urlencode(p.from_address)),
        format!("fromAmount={}", urlencode(p.from_amount)),
    ];
    if let Some(to_addr) = p.to_address {
        q.push(format!("toAddress={}", urlencode(to_addr)));
    }
    if let Some(sl) = p.slippage {
        q.push(format!("slippage={}", sl));
    }
    if let Some(order) = p.order {
        q.push(format!("order={}", urlencode(order)));
    }
    if !p.deny_bridges.is_empty() {
        for b in &p.deny_bridges {
            q.push(format!("denyBridges={}", urlencode(b)));
        }
    }
    if let Some(integ) = p.integrator {
        q.push(format!("integrator={}", urlencode(integ)));
    }
    let url = format!("{}/quote?{}", LIFI_API_BASE, q.join("&"));
    http_get(&url).await
}

/// POST /v1/advanced/routes — returns up to N routes (multi-hop alternatives) with metadata.
pub async fn post_routes(p: &QuoteParams<'_>) -> anyhow::Result<Value> {
    let mut body = serde_json::json!({
        "fromChainId": p.from_chain,
        "toChainId": p.to_chain,
        "fromTokenAddress": p.from_token,
        "toTokenAddress": p.to_token,
        "fromAddress": p.from_address,
        "fromAmount": p.from_amount,
    });
    if let Some(to_addr) = p.to_address {
        body["toAddress"] = serde_json::Value::String(to_addr.to_string());
    }
    if let Some(sl) = p.slippage {
        body["options"] = serde_json::json!({ "slippage": sl });
    }
    if let Some(order) = p.order {
        body["options"] = match body["options"].clone() {
            Value::Object(mut m) => { m.insert("order".to_string(), Value::String(order.to_string())); Value::Object(m) }
            _ => serde_json::json!({ "order": order }),
        };
    }
    http_post(&format!("{}/advanced/routes", LIFI_API_BASE), &body).await
}

/// GET /v1/status?txHash=...&fromChain=X&toChain=Y[&bridge=...]
pub async fn get_status(
    tx_hash: &str,
    from_chain: Option<u64>,
    to_chain: Option<u64>,
    bridge: Option<&str>,
) -> anyhow::Result<Value> {
    let mut q: Vec<String> = vec![format!("txHash={}", urlencode(tx_hash))];
    if let Some(c) = from_chain {
        q.push(format!("fromChain={}", c));
    }
    if let Some(c) = to_chain {
        q.push(format!("toChain={}", c));
    }
    if let Some(b) = bridge {
        q.push(format!("bridge={}", urlencode(b)));
    }
    let url = format!("{}/status?{}", LIFI_API_BASE, q.join("&"));
    http_get(&url).await
}

/// Minimal URL-encoder for the few characters we actually need to escape (no external dep).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(ch),
            _ => {
                let mut buf = [0u8; 4];
                let bytes = ch.encode_utf8(&mut buf).as_bytes().to_vec();
                for b in bytes {
                    out.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    out
}
