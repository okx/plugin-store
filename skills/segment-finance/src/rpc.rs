// Segment Finance — Direct eth_call RPC layer

use anyhow::Result;
use serde_json::{json, Value};

pub fn build_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .or_else(|_| std::env::var("http_proxy"))
    {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_default()
}

/// Raw eth_call — returns hex result string
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> Result<String> {
    let client = build_client();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });
    let resp: Value = client.post(rpc_url).json(&body).send().await?.json().await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Decode a hex string as a single uint256 (first 32 bytes)
pub fn decode_uint256(hex: &str) -> u128 {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 64 {
        return 0;
    }
    // Use last 32 hex chars to avoid u128 overflow on large uint256
    let tail = if clean.len() > 32 {
        &clean[clean.len() - 32..]
    } else {
        clean
    };
    u128::from_str_radix(tail, 16).unwrap_or(0)
}

/// Decode a hex string as an address (last 20 bytes of first word)
pub fn decode_address(hex: &str) -> String {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 64 {
        return "0x0000000000000000000000000000000000000000".to_string();
    }
    format!("0x{}", &clean[24..64])
}

/// Decode a dynamic bytes/string return from eth_call
pub fn decode_string(hex: &str) -> String {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 128 {
        // Try fixed bytes32 fallback
        if clean.len() >= 64 {
            let bytes = (0..64)
                .step_by(2)
                .filter_map(|i| u8::from_str_radix(&clean[i..i + 2], 16).ok())
                .take_while(|&b| b != 0)
                .collect::<Vec<u8>>();
            return String::from_utf8_lossy(&bytes).into_owned();
        }
        return String::new();
    }
    // offset at [0..64], length at [64..128], data starts at [128..]
    let len = usize::from_str_radix(&clean[64..128], 16).unwrap_or(0);
    if len == 0 || 128 + len * 2 > clean.len() {
        return String::new();
    }
    let data_hex = &clean[128..128 + len * 2];
    let bytes: Vec<u8> = (0..data_hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&data_hex[i..i + 2], 16).ok())
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Pad an address to 32 bytes (for calldata)
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a uint256 to 32 bytes
pub fn pad_uint256(val: u128) -> String {
    format!("{:064x}", val)
}

/// Get ERC-20 symbol
pub async fn erc20_symbol(rpc_url: &str, token: &str) -> String {
    // symbol() selector: 0x95d89b41
    if let Ok(hex) = eth_call(rpc_url, token, "0x95d89b41").await {
        let s = decode_string(&hex);
        if !s.is_empty() {
            return s;
        }
    }
    "UNKNOWN".to_string()
}

/// Get ERC-20 decimals
pub async fn erc20_decimals(rpc_url: &str, token: &str) -> u32 {
    // decimals() selector: 0x313ce567
    if let Ok(hex) = eth_call(rpc_url, token, "0x313ce567").await {
        let clean = hex.trim_start_matches("0x");
        if clean.len() >= 2 {
            if let Ok(v) = u32::from_str_radix(&clean[clean.len().saturating_sub(2)..], 16) {
                return v;
            }
        }
    }
    18
}

/// Get ERC-20 balance of address
pub async fn erc20_balance(rpc_url: &str, token: &str, holder: &str) -> u128 {
    // balanceOf(address) selector: 0x70a08231
    let data = format!("0x70a08231{}", pad_address(holder));
    if let Ok(hex) = eth_call(rpc_url, token, &data).await {
        return decode_uint256(&hex);
    }
    0
}

/// Comptroller.getAllMarkets() -> address[]
/// Note: Segment Finance uses Diamond proxy — getAllMarkets selector 0xb0772d0b works.
pub async fn get_all_markets(rpc_url: &str, comptroller: &str) -> Result<Vec<String>> {
    // getAllMarkets() selector: 0xb0772d0b
    let hex = eth_call(rpc_url, comptroller, "0xb0772d0b").await?;
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 128 {
        return Ok(vec![]);
    }
    let count = usize::from_str_radix(&clean[64..128], 16).unwrap_or(0);
    // Sanity check: the Diamond proxy packs selectors not addresses in some positions,
    // which can produce garbage. Use only known market addresses.
    if count > 20 || count == 0 {
        // Fall back to hardcoded known markets
        return Ok(get_known_markets());
    }
    let mut markets = Vec::with_capacity(count);
    for i in 0..count {
        let start = 128 + i * 64;
        if start + 64 > clean.len() {
            break;
        }
        let addr = format!("0x{}", &clean[start + 24..start + 64]);
        // Basic sanity check: valid EVM address
        if addr.len() == 42 && addr != "0x0000000000000000000000000000000000000000" {
            markets.push(addr);
        }
    }
    if markets.is_empty() {
        return Ok(get_known_markets());
    }
    Ok(markets)
}

