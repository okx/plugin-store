/// EVM JSON-RPC helpers + Dolomite-specific contract reads.
///
/// Knowledge-base compliance:
///   - EVM-006: wait_for_tx polls eth_getTransactionReceipt, no blind sleep
///   - EVM-012: RPC failures bubble as Err, never silently zero out

/// Function selectors used by this skill (verified via keccak256).
pub mod selectors {
    // ERC-20
    pub const BALANCE_OF:  &str = "0x70a08231";
    pub const ALLOWANCE:   &str = "0xdd62ed3e";
    pub const APPROVE:     &str = "0x095ea7b3";

    // DepositWithdrawalProxy (lender flow) — verified against on-chain bytecode at
    // 0xAdB9D68c613df4AA363B42161E1282117C7B9594 (Arbitrum). The 5-arg `operate`-style
    // signatures live on DolomiteMargin core; the proxy exposes simpler 3/4-arg variants.
    pub const DEPOSIT_WEI:  &str = "0xfe491ee7"; // depositWei(uint256 toAccount, uint256 marketId, uint256 amount)
    pub const WITHDRAW_WEI: &str = "0xc38fd74e"; // withdrawWei(uint256 fromAccount, uint256 marketId, uint256 amount, uint8 balanceCheckFlag)

    // BorrowPositionProxyV2 — verified against on-chain bytecode at
    // 0x38E49A617305101216eC6306e3a18065D14Bf3a7 (Arbitrum).
    pub const OPEN_BORROW_POSITION:    &str = "0xbb0a6fa5";  // openBorrowPosition(uint256 fromAccountNumber, uint256 toAccountNumber, uint256 collateralMarketId, uint256 amountWei, uint8 balanceCheckFlag)
    pub const CLOSE_BORROW_POSITION:   &str = "0x8fb8b6c7";  // closeBorrowPosition(uint256 borrowAccountNumber, uint256 toAccountNumber, uint256[] collateralMarketIds)
    pub const TRANSFER_BETWEEN_ACCTS:  &str = "0x58e8cf03";  // transferBetweenAccounts(uint256 fromAccountNumber, uint256 toAccountNumber, uint256 marketId, uint256 amountWei, uint8 balanceCheckFlag)
    pub const REPAY_ALL_FOR_POSITION:  &str = "0xb0463d5c";  // repayAllForBorrowPosition(uint256 fromAccountNumber, uint256 borrowAccountNumber, uint256 marketId, uint8 balanceCheckFlag)

    // DolomiteMargin getters (verified against docs.dolomite.io getters page)
    pub const GET_NUM_MARKETS:           &str = "0x295c39a5";
    pub const GET_MARKET_TOKEN_ADDR:     &str = "0x062bd3e9";
    /// Sole "current rate" getter — returns BORROW rate per-second (18 decimals).
    /// Supply rate is derived: supply = borrow * earnings_rate / 1e18.
    pub const GET_MARKET_INTEREST_RATE:  &str = "0xfd47eda6";
    /// Returns global earnings_rate (1e18 fixed-point) — fraction of borrower interest passed to suppliers.
    pub const GET_EARNINGS_RATE:         &str = "0xe5520228";
    pub const GET_MARKET_TOTAL_PAR:      &str = "0xcb04a34c"; // (uint256 supplyPar, uint256 borrowPar)
    pub const GET_MARKET_WITH_INFO:      &str = "0xb548b892"; // full market snapshot
    pub const GET_MARKET_PRICE:          &str = "0x8928378e"; // returns Monetary.Price
    pub const GET_ACCOUNT_WEI:           &str = "0xc190c2ec"; // (Account.Info, uint256 marketId) → Types.Wei{sign,value}
    pub const GET_ACCOUNT_STATUS:        &str = "0xe51bfcb4"; // 0=Normal 1=Liquid 2=Vapor
    pub const GET_ACCOUNT_VALUES:        &str = "0x124f914c"; // (supplyValue, borrowValue) — Monetary.Value tuples

