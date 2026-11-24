use anyhow::Context;
use std::time::{Duration, Instant};

const WAIT_TIMEOUT: Duration = Duration::from_secs(60);
const WAIT_INTERVAL: Duration = Duration::from_secs(2);

/// Make a raw eth_call via JSON-RPC.
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("Failed to build RPC client")?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": data },
            "latest"
        ],
        "id": 1
    });
    let response = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("RPC request failed")?;
    let status = response.status();
    let raw_body = response
        .text()
        .await
        .context("RPC response body read failed")?;
    if !status.is_success() {
        anyhow::bail!("eth_call HTTP {}: {}", status, output_snippet(&raw_body));
    }
    let resp: serde_json::Value = serde_json::from_str(&raw_body)
        .with_context(|| format!("RPC response parse failed: {}", output_snippet(&raw_body)))?;

    if let Some(err) = resp.get("error").filter(|error| !error.is_null()) {
        anyhow::bail!("eth_call error: {}", err);
    }
    let result = resp["result"]
        .as_str()
        .context("Missing result field in RPC response")?
        .to_string();
    Ok(result)
}

fn output_snippet(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.chars().count() <= 300 {
        trimmed.to_string()
    } else {
        format!("{}…", trimmed.chars().take(300).collect::<String>())
    }
}

/// Read ERC-20 balance of `owner` at `token`.
/// Returns raw u128 balance.
pub async fn erc20_balance_of(token: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    // balanceOf(address) selector = 0x70a08231
    let owner_clean = owner.trim_start_matches("0x");
    let data = format!("0x70a08231{:0>64}", owner_clean);
    let hex = eth_call(token, &data, rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() || hex_clean == "0" {
        return Ok(0);
    }
    let padded = format!("{:0>64}", hex_clean);
    let val = u128::from_str_radix(&padded[padded.len() - 32..], 16)?;
    Ok(val)
}

/// Read ERC-20 allowance for `spender` from `owner`.
pub async fn erc20_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data = encode_allowance_call(owner, spender)?;
    read_uint_call(token, &data, rpc_url).await
}

pub fn allowance_needs_approval(current: u128, required: u128) -> bool {
    current < required
}

/// Wait until an approve is visible through the state the next transaction
/// actually depends on. This intentionally avoids eth_getTransactionReceipt,
/// which is restricted by some public RPC endpoints.
pub async fn wait_for_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    min_allowance: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data = encode_allowance_call(owner, spender)?;
    wait_for_uint_at_least(
        "allowance",
        token,
        &data,
        min_allowance,
        rpc_url,
        WAIT_TIMEOUT,
        WAIT_INTERVAL,
    )
    .await
}

/// Wait until an ETH->WETH wrap is visible as an ERC-20 balance.
pub async fn wait_for_erc20_balance(
    token: &str,
    owner: &str,
    min_balance: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data = encode_balance_of_call(owner)?;
    wait_for_uint_at_least(
        "token balance",
        token,
        &data,
        min_balance,
        rpc_url,
        WAIT_TIMEOUT,
        WAIT_INTERVAL,
    )
    .await
}

async fn wait_for_uint_at_least(
    label: &str,
    token: &str,
    data: &str,
    min_value: u128,
    rpc_url: &str,
    timeout: Duration,
    interval: Duration,
) -> anyhow::Result<u128> {
    let deadline = Instant::now() + timeout;
    let mut last_seen: Option<u128> = None;
    let mut last_error: Option<String> = None;

    loop {
        match read_uint_call(token, data, rpc_url).await {
            Ok(value) => {
                last_seen = Some(value);
                if value >= min_value {
                    return Ok(value);
                }
            }
            Err(error) => last_error = Some(format!("{:#}", error)),
        }

        if Instant::now() + interval > deadline {
            anyhow::bail!(
                "Timed out after {}s waiting for {} to reach {} (last observed: {}){}",
                timeout.as_secs_f64(),
                label,
                min_value,
                last_seen.map_or_else(|| "never read".to_string(), |value| value.to_string()),
                last_error.map_or_else(String::new, |error| format!(
                    " -- last RPC error: {}",
                    error
                )),
            );
        }
        tokio::time::sleep(interval).await;
    }
}

