/// EVM JSON-RPC helpers: ERC-20 reads, ERC-4626 vault reads (sUSDS), eth_getBalance,
/// wait_for_tx polling. All chains we support are EVM and use the same RPC interface.
///
/// Knowledge base compliance:
///   - EVM-006: wait_for_tx polls eth_getTransactionReceipt, no blind sleep
///   - EVM-012: RPC failures bubble as Err — we never silently zero-out

/// Standard ERC-20 + ERC-4626 selectors used by this skill.
pub mod selectors {
    pub const BALANCE_OF:        &str = "0x70a08231";
    pub const ALLOWANCE:         &str = "0xdd62ed3e";
    pub const APPROVE:           &str = "0x095ea7b3";
    #[allow(dead_code)]
    pub const ASSET:             &str = "0x38d52e0f"; // ERC-4626 asset() → underlying token
    pub const TOTAL_ASSETS:      &str = "0x01e1d114"; // ERC-4626 totalAssets() → vault TVL
    pub const CONVERT_TO_ASSETS: &str = "0x07a2d13a"; // ERC-4626 convertToAssets(uint256 shares)
    pub const PREVIEW_DEPOSIT:   &str = "0xef8b30f7"; // ERC-4626 previewDeposit(uint256 assets)
    pub const PREVIEW_REDEEM:    &str = "0x4cdad506"; // ERC-4626 previewRedeem(uint256 shares)
    /// sUSDS-specific: ssr() returns the per-second savings rate as a ray (1e27).
    /// chi() returns the cumulative rate index. Both used to compute APY.
    /// Selectors computed via keccak256 of the function signature.
    pub const SSR:               &str = "0x03607ceb"; // ssr()
    pub const CHI:               &str = "0xc92aecc4"; // chi()
    /// PSM swapExactIn(address,address,uint256,uint256,address,uint256) — used on Base/Arbitrum
    pub const PSM_SWAP_EXACT_IN: &str = "0x1a019e37";
    /// DaiUsds.daiToUsds(address,uint256) — Ethereum-only migrator
    pub const DAI_TO_USDS:       &str = "0xf2c07aae";
}

pub fn pad_address(addr: &str) -> String {
    let a = addr.trim_start_matches("0x");
    format!("{:0>64}", a)
}

pub fn pad_u256(v: u128) -> String {
    format!("{:064x}", v)
}

async fn eth_call(rpc: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"]
    });
    let resp: serde_json::Value = client
        .post(rpc).json(&body).send().await
        .map_err(|e| anyhow::anyhow!("eth_call HTTP failed: {}", e))?
        .json().await
        .map_err(|e| anyhow::anyhow!("eth_call JSON parse failed: {}", e))?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call rpc error: {}", err);
    }
    Ok(resp["result"].as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_call missing result field"))?
        .to_string())
}

/// Decode last 32 bytes of a hex word as u128 (truncates the high half if value > u128).
fn parse_u128_word(hex: &str) -> u128 {
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.is_empty() { return 0; }
    let take = trimmed.len().saturating_sub(32);
    u128::from_str_radix(&trimmed[take..], 16).unwrap_or(0)
}

/// ERC-20 balanceOf(address) → u128 atomic units.
pub async fn erc20_balance(token: &str, owner: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("{}{}", selectors::BALANCE_OF, pad_address(owner));
    let hex = eth_call(rpc, token, &data).await
        .map_err(|e| anyhow::anyhow!("erc20 balanceOf({}) on {} failed: {}", token, rpc, e))?;
    Ok(parse_u128_word(&hex))
}

/// ERC-20 allowance(owner, spender) → u128.
pub async fn erc20_allowance(token: &str, owner: &str, spender: &str, rpc: &str)
    -> anyhow::Result<u128>
{
    let data = format!("{}{}{}", selectors::ALLOWANCE, pad_address(owner), pad_address(spender));
    let hex = eth_call(rpc, token, &data).await
        .map_err(|e| anyhow::anyhow!("erc20 allowance failed: {}", e))?;
    Ok(parse_u128_word(&hex))
}

/// Native balance via eth_getBalance.
pub async fn native_balance(addr: &str, rpc: &str) -> anyhow::Result<u128> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "eth_getBalance",
        "params": [addr, "latest"]
    });
    let resp: serde_json::Value = client
        .post(rpc).json(&body).send().await
        .map_err(|e| anyhow::anyhow!("eth_getBalance HTTP failed: {}", e))?
        .json().await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_getBalance rpc error: {}", err);
    }
    let hex = resp["result"].as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_getBalance missing result"))?;
    Ok(parse_u128_word(hex))
}

/// ERC-4626 totalAssets() — total underlying assets in the vault.
pub async fn vault_total_assets(vault: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, vault, selectors::TOTAL_ASSETS).await?;
    Ok(parse_u128_word(&hex))
}

/// ERC-4626 convertToAssets(shares) — value of `shares` in underlying-token atomic units.
pub async fn vault_convert_to_assets(vault: &str, shares: u128, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("{}{}", selectors::CONVERT_TO_ASSETS, pad_u256(shares));
    let hex = eth_call(rpc, vault, &data).await?;
    Ok(parse_u128_word(&hex))
}

/// ERC-4626 previewDeposit(assets) — shares minted for given asset deposit.
pub async fn vault_preview_deposit(vault: &str, assets: u128, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("{}{}", selectors::PREVIEW_DEPOSIT, pad_u256(assets));
    let hex = eth_call(rpc, vault, &data).await?;
    Ok(parse_u128_word(&hex))
}

