//! Minimal JSON-RPC eth_call helpers for Arbitrum (read-only EVM queries).

/// Arbitrum One RPC. Must serve `eth_getTransactionReceipt`.
///
/// The previous default (publicnode) answers that method with
/// `-32602 Archive requests require a personal token` even for a tx in the
/// latest block, while still serving `eth_call` / `eth_getBalance`. Every
/// confirmation wait therefore burned its full timeout and reported a
/// misleading "timed out" for a tx that had already succeeded.
pub const ARBITRUM_RPC: &str = "https://arb1.arbitrum.io/rpc";

/// Arbitrum RPC endpoint with a test-injectable override.
///
/// Only the pre-authorization balance lookup on the order path reads this; the other
/// call sites keep using the constant directly, since nothing needs to intercept them.
/// Unset — the production case — this returns the constant unchanged.
pub fn arbitrum_rpc() -> &'static str {
    static OVERRIDE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    OVERRIDE
        .get_or_init(|| {
            if !crate::config::test_overrides_enabled() {
                return None;
            }
            std::env::var("HYPERLIQUID_TEST_ARBITRUM_RPC")
                .ok()
                .filter(|v| !v.is_empty())
        })
        .as_deref()
        .unwrap_or(ARBITRUM_RPC)
}

/// Pad a 20-byte Ethereum address to 32-byte ABI encoding.
pub fn pad_address(addr: &str) -> String {
    let a = addr.trim_start_matches("0x");
    format!("{:0>64}", a)
}

/// Pad a u128 value to 32-byte ABI encoding.
pub fn pad_u256(val: u128) -> String {
    format!("{:064x}", val)
}

/// eth_call helper: sends a JSON-RPC eth_call to the given RPC URL.
async fn eth_call(rpc: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"]
    });
    let resp: serde_json::Value = client
        .post(rpc)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    let result = resp["result"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_call failed: {:?}", resp["error"]))?
        .to_string();
    Ok(result)
}

/// Query ERC-20 balanceOf(address) → u128 (token units).
pub async fn erc20_balance(token: &str, owner: &str, rpc: &str) -> anyhow::Result<u128> {
    // balanceOf(address) selector: 0x70a08231
    let data = format!("0x70a08231{}", pad_address(owner));
    let hex = eth_call(rpc, token, &data).await?;
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.is_empty() || trimmed == "0".repeat(trimmed.len()).as_str() {
        return Ok(0);
    }
    let val = u128::from_str_radix(&trimmed[trimmed.len().saturating_sub(32)..], 16)
        .map_err(|e| anyhow::anyhow!("erc20_balance: malformed hex response '{}': {}", trimmed, e))?;
    Ok(val)
}

/// Query ERC-2612 nonces(address) → u64 (permit nonce for signing).
pub async fn usdc_permit_nonce(token: &str, owner: &str, rpc: &str) -> anyhow::Result<u64> {
    // nonces(address) selector: 0x7ecebe00
    let data = format!("0x7ecebe00{}", pad_address(owner));
    let hex = eth_call(rpc, token, &data).await?;
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.is_empty() {
        return Ok(0);
    }
    let val = u64::from_str_radix(&trimmed[trimmed.len().saturating_sub(16)..], 16)
        .map_err(|e| anyhow::anyhow!("usdc_permit_nonce: malformed hex response '{}': {}", trimmed, e))?;
    Ok(val)
}

/// Query ERC-20 allowance(owner, spender) → u128.
pub async fn erc20_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc: &str,
) -> anyhow::Result<u128> {
    // allowance(address,address) selector: 0xdd62ed3e
    let data = format!("0xdd62ed3e{}{}", pad_address(owner), pad_address(spender));
    let hex = eth_call(rpc, token, &data).await?;
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.is_empty() {
        return Ok(0);
    }
    let val = u128::from_str_radix(&trimmed[trimmed.len().saturating_sub(32)..], 16)
        .map_err(|e| anyhow::anyhow!("erc20_allowance: malformed hex response '{}': {}", trimmed, e))?;
    Ok(val)
}

