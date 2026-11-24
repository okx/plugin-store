/// EVM JSON-RPC helpers + Compound V2 contract reads.
///
/// Knowledge-base compliance:
///   - EVM-006: wait_for_tx polls eth_getTransactionReceipt, no blind sleep
///   - EVM-012: RPC failures bubble as Err, never silently zero out
///   - TX-001: wait_for_tx confirms `status == 0x1` before reporting success

/// Function selectors used by this skill — all keccak256 verified.
pub mod selectors {
    // ERC-20 (also used for cTokens since they extend ERC-20)
    pub const BALANCE_OF: &str = "0x70a08231";
    pub const ALLOWANCE:  &str = "0xdd62ed3e";
    pub const APPROVE:    &str = "0x095ea7b3";
    pub const DECIMALS:   &str = "0x313ce567";
    pub const SYMBOL:     &str = "0x95d89b41";

    // cToken write functions (CErc20 + CEther)
    pub const MINT_ERC20:        &str = "0xa0712d68";  // mint(uint256) — for ERC20 underlying cTokens
    pub const MINT_NATIVE:       &str = "0x1249c58b";  // mint() — for cETH (payable, no args)
    pub const REDEEM:            &str = "0xdb006a75";  // redeem(uint256 cTokenAmount)
    pub const REDEEM_UNDERLYING: &str = "0x852a12e3";  // redeemUnderlying(uint256 underlyingAmount)
    pub const BORROW:            &str = "0xc5ebeaec";  // borrow(uint256)
    pub const REPAY_BORROW:      &str = "0x0e752702";  // repayBorrow(uint256) — pass uint256.max for repay-all (LEND-001)

    // cToken read functions
    pub const BALANCE_OF_UNDERLYING:   &str = "0x3af9e669";  // balanceOfUnderlying(address) — view-after-accrue
    pub const BORROW_BALANCE_CURRENT:  &str = "0x17bfdfbc";  // borrowBalanceCurrent(address) — view-after-accrue
    pub const BORROW_BALANCE_STORED:   &str = "0x95dd9193";  // borrowBalanceStored(address)
    pub const SUPPLY_RATE_PER_BLOCK:   &str = "0xae9d70b0";  // supplyRatePerBlock()
    pub const BORROW_RATE_PER_BLOCK:   &str = "0xf8f9da28";  // borrowRatePerBlock()
    pub const EXCHANGE_RATE_STORED:    &str = "0x182df0f5";  // exchangeRateStored()
    pub const TOTAL_BORROWS:           &str = "0x47bd3718";  // totalBorrows()
    pub const TOTAL_SUPPLY:            &str = "0x18160ddd";  // totalSupply() — in cToken units (8 dec)
    pub const TOTAL_RESERVES:          &str = "0x8f840ddd";  // totalReserves()
    pub const GET_CASH:                &str = "0x3b1d21a2";  // getCash() — underlying liquidity
    pub const UNDERLYING:              &str = "0x6f307dc3";  // underlying() — ERC-20 addr (cETH lacks this)

    // Comptroller (Unitroller proxy delegating to current implementation)
    pub const ENTER_MARKETS:           &str = "0xc2998238";  // enterMarkets(address[])
    pub const EXIT_MARKET:             &str = "0xede4edd0";  // exitMarket(address)
    pub const CLAIM_COMP_HOLDER_LIST:  &str = "0x1c3db2e0";  // claimComp(address holder, address[] cTokens)
    pub const COMP_ACCRUED:            &str = "0xcc7ebdc4";  // compAccrued(address)
    pub const COMP_SUPPLY_SPEEDS:      &str = "0x6aa875b5";  // compSupplySpeeds(address cToken)
    pub const COMP_BORROW_SPEEDS:      &str = "0xf4a433c0";  // compBorrowSpeeds(address cToken)
    pub const GET_ALL_MARKETS:         &str = "0xb0772d0b";  // getAllMarkets()
    pub const GET_ACCOUNT_LIQUIDITY:   &str = "0x5ec88c79";  // getAccountLiquidity(address) → (err, liquidity, shortfall) — 1e18 scaled USD
    pub const GET_ASSETS_IN:           &str = "0xabfceffc";  // getAssetsIn(address) → cTokens entered
    pub const MARKETS:                 &str = "0x8e8f294b";  // markets(address) → (isListed, collateralFactor, isComped)
    pub const MINT_GUARDIAN_PAUSED:    &str = "0x731f0c2b";  // mintGuardianPaused(address cToken)
    pub const BORROW_GUARDIAN_PAUSED:  &str = "0x6d154ea5";  // borrowGuardianPaused(address cToken)
}

