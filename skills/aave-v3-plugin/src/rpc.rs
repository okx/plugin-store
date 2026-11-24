use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

/// Raw JSON-RPC request/response
#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    params: Value,
    id: u64,
}

#[derive(Deserialize)]
struct RpcResponse {
    result: Option<String>,
    error: Option<Value>,
}

/// How long the approve→action waits poll before giving up.
const WAIT_TIMEOUT: Duration = Duration::from_secs(60);
/// Poll cadence. Base and Arbitrum produce blocks every ~2s.
const WAIT_INTERVAL: Duration = Duration::from_secs(2);
/// Per-request cap so one hung endpoint cannot eat the whole deadline.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Why a single RPC poll failed.
enum PollFailure {
    /// Retrying cannot help: the endpoint refuses to serve this method at all.
    Fatal(String),
    /// May succeed on a later attempt (network blip, 5xx, rate limit).
    Transient(String),
}

/// True when an RPC error means "this endpoint will not serve this method".
///
/// publicnode answers eth_getTransactionReceipt with
/// `-32602 Archive requests require a personal token` on Base and Arbitrum, and
/// hard-403s it after repeated probes. Retrying either one only burns the whole
/// deadline and then reports a misleading timeout.
fn is_endpoint_refusal(code: i64, message: &str) -> bool {
    if matches!(code, -32601 | -32602 | -32004) {
        return true;
    }
    let m = message.to_ascii_lowercase();
    [
        "personal token",
        "not supported",
        "unsupported",
        "unauthorized",
        "api key",
        "requires a",
    ]
    .iter()
    .any(|needle| m.contains(needle))
}

fn body_snippet(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "<empty body>".to_string();
    }
    let max = 200;
    if trimmed.chars().count() <= max {
        trimmed.to_string()
    } else {
        format!("{}...", trimmed.chars().take(max).collect::<String>())
    }
}

fn poll_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("failed to build HTTP client")
}

/// One RPC round-trip for a polling loop. Returns the raw `result` value — which
/// may legitimately be null, e.g. a receipt for a tx that is not mined yet — or a
/// classified failure. No failure is discarded: every one carries a reason.
async fn rpc_poll_once(
    client: &reqwest::Client,
    rpc_url: &str,
    method: &str,
    params: Value,
) -> Result<Value, PollFailure> {
    let req = json!({ "jsonrpc": "2.0", "method": method, "params": params, "id": 1 });

    let resp = client
        .post(rpc_url)
        .json(&req)
        .send()
        .await
        .map_err(|e| PollFailure::Transient(format!("{} request failed: {}", method, e)))?;

    let status = resp.status().as_u16();
    let body = resp
        .text()
        .await
        .map_err(|e| PollFailure::Transient(format!("{} body read failed: {}", method, e)))?;

    if !(200..300).contains(&status) {
        let msg = format!("{} -> HTTP {}: {}", method, status, body_snippet(&body));
        // 408/429/5xx can clear on retry; any other non-2xx means the endpoint is
        // refusing this method outright.
        return Err(
            if status == 408 || status == 429 || (500..600).contains(&status) {
                PollFailure::Transient(msg)
            } else {
                PollFailure::Fatal(msg)
            },
        );
    }

    let parsed: Value = serde_json::from_str(&body).map_err(|_| {
        PollFailure::Transient(format!(
            "{} -> non-JSON response: {}",
            method,
            body_snippet(&body)
        ))
    })?;

    if let Some(err) = parsed.get("error").filter(|e| !e.is_null()) {
        let code = err["code"].as_i64().unwrap_or(0);
        let message = err["message"].as_str().unwrap_or("").to_string();
        let msg = format!("{} -> RPC error {}: {}", method, code, message);
        return Err(if is_endpoint_refusal(code, &message) {
            PollFailure::Fatal(msg)
        } else {
            PollFailure::Transient(msg)
        });
    }

    Ok(parsed["result"].clone())
}

/// Poll eth_getTransactionReceipt until the tx is mined.
/// Returns true if the tx succeeded (status=0x1), false if it reverted.
///
/// Kept as a correct primitive, but no longer used for approve->action
/// sequencing: several public endpoints gate receipt lookups while still serving
/// eth_call, so the waits below poll the on-chain condition instead.
#[allow(dead_code)]
pub async fn wait_for_tx(rpc_url: &str, tx_hash: &str) -> anyhow::Result<bool> {
    wait_for_tx_inner(rpc_url, tx_hash, WAIT_TIMEOUT, WAIT_INTERVAL).await
}

