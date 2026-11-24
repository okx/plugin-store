/// Minimal JSON-RPC eth_call helpers for read-only EVM queries.
/// Each chain's RPC URL is taken from `config::ChainInfo::rpc`.

/// ABI-pad a 20-byte address to 32 bytes (left-padded zeros).
pub fn pad_address(addr: &str) -> String {
    let a = addr.trim_start_matches("0x");
    format!("{:0>64}", a)
}

/// eth_call helper: sends a JSON-RPC eth_call to the given RPC URL.
/// Treats response.result == null/missing as an error rather than silently returning "".
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
        .await
        .map_err(|e| anyhow::anyhow!("eth_call HTTP failed: {}", e))?
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("eth_call JSON parse failed: {}", e))?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call rpc error: {}", err);
    }
    let result = resp["result"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_call missing result field"))?
        .to_string();
    Ok(result)
}

/// Decode a hex word as a u128 from the LAST 32 bytes (uint256 max value will be truncated, OK
/// for practical token amounts which fit in 128 bits).
fn parse_u128_word(hex: &str) -> u128 {
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.is_empty() {
        return 0;
    }
    let take = trimmed.len().saturating_sub(32);
    u128::from_str_radix(&trimmed[take..], 16).unwrap_or(0)
}

/// Query ERC-20 balanceOf(address) → u128 (token atomic units).
/// Returns Err on RPC failure (per knowledge base EVM-012: do NOT silently return 0).
pub async fn erc20_balance(token: &str, owner: &str, rpc: &str) -> anyhow::Result<u128> {
    // balanceOf(address) selector: 0x70a08231
    let data = format!("0x70a08231{}", pad_address(owner));
    let hex = eth_call(rpc, token, &data)
        .await
        .map_err(|e| anyhow::anyhow!("erc20 balanceOf({}) on RPC {} failed: {}", token, rpc, e))?;
    Ok(parse_u128_word(&hex))
}

/// Query ERC-20 allowance(owner, spender) → u128.
pub async fn erc20_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc: &str,
) -> anyhow::Result<u128> {
    // allowance(address,address) selector: 0xdd62ed3e
    let data = format!(
        "0xdd62ed3e{}{}",
        pad_address(owner),
        pad_address(spender)
    );
    let hex = eth_call(rpc, token, &data)
        .await
        .map_err(|e| anyhow::anyhow!("erc20 allowance({}) on RPC {} failed: {}", token, rpc, e))?;
    Ok(parse_u128_word(&hex))
}

/// Query the native token balance of an address via `eth_getBalance`.
pub async fn native_balance(addr: &str, rpc: &str) -> anyhow::Result<u128> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_getBalance",
        "params": [addr, "latest"]
    });
    let resp: serde_json::Value = client
        .post(rpc)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("eth_getBalance HTTP failed: {}", e))?
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("eth_getBalance JSON parse failed: {}", e))?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_getBalance rpc error: {}", err);
    }
    let hex = resp["result"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_getBalance missing result"))?;
    Ok(parse_u128_word(hex))
}

/// Poll `eth_getTransactionReceipt` until the tx is mined (or deadline hits).
/// Returns Ok(()) on status=0x1, Err on status=0x0 (reverted) or timeout.
/// Knowledge base EVM-006: never use blind sleep — must poll on-chain confirmation.
pub async fn wait_for_tx(tx_hash: &str, rpc: &str, timeout_secs: u64) -> anyhow::Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    loop {
        if std::time::Instant::now() > deadline {
            anyhow::bail!("Timeout ({}s) waiting for tx {} to confirm", timeout_secs, tx_hash);
        }
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash]
        });
        let resp = client.post(rpc).json(&body).send().await;
        if let Ok(r) = resp {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                if v["result"].is_object() {
                    let status = v["result"]["status"].as_str().unwrap_or("");
                    match status {
                        "0x1" => return Ok(()),
                        "0x0" => anyhow::bail!(
                            "tx {} mined but reverted (status 0x0). Inspect on the explorer for revert reason.",
                            tx_hash
                        ),
                        _ => {}
                    }
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

/// Format a u128 atomic amount with the token's decimals. Trims trailing zeros after the dot.
pub fn fmt_token_amount(raw: u128, decimals: u32) -> String {
    if decimals == 0 {
        return raw.to_string();
    }
    let factor = 10u128.pow(decimals);
    let whole = raw / factor;
    let frac = raw % factor;
    if frac == 0 {
        return whole.to_string();
    }
    let frac_str = format!("{:0width$}", frac, width = decimals as usize);
    let trimmed = frac_str.trim_end_matches('0');
    if trimmed.is_empty() {
        whole.to_string()
    } else {
        format!("{}.{}", whole, trimmed)
    }
}