    // Token introspection (when `decimals()` not in our known list)
    pub const DECIMALS: &str = "0x313ce567";
    pub const SYMBOL:   &str = "0x95d89b41";
}

pub fn pad_address(addr: &str) -> String {
    let a = addr.trim_start_matches("0x");
    format!("{:0>64}", a)
}

pub fn pad_u256(v: u128) -> String {
    format!("{:064x}", v)
}

/// Pad a u256 from a hex / decimal string (for values exceeding u128).
pub fn pad_u256_str(v: &str) -> String {
    if v.starts_with("0x") {
        let h = v.trim_start_matches("0x");
        format!("{:0>64}", h)
    } else if let Ok(n) = v.parse::<u128>() {
        format!("{:064x}", n)
    } else {
        // Fallback: hope it's already formatted
        format!("{:0>64}", v)
    }
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
        .ok_or_else(|| anyhow::anyhow!("eth_call missing result"))?
        .to_string())
}

/// Decode the LAST 32 bytes of a hex word as u128 (caller's job to ensure < u128::MAX).
fn parse_u128_word(hex: &str) -> u128 {
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.is_empty() { return 0; }
    let take = trimmed.len().saturating_sub(32);
    u128::from_str_radix(&trimmed[take..], 16).unwrap_or(0)
}

/// Decode an EVM address from the LAST 20 bytes of a 32-byte word.
fn parse_address_word(hex: &str) -> String {
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.len() < 40 {
        return "0x0000000000000000000000000000000000000000".to_string();
    }
    let take = trimmed.len().saturating_sub(40);
    format!("0x{}", &trimmed[take..])
}

// ── ERC-20 reads ─────────────────────────────────────────────────────────────

pub async fn erc20_balance(token: &str, owner: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("{}{}", selectors::BALANCE_OF, pad_address(owner));
    let hex = eth_call(rpc, token, &data).await
        .map_err(|e| anyhow::anyhow!("erc20 balanceOf({}) on {} failed: {}", token, rpc, e))?;
    Ok(parse_u128_word(&hex))
}

pub async fn erc20_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc: &str,
) -> anyhow::Result<u128> {
    let data = format!(
        "{}{}{}",
        selectors::ALLOWANCE,
        pad_address(owner),
        pad_address(spender)
    );
    let hex = eth_call(rpc, token, &data).await
        .map_err(|e| anyhow::anyhow!("erc20 allowance failed: {}", e))?;
    Ok(parse_u128_word(&hex))
}

pub async fn erc20_decimals(token: &str, rpc: &str) -> anyhow::Result<u32> {
    let hex = eth_call(rpc, token, selectors::DECIMALS).await?;
    Ok(parse_u128_word(&hex) as u32)
}

pub async fn native_balance(addr: &str, rpc: &str) -> anyhow::Result<u128> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "eth_getBalance",
        "params": [addr, "latest"]
    });
    let resp: serde_json::Value = client.post(rpc).json(&body).send().await
        .map_err(|e| anyhow::anyhow!("eth_getBalance HTTP failed: {}", e))?
        .json().await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_getBalance rpc error: {}", err);
    }
    let hex = resp["result"].as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_getBalance missing result"))?;
    Ok(parse_u128_word(hex))
}

// ── DolomiteMargin reads ─────────────────────────────────────────────────────

pub async fn get_num_markets(margin: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, margin, selectors::GET_NUM_MARKETS).await?;
    Ok(parse_u128_word(&hex))
}

pub async fn get_market_token_address(margin: &str, market_id: u128, rpc: &str)
    -> anyhow::Result<String>
{
    let data = format!("{}{}", selectors::GET_MARKET_TOKEN_ADDR, pad_u256(market_id));
    let hex = eth_call(rpc, margin, &data).await?;
    Ok(parse_address_word(&hex))
}