async fn wait_for_tx_inner(
    rpc_url: &str,
    tx_hash: &str,
    timeout: Duration,
    interval: Duration,
) -> anyhow::Result<bool> {
    let client = poll_client()?;
    let deadline = Instant::now() + timeout;
    let mut last_transient: Option<String> = None;

    loop {
        let params = json!([tx_hash]);
        match rpc_poll_once(&client, rpc_url, "eth_getTransactionReceipt", params).await {
            Ok(receipt) => {
                if !receipt.is_null() {
                    let status = receipt["status"].as_str().unwrap_or("0x1");
                    return Ok(status == "0x1");
                }
            }
            Err(PollFailure::Fatal(reason)) => anyhow::bail!(
                "{} -- this endpoint will not confirm transactions. Use an RPC endpoint \
                 that serves eth_getTransactionReceipt, or check the tx on a block explorer.",
                reason
            ),
            Err(PollFailure::Transient(reason)) => last_transient = Some(reason),
        }

        if Instant::now() + interval > deadline {
            anyhow::bail!(
                "Timed out after {}s waiting for tx {} to be mined{}",
                timeout.as_secs(),
                tx_hash,
                match last_transient {
                    Some(reason) => format!(" -- last RPC failure: {}", reason),
                    None => " -- the RPC kept reporting it as pending".to_string(),
                }
            );
        }
        tokio::time::sleep(interval).await;
    }
}

/// Wait until `spender`'s allowance over `owner`'s `token` reaches `min_allowance`.
///
/// This is the condition an approve->repay/supply sequence actually depends on,
/// and it is read with eth_call, which public endpoints serve even where they
/// gate receipt lookups. Returns the observed allowance.
pub async fn wait_for_allowance(
    token_addr: &str,
    owner_addr: &str,
    spender_addr: &str,
    min_allowance: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data = encode_allowance_call(owner_addr, spender_addr)?;
    wait_for_uint_at_least(
        "allowance",
        rpc_url,
        token_addr,
        &data,
        min_allowance,
        WAIT_TIMEOUT,
        WAIT_INTERVAL,
    )
    .await
}

/// Wait until `account`'s balance of `token` reaches `min_balance`.
/// Used after an ETH->WETH wrap, for the same reason as `wait_for_allowance`.
pub async fn wait_for_erc20_balance(
    token_addr: &str,
    account: &str,
    min_balance: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data = encode_balance_of_call(account)?;
    wait_for_uint_at_least(
        "token balance",
        rpc_url,
        token_addr,
        &data,
        min_balance,
        WAIT_TIMEOUT,
        WAIT_INTERVAL,
    )
    .await
}

/// Poll a uint256-returning eth_call until it reports at least `min`.
async fn wait_for_uint_at_least(
    label: &str,
    rpc_url: &str,
    to: &str,
    data: &str,
    min: u128,
    timeout: Duration,
    interval: Duration,
) -> anyhow::Result<u128> {
    let client = poll_client()?;
    let deadline = Instant::now() + timeout;
    let mut last_transient: Option<String> = None;
    let mut last_seen: Option<u128> = None;

    loop {
        let params = json!([{ "to": to, "data": data }, "latest"]);
        match rpc_poll_once(&client, rpc_url, "eth_call", params).await {
            Ok(result) => match result.as_str() {
                Some(hex_result) => {
                    let raw = strip_0x(hex_result);
                    if raw.len() >= 64 {
                        let value = decode_u128_at(raw, 0)?;
                        last_seen = Some(value);
                        if value >= min {
                            return Ok(value);
                        }
                    } else {
                        last_transient =
                            Some(format!("eth_call -> short response '{}'", hex_result));
                    }
                }
                None => last_transient = Some("eth_call -> null result".to_string()),
            },
            Err(PollFailure::Fatal(reason)) => anyhow::bail!(
                "{} -- cannot read {} from this endpoint. Point the plugin at an RPC \
                 endpoint that serves eth_call.",
                reason,
                label
            ),
            Err(PollFailure::Transient(reason)) => last_transient = Some(reason),
        }

        if Instant::now() + interval > deadline {
            anyhow::bail!(
                "Timed out after {}s waiting for {} to reach {} (last observed: {}){}",
                timeout.as_secs(),
                label,
                min,
                match last_seen {
                    Some(v) => v.to_string(),
                    None => "never read".to_string(),
                },
                match last_transient {
                    Some(reason) => format!(" -- last RPC failure: {}", reason),
                    None => String::new(),
                }
            );
        }
        tokio::time::sleep(interval).await;
    }
}