async fn read_uint_call(token: &str, data: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex = eth_call(token, data, rpc_url).await?;
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 64 {
        anyhow::bail!("eth_call returned a short uint256 response: '{}'", hex);
    }
    let low_128 = &raw[raw.len() - 32..];
    u128::from_str_radix(low_128, 16)
        .with_context(|| format!("Invalid uint256 response from eth_call: '{}'", hex))
}

fn encode_balance_of_call(owner: &str) -> anyhow::Result<String> {
    let owner = encode_address_word(owner)?;
    Ok(format!("0x70a08231{}", owner))
}

fn encode_allowance_call(owner: &str, spender: &str) -> anyhow::Result<String> {
    let owner = encode_address_word(owner)?;
    let spender = encode_address_word(spender)?;
    Ok(format!("0xdd62ed3e{}{}", owner, spender))
}

fn encode_address_word(address: &str) -> anyhow::Result<String> {
    let raw = address.trim_start_matches("0x");
    if raw.len() != 40 || !raw.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!("Invalid EVM address '{}'", address);
    }
    Ok(format!("{:0>64}", raw.to_ascii_lowercase()))
}

/// Read ERC-20 decimals.
pub async fn erc20_decimals(token: &str, rpc_url: &str) -> anyhow::Result<u8> {
    // decimals() selector = 0x313ce567
    let hex = eth_call(token, "0x313ce567", rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() {
        return Ok(18);
    }
    let padded = format!("{:0>64}", hex_clean);
    let val = u8::from_str_radix(&padded[padded.len() - 2..], 16).unwrap_or(18);
    Ok(val)
}

/// Read ERC-20 symbol.
/// Handles both dynamic string ABI encoding (ERC-20 standard) and bytes32 encoding
/// used by older tokens (USDC, USDT deployed pre-ERC-20 string standard).
pub async fn erc20_symbol(token: &str, rpc_url: &str) -> anyhow::Result<String> {
    // symbol() selector = 0x95d89b41
    let hex = eth_call(token, "0x95d89b41", rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");

    // bytes32 encoding: exactly 64 hex chars (32 bytes), null-padded ASCII
    // Used by USDC, USDT, and other tokens deployed before the string standard
    if hex_clean.len() == 64 {
        let bytes = hex::decode(hex_clean).unwrap_or_default();
        let trimmed: Vec<u8> = bytes.into_iter().take_while(|&b| b != 0).collect();
        if !trimmed.is_empty() {
            return Ok(String::from_utf8_lossy(&trimmed).to_string());
        }
        return Ok("UNKNOWN".to_string());
    }

    // Dynamic string ABI encoding: offset(32) + length(32) + data
    // Each ABI slot is 32 bytes = 64 hex chars
    // [0..64]   = offset pointer (always 0x20)
    // [64..128] = string byte length (32-byte slot)
    // [128..]   = string data (padded to 32-byte boundary)
    if hex_clean.len() < 128 {
        return Ok("UNKNOWN".to_string());
    }
    let len_hex = &hex_clean[64..128];
    let len = usize::from_str_radix(len_hex, 16).unwrap_or(0);
    if len == 0 || hex_clean.len() < 128 + len * 2 {
        return Ok("UNKNOWN".to_string());
    }
    let data_hex = &hex_clean[128..128 + len * 2];
    let bytes = hex::decode(data_hex).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Read native ETH balance of `owner`.
pub async fn eth_balance(owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [owner, "latest"],
        "id": 1
    });
    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("RPC request failed")?
        .json()
        .await
        .context("RPC response parse failed")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_getBalance error: {}", err);
    }
    let hex = resp["result"]
        .as_str()
        .context("Missing result field in eth_getBalance response")?;
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() {
        return Ok(0);
    }
    let padded = format!("{:0>32}", hex_clean);
    let val = u128::from_str_radix(&padded[padded.len().saturating_sub(32)..], 16).unwrap_or(0);
    Ok(val)
}