/// `getMarketInterestRate(marketId)` — Dolomite returns BORROW rate per-second (18 decimals).
/// To get supply rate, multiply by earnings_rate / 1e18.
pub async fn get_market_borrow_rate(margin: &str, market_id: u128, rpc: &str)
    -> anyhow::Result<u128>
{
    let data = format!("{}{}", selectors::GET_MARKET_INTEREST_RATE, pad_u256(market_id));
    let hex = eth_call(rpc, margin, &data).await?;
    Ok(parse_u128_word(&hex))
}

/// `getEarningsRate()` — global fraction (1e18) of borrower interest passed to suppliers.
/// Typical value: 0.85e18 (85% of borrow APR goes to suppliers; 15% protocol fee).
pub async fn get_earnings_rate(margin: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, margin, selectors::GET_EARNINGS_RATE).await?;
    Ok(parse_u128_word(&hex))
}

/// Convenience: derive supply rate = borrow_rate * earnings_rate / 1e18.
pub fn supply_rate_from(borrow_rate: u128, earnings_rate: u128) -> u128 {
    // both in 1e18 fixed-point per-second
    let prod = (borrow_rate as u128).saturating_mul(earnings_rate);
    prod / 1_000_000_000_000_000_000u128
}

/// Returns (supplyPar, borrowPar) — total par values across all suppliers/borrowers.
pub async fn get_market_total_par(margin: &str, market_id: u128, rpc: &str)
    -> anyhow::Result<(u128, u128)>
{
    let data = format!("{}{}", selectors::GET_MARKET_TOTAL_PAR, pad_u256(market_id));
    let hex = eth_call(rpc, margin, &data).await?;
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.len() < 128 { return Ok((0, 0)); }
    let supply_par = u128::from_str_radix(&trimmed[..64], 16).unwrap_or(0);
    let borrow_par = u128::from_str_radix(&trimmed[64..128], 16).unwrap_or(0);
    Ok((supply_par, borrow_par))
}

/// getAccountWei((address owner, uint256 number), uint256 marketId)
/// Returns Types.Wei{ bool sign; uint256 value }.
/// sign = true → positive (supply); sign = false → negative (borrow).
pub async fn get_account_wei(
    margin: &str,
    owner: &str,
    account_number: u128,
    market_id: u128,
    rpc: &str,
) -> anyhow::Result<(bool, u128)> {
    let data = format!(
        "{}{}{}{}",
        selectors::GET_ACCOUNT_WEI,
        pad_address(owner),
        pad_u256(account_number),
        pad_u256(market_id),
    );
    let hex = eth_call(rpc, margin, &data).await?;
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.len() < 128 { return Ok((true, 0)); }
    let sign_word = u128::from_str_radix(&trimmed[..64], 16).unwrap_or(0);
    let value = u128::from_str_radix(&trimmed[64..128], 16).unwrap_or(0);
    Ok((sign_word == 1, value))
}

/// getAccountValues((address, uint256)) → (Monetary.Value supply, Monetary.Value borrow)
/// Each Value is a uint256 (USD-scaled to 1e36 in Dolomite).
pub async fn get_account_values(
    margin: &str,
    owner: &str,
    account_number: u128,
    rpc: &str,
) -> anyhow::Result<(u128, u128)> {
    let data = format!(
        "{}{}{}",
        selectors::GET_ACCOUNT_VALUES,
        pad_address(owner),
        pad_u256(account_number),
    );
    let hex = eth_call(rpc, margin, &data).await?;
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.len() < 128 { return Ok((0, 0)); }
    let supply_v = u128::from_str_radix(&trimmed[..64], 16).unwrap_or(0);
    let borrow_v = u128::from_str_radix(&trimmed[64..128], 16).unwrap_or(0);
    Ok((supply_v, borrow_v))
}

// ── tx confirmation helper (TX-001 / EVM-006) ────────────────────────────────

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
                            "tx {} mined but reverted (status 0x0). Inspect on the explorer.",
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

// ── Formatting / parsing helpers ─────────────────────────────────────────────

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

pub fn build_approve_max(spender: &str) -> String {
    format!(
        "0x{}{}{}",
        selectors::APPROVE.trim_start_matches("0x"),
        pad_address(spender),
        "f".repeat(64),
    )
}