/// Parse a hex or decimal wei string into u128.
///
/// Returns an error if the string is non-empty but cannot be decoded —
/// a malformed value would silently set ETH value to 0, skipping required msg.value.
pub fn parse_wei(raw: &str) -> anyhow::Result<u128> {
    let s = raw.trim();
    if s.is_empty() || s == "0x0" || s == "0" {
        // 0 is legitimate: no ETH value attached to the call
        return Ok(0);
    }
    if let Some(hex) = s.strip_prefix("0x") {
        u128::from_str_radix(hex, 16)
            .map_err(|e| anyhow::anyhow!("parse_wei: malformed hex '{}': {}", s, e))
    } else {
        s.parse::<u128>()
            .map_err(|e| anyhow::anyhow!("parse_wei: malformed decimal '{}': {}", s, e))
    }
}

/// Query native ETH/HYPE balance via eth_getBalance.
pub async fn eth_native_balance(address: &str, rpc: &str) -> anyhow::Result<u128> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp: serde_json::Value = client
        .post(rpc)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getBalance",
            "params": [address, "latest"]
        }))
        .send()
        .await?
        .json()
        .await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_getBalance error: {}", err);
    }
    let hex = resp["result"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_getBalance: missing 'result' field in response"))?;
    let wei = u128::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|e| anyhow::anyhow!("eth_getBalance: malformed hex '{}': {}", hex, e))?;
    Ok(wei)
}

/// Poll for a transaction receipt until mined or timeout.
///
/// `Ok(true)`  — mined, status 0x1
/// `Ok(false)` — mined, status 0x0 (reverted)
/// `Err(why)`  — no receipt was ever observed; `why` says what actually went
///               wrong so the caller does not have to report a bare "timed out"
///               for a transaction that may well have succeeded.
///
/// Bails out immediately when the endpoint refuses the method outright (a
/// JSON-RPC error, or a non-retryable HTTP status) instead of retrying until the
/// deadline: retrying a refusal only wastes the whole timeout and hides why.
pub async fn wait_tx_mined(tx_hash: &str, rpc: &str) -> Result<bool, String> {
    const ATTEMPTS: u32 = 30;
    const INTERVAL_SECS: u64 = 2;
    let client = reqwest::Client::new();
    let mut last_err: Option<String> = None;

    for _ in 0..ATTEMPTS {
        tokio::time::sleep(std::time::Duration::from_secs(INTERVAL_SECS)).await;
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });
        let resp = match client.post(rpc).json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(format!("request failed: {}", e));
                continue;
            }
        };
        let status_code = resp.status().as_u16();
        let text = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                last_err = Some(format!("body read failed: {}", e));
                continue;
            }
        };
        if !(200..300).contains(&status_code) {
            let msg = format!("HTTP {}: {}", status_code, snippet(&text));
            // 408/429/5xx may clear on retry; anything else is a refusal.
            if status_code == 408 || status_code == 429 || (500..600).contains(&status_code) {
                last_err = Some(msg);
                continue;
            }
            return Err(format!(
                "{} — this RPC will not serve eth_getTransactionReceipt", msg
            ));
        }
        let v: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                last_err = Some(format!("non-JSON response: {}", snippet(&text)));
                continue;
            }
        };
        if let Some(err) = v.get("error").filter(|e| !e.is_null()) {
            return Err(format!(
                "RPC error {}: {} — this RPC will not serve eth_getTransactionReceipt",
                err["code"].as_i64().unwrap_or(0),
                err["message"].as_str().unwrap_or("")
            ));
        }
        match v["result"]["status"].as_str() {
            Some("0x1") => return Ok(true),
            Some("0x0") => return Ok(false),
            _ => {} // null result: not mined yet
        }
    }
    Err(match last_err {
        Some(why) => format!(
            "no receipt after {}s — last RPC failure: {}",
            ATTEMPTS as u64 * INTERVAL_SECS, why
        ),
        None => format!(
            "no receipt after {}s — the RPC kept reporting it as pending",
            ATTEMPTS as u64 * INTERVAL_SECS
        ),
    })
}

fn snippet(body: &str) -> String {
    let t = body.trim();
    if t.is_empty() {
        return "<empty body>".to_string();
    }
    t.chars().take(160).collect()
}