/// Read vault share balance (ERC-20 balanceOf, same encoding).
pub async fn vault_share_balance(vault: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    erc20_balance_of(vault, owner, rpc_url).await
}

/// convertToAssets(shares) on ERC-4626 vault.
pub async fn vault_convert_to_assets(
    vault: &str,
    shares: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    // convertToAssets(uint256) selector = 0x07a2d13a
    let shares_hex = format!("{:064x}", shares);
    let data = format!("0x07a2d13a{}", shares_hex);
    let hex = eth_call(vault, &data, rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() {
        return Ok(0);
    }
    let padded = format!("{:0>64}", hex_clean);
    let val = u128::from_str_radix(&padded[padded.len() - 32..], 16)?;
    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn mock_rpc(responses: Vec<&'static str>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let mut index = 0usize;
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut request = [0u8; 4096];
                let _ = stream.read(&mut request);
                let body = responses[index.min(responses.len() - 1)];
                index += 1;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        format!("http://{}", address)
    }

    const TOKEN: &str = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913";
    const OWNER: &str = "0xf3e0091706e3cfbc39a00d8f172de826ed6fdcde";
    const SPENDER: &str = "0xa238dd80c259a72e81d7e4664a9801593f98d1c5";
    const ZERO: &str = r#"{"jsonrpc":"2.0","id":1,"result":"0x0000000000000000000000000000000000000000000000000000000000000000"}"#;
    const TEN: &str = r#"{"jsonrpc":"2.0","id":1,"result":"0x000000000000000000000000000000000000000000000000000000000000000a"}"#;

    #[test]
    fn encodes_allowance_and_balance_calls() {
        assert_eq!(
            encode_balance_of_call(OWNER).unwrap(),
            "0x70a08231000000000000000000000000f3e0091706e3cfbc39a00d8f172de826ed6fdcde"
        );
        assert_eq!(
            encode_allowance_call(OWNER, SPENDER).unwrap(),
            concat!(
                "0xdd62ed3e",
                "000000000000000000000000f3e0091706e3cfbc39a00d8f172de826ed6fdcde",
                "000000000000000000000000a238dd80c259a72e81d7e4664a9801593f98d1c5"
            )
        );
    }

    #[test]
    fn sufficient_allowance_skips_approval() {
        assert!(!allowance_needs_approval(10, 10));
        assert!(!allowance_needs_approval(11, 10));
        assert!(allowance_needs_approval(9, 10));
    }

    #[tokio::test]
    async fn allowance_polling_continues_when_state_becomes_visible() {
        let url = mock_rpc(vec![ZERO, TEN]);
        let seen = wait_for_uint_at_least(
            "allowance",
            TOKEN,
            &encode_allowance_call(OWNER, SPENDER).unwrap(),
            10,
            &url,
            Duration::from_secs(2),
            Duration::from_millis(25),
        )
        .await
        .unwrap();
        assert_eq!(seen, 10);
    }

    #[tokio::test]
    async fn wrapped_balance_polling_uses_balance_of_state() {
        let url = mock_rpc(vec![ZERO, TEN]);
        let seen = wait_for_uint_at_least(
            "token balance",
            TOKEN,
            &encode_balance_of_call(OWNER).unwrap(),
            10,
            &url,
            Duration::from_secs(2),
            Duration::from_millis(25),
        )
        .await
        .unwrap();
        assert_eq!(seen, 10);
    }

    #[tokio::test]
    async fn allowance_timeout_reports_last_value_and_rpc_error() {
        let url = mock_rpc(vec![
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"lagging"}}"#,
            ZERO,
        ]);
        let err = wait_for_uint_at_least(
            "allowance",
            TOKEN,
            &encode_allowance_call(OWNER, SPENDER).unwrap(),
            10,
            &url,
            Duration::from_millis(120),
            Duration::from_millis(25),
        )
        .await
        .unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("last observed: 0"), "got: {message}");
        assert!(message.contains("lagging"), "got: {message}");
    }
}