/// Perform a raw eth_call against the given RPC endpoint.
/// `to` and `data` are hex strings (0x-prefixed).
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let req = RpcRequest {
        jsonrpc: "2.0",
        method: "eth_call",
        params: json!([
            { "to": to, "data": data },
            "latest"
        ]),
        id: 1,
    };
    let resp: RpcResponse = client
        .post(rpc_url)
        .json(&req)
        .send()
        .await
        .context("eth_call HTTP request failed")?
        .json()
        .await
        .context("eth_call response parse failed")?;

    if let Some(err) = resp.error {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    resp.result
        .ok_or_else(|| anyhow::anyhow!("eth_call returned null result"))
}

/// Resolve the Pool address by calling PoolAddressesProvider.getPool()
/// Function selector: getPool() -> 0x026b1d5f
/// Verified on-chain against Aave V3 deployments on Ethereum, Base, Polygon, Arbitrum.
/// Note: 0x0c2c3d97 (often cited as getPool() selector) is incorrect for the actual
/// deployed PoolAddressesProvider — 0x026b1d5f is the correct observed selector.
pub async fn get_pool(provider_addr: &str, rpc_url: &str) -> anyhow::Result<String> {
    // getPool() selector — verified empirically against live Aave V3 deployments
    let data = "0x026b1d5f";
    let hex_result = eth_call(rpc_url, provider_addr, data).await?;
    // Result is a 32-byte ABI-encoded address (left-padded with zeros)
    let addr = decode_address_result(&hex_result)?;
    Ok(addr)
}

#[allow(dead_code)]
/// Resolve the PoolDataProvider address by calling PoolAddressesProvider.getPoolDataProvider()
/// Function selector: getPoolDataProvider() -> 0x0e67178c
/// Verified on-chain against Aave V3 Base deployment.
pub async fn get_pool_data_provider(provider_addr: &str, rpc_url: &str) -> anyhow::Result<String> {
    // getPoolDataProvider() selector — verified empirically against live Aave V3 deployments
    let data = "0x0e67178c";
    let hex_result = eth_call(rpc_url, provider_addr, data).await?;
    let addr = decode_address_result(&hex_result)?;
    Ok(addr)
}

/// Account data returned by Pool.getUserAccountData(address)
#[derive(Debug, Clone)]
pub struct UserAccountData {
    /// Total collateral in USD base units (8 decimals)
    pub total_collateral_base: u128,
    /// Total debt in USD base units (8 decimals)
    pub total_debt_base: u128,
    /// Available borrows in USD base units (8 decimals)
    pub available_borrows_base: u128,
    /// Current liquidation threshold (basis points, e.g. 8250 = 82.5%)
    pub current_liquidation_threshold: u128,
    /// LTV (basis points)
    pub ltv: u128,
    /// Health factor scaled 1e18 (< 1e18 = liquidatable)
    pub health_factor: u128,
}

impl UserAccountData {
    /// Returns health factor as a human-readable f64
    pub fn health_factor_f64(&self) -> f64 {
        self.health_factor as f64 / 1e18
    }

    /// Returns health factor status string
    pub fn health_factor_status(&self) -> &'static str {
        let hf = self.health_factor_f64();
        if hf >= 1.1 {
            "safe"
        } else if hf >= 1.05 {
            "warning"
        } else {
            "danger"
        }
    }

    /// Returns total collateral in USD as f64
    pub fn total_collateral_usd(&self) -> f64 {
        self.total_collateral_base as f64 / 1e8
    }

    /// Returns total debt in USD as f64
    pub fn total_debt_usd(&self) -> f64 {
        self.total_debt_base as f64 / 1e8
    }

    /// Returns available borrows in USD as f64
    pub fn available_borrows_usd(&self) -> f64 {
        self.available_borrows_base as f64 / 1e8
    }
}