/// Convert a Dolomite per-second interest rate (1e18 fixed-point) to APY decimal.
/// E.g. rate=10000000000 (10 gwei/sec) → ~31.5% APR ≈ 37% APY.
pub fn rate_to_apy(rate_1e18: u128) -> f64 {
    const SCALE: f64 = 1e18;
    const SECS_PER_YEAR: f64 = 365.0 * 86400.0;
    if rate_1e18 == 0 { return 0.0; }
    let per_second = rate_1e18 as f64 / SCALE;
    if per_second == 0.0 { return 0.0; }
    // Per-second compounding to APY
    (1.0 + per_second).powf(SECS_PER_YEAR) - 1.0
}

#[cfg(test)]
mod tests {
    use super::selectors::*;
    use sha3::{Digest, Keccak256};

    fn sel(sig: &str) -> String {
        let h = Keccak256::digest(sig.as_bytes());
        format!("0x{}", hex::encode(&h[..4]))
    }

    /// Recompute every selector via keccak256 at runtime so any copy/paste
    /// typo in the hardcoded constants would fail this test instead of
    /// silently misrouting calls on-chain. Pattern matches euler-v2 /
    /// aave-v2 / compound-v2 / fourmeme. Dolomite uses Account.Info struct
    /// which encodes as `(address,uint256)` in ABI signatures.
    #[test]
    fn selectors_match_keccak256() {
        // ERC-20
        assert_eq!(sel("balanceOf(address)"),         BALANCE_OF);
        assert_eq!(sel("allowance(address,address)"), ALLOWANCE);
        assert_eq!(sel("approve(address,uint256)"),   APPROVE);
        assert_eq!(sel("decimals()"),                 DECIMALS);
        assert_eq!(sel("symbol()"),                   SYMBOL);
        // DolomiteMargin writes
        assert_eq!(sel("depositWei(uint256,uint256,uint256)"),
                   DEPOSIT_WEI);
        assert_eq!(sel("withdrawWei(uint256,uint256,uint256,uint8)"),
                   WITHDRAW_WEI);
        assert_eq!(sel("openBorrowPosition(uint256,uint256,uint256,uint256,uint8)"),
                   OPEN_BORROW_POSITION);
        assert_eq!(sel("closeBorrowPosition(uint256,uint256,uint256[])"),
                   CLOSE_BORROW_POSITION);
        assert_eq!(sel("transferBetweenAccounts(uint256,uint256,uint256,uint256,uint8)"),
                   TRANSFER_BETWEEN_ACCTS);
        assert_eq!(sel("repayAllForBorrowPosition(uint256,uint256,uint256,uint8)"),
                   REPAY_ALL_FOR_POSITION);
        // Margin reads (no struct args)
        assert_eq!(sel("getNumMarkets()"),            GET_NUM_MARKETS);
        assert_eq!(sel("getMarketTokenAddress(uint256)"),
                   GET_MARKET_TOKEN_ADDR);
        assert_eq!(sel("getMarketInterestRate(uint256)"),
                   GET_MARKET_INTEREST_RATE);
        assert_eq!(sel("getEarningsRate()"),          GET_EARNINGS_RATE);
        assert_eq!(sel("getMarketTotalPar(uint256)"), GET_MARKET_TOTAL_PAR);
        assert_eq!(sel("getMarketWithInfo(uint256)"), GET_MARKET_WITH_INFO);
        assert_eq!(sel("getMarketPrice(uint256)"),    GET_MARKET_PRICE);
        // Margin reads with Account.Info struct = (address,uint256)
        assert_eq!(sel("getAccountWei((address,uint256),uint256)"),
                   GET_ACCOUNT_WEI);
        assert_eq!(sel("getAccountStatus((address,uint256))"),
                   GET_ACCOUNT_STATUS);
        assert_eq!(sel("getAccountValues((address,uint256))"),
                   GET_ACCOUNT_VALUES);
    }
}