/// Known Segment Finance markets (BSC mainnet) — fallback list
pub fn get_known_markets() -> Vec<String> {
    vec![
        crate::config::SEBNB.to_string(),
        crate::config::SEUSDT.to_string(),
        crate::config::SEUSDC.to_string(),
        crate::config::SEBTC.to_string(),
        crate::config::SEETH.to_string(),
    ]
}

/// seToken.getAccountSnapshot(address) -> (error, seTokenBalance, borrowBalance, exchangeRate)
pub async fn get_account_snapshot(
    rpc_url: &str,
    setoken: &str,
    wallet: &str,
) -> Result<(u128, u128, u128, u128)> {
    // getAccountSnapshot(address) selector: 0xc37f68e2
    let data = format!("0xc37f68e2{}", pad_address(wallet));
    let hex = eth_call(rpc_url, setoken, &data).await?;
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 256 {
        return Ok((0, 0, 0, 0));
    }
    let err_code = decode_uint256(&format!("0x{}", &clean[0..64]));
    let setoken_bal = decode_uint256(&format!("0x{}", &clean[64..128]));
    let borrow_bal = decode_uint256(&format!("0x{}", &clean[128..192]));
    let exchange_rate = decode_uint256(&format!("0x{}", &clean[192..256]));
    Ok((err_code, setoken_bal, borrow_bal, exchange_rate))
}

/// Comptroller.getAccountLiquidity(address) -> (error, liquidity, shortfall)
pub async fn get_account_liquidity(
    rpc_url: &str,
    comptroller: &str,
    wallet: &str,
) -> Result<(u128, u128, u128)> {
    // getAccountLiquidity(address) selector: 0x5ec88c79
    let data = format!("0x5ec88c79{}", pad_address(wallet));
    let hex = eth_call(rpc_url, comptroller, &data).await?;
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 192 {
        return Ok((0, 0, 0));
    }
    let err = decode_uint256(&format!("0x{}", &clean[0..64]));
    let liquidity = decode_uint256(&format!("0x{}", &clean[64..128]));
    let shortfall = decode_uint256(&format!("0x{}", &clean[128..192]));
    Ok((err, liquidity, shortfall))
}

/// seToken supply rate per block
pub async fn get_supply_rate_per_block(rpc_url: &str, setoken: &str) -> u128 {
    // supplyRatePerBlock() selector: 0xae9d70b0
    if let Ok(hex) = eth_call(rpc_url, setoken, "0xae9d70b0").await {
        return decode_uint256(&hex);
    }
    0
}

/// seToken borrow rate per block
pub async fn get_borrow_rate_per_block(rpc_url: &str, setoken: &str) -> u128 {
    // borrowRatePerBlock() selector: 0xf8f9da28
    if let Ok(hex) = eth_call(rpc_url, setoken, "0xf8f9da28").await {
        return decode_uint256(&hex);
    }
    0
}

/// seToken total borrows
pub async fn get_total_borrows(rpc_url: &str, setoken: &str) -> u128 {
    // totalBorrows() selector: 0x47bd3718
    if let Ok(hex) = eth_call(rpc_url, setoken, "0x47bd3718").await {
        return decode_uint256(&hex);
    }
    0
}

/// seToken available cash
pub async fn get_cash(rpc_url: &str, setoken: &str) -> u128 {
    // getCash() selector: 0x3b1d21a2
    if let Ok(hex) = eth_call(rpc_url, setoken, "0x3b1d21a2").await {
        return decode_uint256(&hex);
    }
    0
}

/// seToken exchange rate stored
pub async fn get_exchange_rate_stored(rpc_url: &str, setoken: &str) -> u128 {
    // exchangeRateStored() selector: 0x182df0f5
    if let Ok(hex) = eth_call(rpc_url, setoken, "0x182df0f5").await {
        return decode_uint256(&hex);
    }
    0
}

/// seToken underlying address
pub async fn get_underlying(rpc_url: &str, setoken: &str) -> String {
    // underlying() selector: 0x6f307dc3
    if let Ok(hex) = eth_call(rpc_url, setoken, "0x6f307dc3").await {
        return decode_address(&hex);
    }
    "0x0000000000000000000000000000000000000000".to_string()
}

/// Oracle.getUnderlyingPrice(address seToken) -> uint256
pub async fn get_underlying_price(rpc_url: &str, oracle: &str, setoken: &str) -> u128 {
    // getUnderlyingPrice(address) selector: 0xfc57d4df
    let data = format!("0xfc57d4df{}", pad_address(setoken));
    if let Ok(hex) = eth_call(rpc_url, oracle, &data).await {
        return decode_uint256(&hex);
    }
    0
}

/// Convert per-block rate to annualized APY percentage
pub fn rate_to_apy(rate_per_block: u128, blocks_per_year: u64) -> f64 {
    let rate_f = rate_per_block as f64 / 1e18;
    rate_f * blocks_per_year as f64 * 100.0
}