/// Call Pool.getUserAccountData(address user)
/// Function selector: 0xbf92857c
pub async fn get_user_account_data(
    pool_addr: &str,
    user_addr: &str,
    rpc_url: &str,
) -> anyhow::Result<UserAccountData> {
    // Encode: selector (4 bytes) + address (32 bytes, left-padded)
    let addr_bytes = parse_address(user_addr)?;
    let mut data = hex::decode("bf92857c")?;
    // Pad address to 32 bytes
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&addr_bytes);

    let data_hex = format!("0x{}", hex::encode(&data));
    let hex_result = eth_call(rpc_url, pool_addr, &data_hex).await?;

    // Result: 6 x uint256 packed (each 32 bytes = 64 hex chars)
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 * 6 {
        anyhow::bail!(
            "getUserAccountData: short response ({} hex chars, expected {})",
            raw.len(),
            64 * 6
        );
    }

    Ok(UserAccountData {
        total_collateral_base: decode_u128_at(raw, 0)?,
        total_debt_base: decode_u128_at(raw, 1)?,
        available_borrows_base: decode_u128_at(raw, 2)?,
        current_liquidation_threshold: decode_u128_at(raw, 3)?,
        ltv: decode_u128_at(raw, 4)?,
        health_factor: decode_u128_at(raw, 5)?,
    })
}

/// Get ERC-20 token balance: token.balanceOf(account)
/// Function selector: balanceOf(address) -> 0x70a08231
pub async fn get_erc20_balance(
    token_addr: &str,
    account: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data_hex = encode_balance_of_call(account)?;
    let hex_result = eth_call(rpc_url, token_addr, &data_hex).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 {
        anyhow::bail!("balanceOf: short response");
    }
    decode_u128_at(raw, 0)
}

/// Check ERC-20 allowance: token.allowance(owner, spender)
/// Function selector: allowance(address,address) -> 0xdd62ed3e
pub async fn get_allowance(
    token_addr: &str,
    owner_addr: &str,
    spender_addr: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data_hex = encode_allowance_call(owner_addr, spender_addr)?;
    let hex_result = eth_call(rpc_url, token_addr, &data_hex).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 {
        anyhow::bail!("allowance: short response");
    }
    decode_u128_at(raw, 0)
}

/// Get ERC-20 token symbol: token.symbol()
/// Function selector: symbol() -> 0x95d89b41
pub async fn get_erc20_symbol(token_addr: &str, rpc_url: &str) -> anyhow::Result<String> {
    let hex_result = eth_call(rpc_url, token_addr, "0x95d89b41").await?;
    let raw = strip_0x(&hex_result);
    // ABI-encoded string: offset(32) + length(32) + data(padded)
    if raw.len() < 128 {
        return Ok(String::new());
    }
    let len = usize::from_str_radix(&raw[64..128], 16).unwrap_or(0);
    if len == 0 || raw.len() < 128 + len * 2 {
        return Ok(String::new());
    }
    let bytes = hex::decode(&raw[128..128 + len * 2]).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Get native ETH balance via eth_getBalance.
pub async fn get_eth_balance(account: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let client = reqwest::Client::new();
    let req = json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [account, "latest"],
        "id": 1
    });
    let resp: RpcResponse = client
        .post(rpc_url)
        .json(&req)
        .send()
        .await
        .context("eth_getBalance HTTP request failed")?
        .json()
        .await
        .context("eth_getBalance response parse failed")?;
    if let Some(err) = resp.error {
        anyhow::bail!("eth_getBalance RPC error: {}", err);
    }
    let hex_str = resp.result.ok_or_else(|| anyhow::anyhow!("eth_getBalance returned null"))?;
    let raw = strip_0x(&hex_str);
    u128::from_str_radix(raw, 16).context("eth_getBalance: hex parse error")
}

