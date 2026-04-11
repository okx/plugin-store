// rpc.rs — Direct eth_call utilities (no onchainos)

/// Parse a human-readable decimal amount string into minimal units (u128).
///
/// Examples (decimals=6):
///   "1.0"  → 1_000_000
///   "0.5"  → 500_000
///   "1000" → 1_000_000_000_000
pub fn parse_human_amount(amount_str: &str, decimals: u8) -> anyhow::Result<u128> {
    let s = amount_str.trim();
    let factor = 10u128.pow(decimals as u32);
    if let Some(dot_pos) = s.find('.') {
        let int_part: u128 = if dot_pos == 0 {
            0
        } else {
            s[..dot_pos]
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?
        };
        let frac_str = &s[dot_pos + 1..];
        if frac_str.len() > decimals as usize {
            anyhow::bail!(
                "Amount '{}' has {} decimal places but token only supports {}",
                s,
                frac_str.len(),
                decimals
            );
        }
        let frac: u128 = if frac_str.is_empty() {
            0
        } else {
            frac_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?
        };
        let frac_factor = 10u128.pow(decimals as u32 - frac_str.len() as u32);
        Ok(int_part * factor + frac * frac_factor)
    } else {
        let int_val: u128 = s
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        Ok(int_val * factor)
    }
}

/// Perform a raw JSON-RPC eth_call
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            {"to": to, "data": data},
            "latest"
        ],
        "id": 1
    });
    let resp: serde_json::Value = client.post(rpc_url).json(&body).send().await?.json().await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Resolve LP token address for a Curve pool via pool.token() (selector 0xfc0c546a).
/// Factory-crypto pools have a separate LP token contract; for other pools this
/// either returns the pool address itself or fails — fall back to pool address on error.
pub async fn lp_token_address(pool_addr: &str, rpc_url: &str) -> String {
    match eth_call(pool_addr, "0xfc0c546a", rpc_url).await {
        Ok(hex) => {
            let raw = hex.trim_start_matches("0x");
            if raw.len() >= 40 {
                let addr = format!("0x{}", &raw[raw.len()-40..]);
                // If the result is zero address, fall back to pool address
                if addr == "0x0000000000000000000000000000000000000000" {
                    pool_addr.to_string()
                } else {
                    addr
                }
            } else {
                pool_addr.to_string()
            }
        }
        Err(_) => pool_addr.to_string(),
    }
}

/// decimals() for an ERC-20 (selector 0x313ce567). Returns 18 on failure.
pub async fn decimals(token_addr: &str, rpc_url: &str) -> u8 {
    match eth_call(token_addr, "0x313ce567", rpc_url).await {
        Ok(hex) => {
            let raw = hex.trim_start_matches("0x");
            u8::from_str_radix(&raw[raw.len().saturating_sub(2)..], 16).unwrap_or(18)
        }
        Err(_) => 18,
    }
}

/// balanceOf(address) for an ERC-20 (selector 0x70a08231)
pub async fn balance_of(token: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let owner_clean = owner.trim_start_matches("0x");
    let owner_padded = format!("{:0>64}", owner_clean);
    let data = format!("0x70a08231{}", owner_padded);
    let hex = eth_call(token, &data, rpc_url).await?;
    Ok(u128::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0))
}

/// allowance(address owner, address spender) selector = 0xdd62ed3e
pub async fn get_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let owner_clean = owner.trim_start_matches("0x");
    let spender_clean = spender.trim_start_matches("0x");
    let owner_padded = format!("{:0>64}", owner_clean);
    let spender_padded = format!("{:0>64}", spender_clean);
    let data = format!("0xdd62ed3e{}{}", owner_padded, spender_padded);
    let hex = eth_call(token, &data, rpc_url).await?;
    Ok(u128::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0))
}

/// Decode a 32-byte ABI-encoded uint256 result to u128
pub fn decode_uint128(hex: &str) -> u128 {
    let clean = hex.trim_start_matches("0x");
    // take last 32 hex chars (16 bytes = u128 range)
    let last32 = if clean.len() >= 32 { &clean[clean.len() - 32..] } else { clean };
    u128::from_str_radix(last32, 16).unwrap_or(0)
}