pub fn pad_address(addr: &str) -> String {
    let a = addr.trim_start_matches("0x");
    format!("{:0>64}", a)
}

pub fn pad_u256(v: u128) -> String {
    format!("{:064x}", v)
}

/// Build a uint256 max sentinel — used for `repayBorrow(uint256.max)` (LEND-001 dust-free).
pub fn pad_u256_max() -> String {
    "f".repeat(64)
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

fn parse_u128_word(hex: &str) -> u128 {
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.is_empty() { return 0; }
    let take = trimmed.len().saturating_sub(32);
    u128::from_str_radix(&trimmed[take..], 16).unwrap_or(0)
}

fn parse_address_word(hex: &str) -> String {
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.len() < 40 {
        return "0x0000000000000000000000000000000000000000".to_string();
    }
    let take = trimmed.len().saturating_sub(40);
    format!("0x{}", &trimmed[take..])
}

/// Decode an `address[]` ABI return: offset(32) + length(32) + addr × N (each padded to 32).
fn parse_address_array(hex: &str) -> Vec<String> {
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.len() < 128 { return Vec::new(); }
    let length = u128::from_str_radix(&trimmed[64..128], 16).unwrap_or(0) as usize;
    let mut out = Vec::with_capacity(length);
    for i in 0..length {
        let off = 128 + i * 64;
        if off + 64 > trimmed.len() { break; }
        out.push(parse_address_word(&trimmed[off..off+64]));
    }
    out
}

// ── ERC-20 / cToken ERC-20 reads ─────────────────────────────────────────────

pub async fn erc20_balance(token: &str, owner: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("{}{}", selectors::BALANCE_OF, pad_address(owner));
    let hex = eth_call(rpc, token, &data).await
        .map_err(|e| anyhow::anyhow!("erc20 balanceOf({}): {}", token, e))?;
    Ok(parse_u128_word(&hex))
}

pub async fn erc20_allowance(token: &str, owner: &str, spender: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("{}{}{}",
        selectors::ALLOWANCE, pad_address(owner), pad_address(spender));
    let hex = eth_call(rpc, token, &data).await
        .map_err(|e| anyhow::anyhow!("erc20 allowance: {}", e))?;
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
        .map_err(|e| anyhow::anyhow!("eth_getBalance HTTP: {}", e))?
        .json().await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_getBalance rpc error: {}", err);
    }
    let hex = resp["result"].as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_getBalance missing result"))?;
    Ok(parse_u128_word(hex))
}

// ── Compound V2 cToken reads ─────────────────────────────────────────────────

/// User's underlying balance held in cToken (post-accrual).
/// Computed by cToken as `balanceOf × exchangeRateCurrent / 1e18`.
/// Note: this is a non-view in Solidity but eth_call simulates accrual transparently.
pub async fn balance_of_underlying(ctoken: &str, account: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("{}{}", selectors::BALANCE_OF_UNDERLYING, pad_address(account));
    let hex = eth_call(rpc, ctoken, &data).await
        .map_err(|e| anyhow::anyhow!("balanceOfUnderlying: {}", e))?;
    Ok(parse_u128_word(&hex))
}

/// User's current borrow balance (post-accrual). underlying-token units.
pub async fn borrow_balance_current(ctoken: &str, account: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("{}{}", selectors::BORROW_BALANCE_CURRENT, pad_address(account));
    let hex = eth_call(rpc, ctoken, &data).await
        .map_err(|e| anyhow::anyhow!("borrowBalanceCurrent: {}", e))?;
    Ok(parse_u128_word(&hex))
}

pub async fn supply_rate_per_block(ctoken: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, ctoken, selectors::SUPPLY_RATE_PER_BLOCK).await?;
    Ok(parse_u128_word(&hex))
}

pub async fn borrow_rate_per_block(ctoken: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, ctoken, selectors::BORROW_RATE_PER_BLOCK).await?;
    Ok(parse_u128_word(&hex))
}