/// Get the aToken address for an asset via IPoolDataProvider.getReserveTokensAddresses(asset).
/// Selector 0xd2493b6c — verified against Aave V3 PoolDataProvider.
/// Returns the aTokenAddress (first of the three returned addresses).
pub async fn get_atoken_address(
    data_provider: &str,
    asset: &str,
    rpc_url: &str,
) -> anyhow::Result<String> {
    let asset_bytes = parse_address(asset)?;
    let mut data = hex::decode("d2493b6c")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&asset_bytes);
    let data_hex = format!("0x{}", hex::encode(&data));
    let hex_result = eth_call(rpc_url, data_provider, &data_hex).await?;
    let raw = strip_0x(&hex_result);
    // Returns 3 x address (each 32 bytes = 64 hex chars), total 192 hex chars minimum
    if raw.len() < 192 {
        anyhow::bail!("getReserveTokensAddresses: short response ({} hex chars)", raw.len());
    }
    // First slot (bytes 0..64) = aTokenAddress
    decode_address_result(&format!("0x{}", &raw[0..64]))
}

// ── calldata encoders (shared by one-shot reads and the polling waits) ────

/// balanceOf(address) -> 0x70a08231
fn encode_balance_of_call(account: &str) -> anyhow::Result<String> {
    let owner = parse_address(account)?;
    let mut data = hex::decode("70a08231")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&owner);
    Ok(format!("0x{}", hex::encode(&data)))
}