/// ERC-4626 previewRedeem(shares) — assets received for given share redemption.
pub async fn vault_preview_redeem(vault: &str, shares: u128, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("{}{}", selectors::PREVIEW_REDEEM, pad_u256(shares));
    let hex = eth_call(rpc, vault, &data).await?;
    Ok(parse_u128_word(&hex))
}

/// sUSDS ssr() — Sky Savings Rate as a per-second compounding ray (1e27 fixed-point).
/// Returns the raw ray value; caller computes APY = (ssr / 1e27)^seconds_per_year - 1.
pub async fn susds_ssr(susds: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, susds, selectors::SSR).await?;
    Ok(parse_u128_word(&hex))
}

/// sUSDS chi() — current cumulative rate index. Mainly for diagnostic / verification.
pub async fn susds_chi(susds: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, susds, selectors::CHI).await?;
    Ok(parse_u128_word(&hex))
}

/// Poll eth_getTransactionReceipt until the tx is mined OR timeout.
/// Returns Ok(()) on status=0x1, Err on status=0x0 (reverted) or timeout.
/// Knowledge base EVM-006: never use blind sleep.
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
            "jsonrpc": "2.0", "id": 1, "method": "eth_getTransactionReceipt",
            "params": [tx_hash]
        });
        if let Ok(r) = client.post(rpc).json(&body).send().await {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                if v["result"].is_object() {
                    match v["result"]["status"].as_str().unwrap_or("") {
                        "0x1" => return Ok(()),
                        "0x0" => anyhow::bail!(
                            "tx {} mined but reverted (status 0x0). Inspect on the block explorer for revert reason.",
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

/// Format a u128 atomic amount with the given decimals. Trims trailing zeros.
pub fn fmt_token_amount(raw: u128, decimals: u32) -> String {
    if decimals == 0 { return raw.to_string(); }
    let factor = 10u128.pow(decimals);
    let whole = raw / factor;
    let frac = raw % factor;
    if frac == 0 { return whole.to_string(); }
    let frac_str = format!("{:0width$}", frac, width = decimals as usize);
    let trimmed = frac_str.trim_end_matches('0');
    if trimmed.is_empty() { whole.to_string() } else { format!("{}.{}", whole, trimmed) }
}

/// Convert a human number string (e.g. "100.5") into atomic u128 given decimals.
pub fn human_to_atomic(s: &str, decimals: u32) -> Result<u128, String> {
    let f: f64 = s.parse().map_err(|_| "not a number".to_string())?;
    if f <= 0.0 || !f.is_finite() {
        return Err("must be a positive finite number".to_string());
    }
    let scaled = f * 10f64.powi(decimals as i32);
    if scaled > u128::MAX as f64 {
        return Err("amount exceeds u128".to_string());
    }
    let atomic = scaled.round() as u128;
    if atomic == 0 {
        return Err(format!("amount too small for {} decimals", decimals));
    }
    Ok(atomic)
}

/// Build calldata for ERC20.approve(spender, type(uint256).max).
pub fn build_approve_max(spender: &str) -> String {
    format!("0x{}{}{}", selectors::APPROVE.trim_start_matches("0x"), pad_address(spender), "f".repeat(64))
}

/// Compute APY (decimal, e.g. 0.075 = 7.5%) from sUSDS `ssr` ray.
/// ssr is a per-second compounding rate stored as a ray (1e27). Sky uses
/// continuous-compounding semantics; we compute (ssr/1e27)^seconds_per_year - 1.
pub fn ssr_to_apy(ssr_ray: u128) -> f64 {
    const RAY: f64 = 1e27;
    const SECS_PER_YEAR: f64 = 365.0 * 86400.0;
    if ssr_ray == 0 { return 0.0; }
    let per_second = ssr_ray as f64 / RAY;
    if per_second <= 1.0 { return 0.0; }
    per_second.powf(SECS_PER_YEAR) - 1.0
}

#[cfg(test)]
mod tests {
    use super::selectors::*;
    use sha3::{Digest, Keccak256};

    fn sel(sig: &str) -> String {
        let h = Keccak256::digest(sig.as_bytes());
        format!("0x{}", hex::encode(&h[..4]))
    }

    #[test]
    fn selectors_match_keccak256() {
        assert_eq!(sel("balanceOf(address)"),         BALANCE_OF);
        assert_eq!(sel("allowance(address,address)"), ALLOWANCE);
        assert_eq!(sel("approve(address,uint256)"),   APPROVE);
        assert_eq!(sel("asset()"),                    ASSET);
        assert_eq!(sel("totalAssets()"),              TOTAL_ASSETS);
        assert_eq!(sel("convertToAssets(uint256)"),   CONVERT_TO_ASSETS);
        assert_eq!(sel("previewDeposit(uint256)"),    PREVIEW_DEPOSIT);
        assert_eq!(sel("previewRedeem(uint256)"),     PREVIEW_REDEEM);
        assert_eq!(sel("ssr()"),                      SSR);
        assert_eq!(sel("chi()"),                      CHI);
        assert_eq!(
            sel("swapExactIn(address,address,uint256,uint256,address,uint256)"),
            PSM_SWAP_EXACT_IN
        );
        assert_eq!(sel("daiToUsds(address,uint256)"), DAI_TO_USDS);
    }
}