pub async fn exchange_rate_stored(ctoken: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, ctoken, selectors::EXCHANGE_RATE_STORED).await?;
    Ok(parse_u128_word(&hex))
}

pub async fn total_borrows(ctoken: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, ctoken, selectors::TOTAL_BORROWS).await?;
    Ok(parse_u128_word(&hex))
}

pub async fn ctoken_total_supply(ctoken: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, ctoken, selectors::TOTAL_SUPPLY).await?;
    Ok(parse_u128_word(&hex))
}

pub async fn get_cash(ctoken: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, ctoken, selectors::GET_CASH).await?;
    Ok(parse_u128_word(&hex))
}

// ── Comptroller reads ────────────────────────────────────────────────────────

pub async fn get_assets_in(comptroller: &str, account: &str, rpc: &str) -> anyhow::Result<Vec<String>> {
    let data = format!("{}{}", selectors::GET_ASSETS_IN, pad_address(account));
    let hex = eth_call(rpc, comptroller, &data).await
        .map_err(|e| anyhow::anyhow!("getAssetsIn: {}", e))?;
    Ok(parse_address_array(&hex))
}

pub async fn is_in_market(comptroller: &str, account: &str, ctoken: &str, rpc: &str) -> anyhow::Result<bool> {
    let assets = get_assets_in(comptroller, account, rpc).await?;
    Ok(assets.iter().any(|a| a.eq_ignore_ascii_case(ctoken)))
}

pub async fn is_mint_paused(comptroller: &str, ctoken: &str, rpc: &str) -> anyhow::Result<bool> {
    let data = format!("{}{}", selectors::MINT_GUARDIAN_PAUSED, pad_address(ctoken));
    let hex = eth_call(rpc, comptroller, &data).await
        .map_err(|e| anyhow::anyhow!("mintGuardianPaused: {}", e))?;
    Ok(parse_u128_word(&hex) != 0)
}

pub async fn is_borrow_paused(comptroller: &str, ctoken: &str, rpc: &str) -> anyhow::Result<bool> {
    let data = format!("{}{}", selectors::BORROW_GUARDIAN_PAUSED, pad_address(ctoken));
    let hex = eth_call(rpc, comptroller, &data).await
        .map_err(|e| anyhow::anyhow!("borrowGuardianPaused: {}", e))?;
    Ok(parse_u128_word(&hex) != 0)
}

/// Returns (isListed, collateralFactorMantissa_1e18, isComped).
pub async fn get_market(comptroller: &str, ctoken: &str, rpc: &str) -> anyhow::Result<(bool, u128, bool)> {
    let data = format!("{}{}", selectors::MARKETS, pad_address(ctoken));
    let hex = eth_call(rpc, comptroller, &data).await?;
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.len() < 192 { return Ok((false, 0, false)); }
    let is_listed = u128::from_str_radix(&trimmed[..64], 16).unwrap_or(0) != 0;
    let cf = u128::from_str_radix(&trimmed[64..128], 16).unwrap_or(0);
    let is_comped = u128::from_str_radix(&trimmed[128..192], 16).unwrap_or(0) != 0;
    Ok((is_listed, cf, is_comped))
}

/// Returns (errorCode, liquidity_1e18, shortfall_1e18). errorCode 0 = OK.
pub async fn get_account_liquidity(comptroller: &str, account: &str, rpc: &str) -> anyhow::Result<(u128, u128, u128)> {
    let data = format!("{}{}", selectors::GET_ACCOUNT_LIQUIDITY, pad_address(account));
    let hex = eth_call(rpc, comptroller, &data).await?;
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.len() < 192 { return Ok((0, 0, 0)); }
    let err = u128::from_str_radix(&trimmed[..64], 16).unwrap_or(0);
    let liquidity = u128::from_str_radix(&trimmed[64..128], 16).unwrap_or(0);
    let shortfall = u128::from_str_radix(&trimmed[128..192], 16).unwrap_or(0);
    Ok((err, liquidity, shortfall))
}