/// allowance(address,address) -> 0xdd62ed3e
fn encode_allowance_call(owner_addr: &str, spender_addr: &str) -> anyhow::Result<String> {
    let owner = parse_address(owner_addr)?;
    let spender = parse_address(spender_addr)?;
    let mut data = hex::decode("dd62ed3e")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&owner);
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&spender);
    Ok(format!("0x{}", hex::encode(&data)))
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn strip_0x(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

fn decode_address_result(hex_result: &str) -> anyhow::Result<String> {
    let raw = strip_0x(hex_result);
    if raw.len() < 64 {
        anyhow::bail!("decode_address_result: short result '{}'", hex_result);
    }
    // Last 40 hex chars = 20 byte address
    let addr_hex = &raw[raw.len() - 40..];
    Ok(format!("0x{}", addr_hex))
}

fn parse_address(addr: &str) -> anyhow::Result<[u8; 20]> {
    let clean = strip_0x(addr);
    if clean.len() != 40 {
        anyhow::bail!("Invalid address (must be 20 bytes / 40 hex chars): {}", addr);
    }
    let bytes = hex::decode(clean).context("Invalid hex address")?;
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn decode_u128_at(raw: &str, slot: usize) -> anyhow::Result<u128> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        anyhow::bail!("decode_u128_at: slot {} out of range (raw len {})", slot, raw.len());
    }
    let slot_hex = &raw[start..end];
    // u256 may exceed u128 — take lower 32 hex chars (16 bytes)
    let low32 = &slot_hex[32..64];
    let val = u128::from_str_radix(low32, 16)
        .with_context(|| format!("decode_u128_at: invalid hex '{}'", low32))?;
    Ok(val)
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Instant;

    /// Canned-response HTTP server for RPC polling tests.
    /// Serves `responses` in order; the last entry repeats for further requests.
    fn mock_rpc(responses: Vec<(u16, &'static str)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let mut i = 0usize;
            for stream in listener.incoming() {
                let mut s = match stream {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let (code, body) = responses[i.min(responses.len() - 1)];
                i += 1;
                let resp = format!(
                    "HTTP/1.1 {} X\r\ncontent-type: application/json\r\n\
                     content-length: {}\r\nconnection: close\r\n\r\n{}",
                    code,
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes());
                let _ = s.flush();
            }
        });
        format!("http://{}", addr)
    }

    const TX: &str = "0x7e9a1342000000000000000000000000000000000000000000000000000000ab";

    /// The reported bug: publicnode answers eth_getTransactionReceipt on Base and
    /// Arbitrum with `-32602 Archive requests require a personal token`. The wait
    /// must surface that reason immediately, not poll for the full deadline and
    /// then blame a timeout.
    #[tokio::test]
    async fn wait_for_tx_surfaces_gated_receipt_method() {
        let url = mock_rpc(vec![(
            200,
            r#"{"jsonrpc":"2.0","error":{"code":-32602,"message":"Archive requests require a personal token. Get one at: https://www.allnodes.com/publicnode"},"id":1}"#,
        )]);
        let started = Instant::now();
        let err = wait_for_tx(&url, TX).await.unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("-32602"), "must name the RPC error code: {}", msg);
        assert!(msg.contains("personal token"), "must carry the RPC message: {}", msg);
        assert!(
            started.elapsed().as_secs() < 10,
            "must fail fast on a refused method, took {:?}",
            started.elapsed()
        );
    }

    /// Same endpoint hard-403s the method once it has been probed a few times.
    /// reqwest treats 403 as a successful send, so the old code dropped it on the
    /// floor and reported a timeout instead.
    #[tokio::test]
    async fn wait_for_tx_surfaces_http_403() {
        let url = mock_rpc(vec![(403, "Forbidden")]);
        let started = Instant::now();
        let err = wait_for_tx(&url, TX).await.unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("403"), "must name the HTTP status: {}", msg);
        assert!(
            started.elapsed().as_secs() < 10,
            "must fail fast on a refused method, took {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn wait_for_tx_reports_success_receipt() {
        let url = mock_rpc(vec![(
            200,
            r#"{"jsonrpc":"2.0","id":1,"result":{"status":"0x1","blockNumber":"0x1"}}"#,
        )]);
        assert!(wait_for_tx(&url, TX).await.unwrap());
    }

    #[tokio::test]
    async fn wait_for_tx_reports_reverted_receipt() {
        let url = mock_rpc(vec![(
            200,
            r#"{"jsonrpc":"2.0","id":1,"result":{"status":"0x0","blockNumber":"0x1"}}"#,
        )]);
        assert!(!wait_for_tx(&url, TX).await.unwrap());
    }

    // ── the new condition-based waits ───────────────────────────────────

    const WORD_ZERO: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";
    const WORD_MAX: &str =
        "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    fn eth_call_ok(word: &str) -> String {
        format!(r#"{{"jsonrpc":"2.0","id":1,"result":"{}"}}"#, word)
    }

    /// A transient failure must not be reported as a bare timeout: the reason has
    /// to survive into the final error, which is what the old code lost.
    #[tokio::test]
    async fn wait_for_tx_timeout_names_last_transient_reason() {
        let url = mock_rpc(vec![(429, r#"{"error":"rate limited"}"#)]);
        let err = wait_for_tx_inner(
            &url,
            TX,
            Duration::from_millis(900),
            Duration::from_millis(200),
        )
        .await
        .unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("Timed out"), "{}", msg);
        assert!(msg.contains("429"), "timeout must name the last failure: {}", msg);
    }

    #[tokio::test]
    async fn wait_for_allowance_returns_once_visible() {
        // Not visible on the first poll, visible on the second.
        let zero = Box::leak(eth_call_ok(WORD_ZERO).into_boxed_str());
        let max = Box::leak(eth_call_ok(WORD_MAX).into_boxed_str());
        let url = mock_rpc(vec![(200, zero), (200, max)]);
        let seen = wait_for_uint_at_least(
            "allowance",
            &url,
            "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
            "0xdd62ed3e",
            u128::MAX,
            Duration::from_secs(5),
            Duration::from_millis(200),
        )
        .await
        .unwrap();
        assert_eq!(seen, u128::MAX);
    }

    /// If eth_call itself gets refused, say so — do not sit through the deadline.
    #[tokio::test]
    async fn wait_for_allowance_surfaces_gated_eth_call() {
        let url = mock_rpc(vec![(
            200,
            r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"the method eth_call does not exist"},"id":1}"#,
        )]);
        let started = Instant::now();
        let err = wait_for_allowance(
            "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
            "0xf3e0091706e3cfbc39a00d8f172de826ed6fdcde",
            "0xa238dd80c259a72e81d7e4664a9801593f98d1c5",
            1,
            &url,
        )
        .await
        .unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("-32601"), "{}", msg);
        assert!(msg.contains("allowance"), "{}", msg);
        assert!(started.elapsed().as_secs() < 10, "took {:?}", started.elapsed());
    }

    #[tokio::test]
    async fn wait_for_allowance_timeout_reports_last_observed_value() {
        let zero = Box::leak(eth_call_ok(WORD_ZERO).into_boxed_str());
        let url = mock_rpc(vec![(200, zero)]);
        let err = wait_for_uint_at_least(
            "allowance",
            &url,
            "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
            "0xdd62ed3e",
            2_000_000,
            Duration::from_millis(900),
            Duration::from_millis(200),
        )
        .await
        .unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("last observed: 0"), "{}", msg);
        assert!(msg.contains("2000000"), "{}", msg);
    }

    // ── calldata encoders ───────────────────────────────────────────────

    #[test]
    fn encoders_produce_selector_plus_padded_args() {
        let owner = "0xf3e0091706e3cfbc39a00d8f172de826ed6fdcde";
        let spender = "0xa238dd80c259a72e81d7e4664a9801593f98d1c5";

        let bal = encode_balance_of_call(owner).unwrap();
        assert_eq!(
            bal,
            "0x70a08231000000000000000000000000f3e0091706e3cfbc39a00d8f172de826ed6fdcde"
        );

        let allow = encode_allowance_call(owner, spender).unwrap();
        let expected_allow = concat!(
            "0xdd62ed3e",
            "000000000000000000000000f3e0091706e3cfbc39a00d8f172de826ed6fdcde",
            "000000000000000000000000a238dd80c259a72e81d7e4664a9801593f98d1c5"
        );
        assert_eq!(allow, expected_allow);
    }

    // ── live checks against the real Base endpoint ──────────────────────
    //
    // Network-dependent, so #[ignore] by default. Run explicitly with:
    //   cargo test -- --ignored --nocapture
    // These reproduce the exact condition behind the "Approve tx did not confirm
    // in time" reports: publicnode serves eth_call on Base but refuses
    // eth_getTransactionReceipt.

    const BASE_RPC: &str = "https://base-rpc.publicnode.com";
    const BASE_USDC: &str = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913";
    const BASE_PROVIDER: &str = "0xe20fCBdBfFC4Dd138cE8b2E6FBb6CB49777ad64D";
    const SOME_OWNER: &str = "0xf3e0091706e3cfbc39a00d8f172de826ed6fdcde";

    #[tokio::test]
    #[ignore]
    async fn live_base_receipt_lookup_is_refused_and_reported() {
        let started = Instant::now();
        let err = wait_for_tx(BASE_RPC, TX).await.unwrap_err();
        let msg = format!("{:#}", err);
        println!("live wait_for_tx -> {}", msg);
        assert!(
            msg.contains("-32602") || msg.contains("403") || msg.contains("-32601"),
            "must carry the endpoint's own refusal: {}",
            msg
        );
        assert!(
            started.elapsed().as_secs() < 15,
            "must fail fast, took {:?}",
            started.elapsed()
        );
    }

    /// The same endpoint serves eth_call, so the allowance wait works where the
    /// receipt wait cannot. Positive case: an already-satisfied threshold returns
    /// immediately.
    #[tokio::test]
    #[ignore]
    async fn live_base_allowance_wait_reads_chain() {
        let pool = get_pool(BASE_PROVIDER, BASE_RPC).await.unwrap();
        println!("live resolved Base Pool -> {}", pool);
        let started = Instant::now();
        let seen = wait_for_allowance(BASE_USDC, SOME_OWNER, &pool, 0, BASE_RPC)
            .await
            .unwrap();
        println!("live allowance read -> {} in {:?}", seen, started.elapsed());
        assert!(started.elapsed().as_secs() < 15);
    }

    /// Negative case on the same live endpoint: an unreachable threshold times out
    /// reporting the value it actually read, instead of a bare timeout.
    #[tokio::test]
    #[ignore]
    async fn live_base_allowance_wait_reports_observed_value() {
        let pool = get_pool(BASE_PROVIDER, BASE_RPC).await.unwrap();
        let data = encode_allowance_call(SOME_OWNER, &pool).unwrap();
        let err = wait_for_uint_at_least(
            "allowance",
            BASE_RPC,
            BASE_USDC,
            &data,
            u128::MAX,
            Duration::from_secs(4),
            Duration::from_secs(1),
        )
        .await
        .unwrap_err();
        let msg = format!("{:#}", err);
        println!("live allowance timeout -> {}", msg);
        assert!(msg.contains("last observed:"), "{}", msg);
    }
}