pub async fn get_comp_accrued(comptroller: &str, account: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("{}{}", selectors::COMP_ACCRUED, pad_address(account));
    let hex = eth_call(rpc, comptroller, &data).await?;
    Ok(parse_u128_word(&hex))
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
                            "tx {} mined but reverted (status 0x0). Inspect on Etherscan.",
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

/// Convert a Compound V2 per-block rate (1e18-scaled) to APR decimal.
/// APR = ratePerBlock × blocks_per_year / 1e18.
/// For continuous compounding APY ≈ APR for small rates; we report APR (matches Compound UI).
pub fn rate_per_block_to_apr(rate_1e18: u128, blocks_per_year: u128) -> f64 {
    if rate_1e18 == 0 { return 0.0; }
    // Use f64 product; for typical rates (<1e15) and 2_102_400 blocks this fits comfortably.
    let per_block = rate_1e18 as f64 / 1e18;
    per_block * blocks_per_year as f64
}

#[cfg(test)]
mod tests {
    use super::selectors::*;
    use sha3::{Digest, Keccak256};

    fn sel(sig: &str) -> String {
        let h = Keccak256::digest(sig.as_bytes());
        format!("0x{}", hex::encode(&h[..4]))
    }

    /// Recompute every selector via keccak256 at runtime so any copy/paste typo
    /// would fail this test instead of silently misrouting calls on-chain.
    /// Pattern matches euler-v2 / aave-v2 / fourmeme.
    #[test]
    fn selectors_match_keccak256() {
        // ERC-20
        assert_eq!(sel("balanceOf(address)"),            BALANCE_OF);
        assert_eq!(sel("allowance(address,address)"),    ALLOWANCE);
        assert_eq!(sel("approve(address,uint256)"),      APPROVE);
        assert_eq!(sel("decimals()"),                    DECIMALS);
        assert_eq!(sel("symbol()"),                      SYMBOL);
        // cToken writes
        assert_eq!(sel("mint(uint256)"),                 MINT_ERC20);
        assert_eq!(sel("mint()"),                        MINT_NATIVE);
        assert_eq!(sel("redeem(uint256)"),               REDEEM);
        assert_eq!(sel("redeemUnderlying(uint256)"),     REDEEM_UNDERLYING);
        assert_eq!(sel("borrow(uint256)"),               BORROW);
        assert_eq!(sel("repayBorrow(uint256)"),          REPAY_BORROW);
        // cToken views
        assert_eq!(sel("balanceOfUnderlying(address)"),  BALANCE_OF_UNDERLYING);
        assert_eq!(sel("borrowBalanceCurrent(address)"), BORROW_BALANCE_CURRENT);
        assert_eq!(sel("borrowBalanceStored(address)"),  BORROW_BALANCE_STORED);
        assert_eq!(sel("supplyRatePerBlock()"),          SUPPLY_RATE_PER_BLOCK);
        assert_eq!(sel("borrowRatePerBlock()"),          BORROW_RATE_PER_BLOCK);
        assert_eq!(sel("exchangeRateStored()"),          EXCHANGE_RATE_STORED);
        assert_eq!(sel("totalBorrows()"),                TOTAL_BORROWS);
        assert_eq!(sel("totalSupply()"),                 TOTAL_SUPPLY);
        assert_eq!(sel("totalReserves()"),               TOTAL_RESERVES);
        assert_eq!(sel("getCash()"),                     GET_CASH);
        assert_eq!(sel("underlying()"),                  UNDERLYING);
        // Comptroller
        assert_eq!(sel("enterMarkets(address[])"),       ENTER_MARKETS);
        assert_eq!(sel("exitMarket(address)"),           EXIT_MARKET);
        assert_eq!(sel("claimComp(address,address[])"),  CLAIM_COMP_HOLDER_LIST);
        assert_eq!(sel("compAccrued(address)"),          COMP_ACCRUED);
        assert_eq!(sel("compSupplySpeeds(address)"),     COMP_SUPPLY_SPEEDS);
        assert_eq!(sel("compBorrowSpeeds(address)"),     COMP_BORROW_SPEEDS);
        assert_eq!(sel("getAllMarkets()"),               GET_ALL_MARKETS);
        assert_eq!(sel("getAccountLiquidity(address)"),  GET_ACCOUNT_LIQUIDITY);
        assert_eq!(sel("getAssetsIn(address)"),          GET_ASSETS_IN);
        assert_eq!(sel("markets(address)"),              MARKETS);
        assert_eq!(sel("mintGuardianPaused(address)"),   MINT_GUARDIAN_PAUSED);
        assert_eq!(sel("borrowGuardianPaused(address)"), BORROW_GUARDIAN_PAUSED);
    }
}
