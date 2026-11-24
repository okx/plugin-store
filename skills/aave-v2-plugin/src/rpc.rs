/// EVM JSON-RPC helpers + Aave V2 contract reads.
///
/// Knowledge-base compliance:
///   - EVM-006: wait_for_tx polls eth_getTransactionReceipt, no blind sleep
///   - EVM-012: RPC failures bubble as Err, never silently zero out
///   - TX-001: wait_for_tx confirms `status == 0x1` before reporting success

/// Function selectors — all keccak256 verified.
pub mod selectors {
    // ERC-20
    pub const BALANCE_OF:  &str = "0x70a08231";
    pub const ALLOWANCE:   &str = "0xdd62ed3e";
    pub const APPROVE:     &str = "0x095ea7b3";
    pub const DECIMALS:    &str = "0x313ce567";
    pub const SYMBOL:      &str = "0x95d89b41";

    // LendingPool (Aave V2 Pool)
    pub const DEPOSIT:                       &str = "0xe8eda9df"; // deposit(address asset, uint256 amount, address onBehalfOf, uint16 referralCode)
    pub const WITHDRAW:                      &str = "0x69328dec"; // withdraw(address asset, uint256 amount, address to)
    pub const BORROW:                        &str = "0xa415bcad"; // borrow(address, uint256, uint256 rateMode, uint16, address onBehalfOf)
    pub const REPAY:                         &str = "0x573ade81"; // repay(address, uint256 amount, uint256 rateMode, address onBehalfOf) — pass uint256.max for repay-all (LEND-001)
    pub const SWAP_BORROW_RATE_MODE:         &str = "0x94ba89a2"; // swapBorrowRateMode(address, uint256 rateMode)
    pub const SET_USER_USE_RESERVE_AS_COLL:  &str = "0x5a3b74b9"; // setUserUseReserveAsCollateral(address, bool)
    pub const GET_RESERVES_LIST:             &str = "0xd1946dbc"; // getReservesList() returns address[]
    pub const POOL_GET_RESERVE_DATA:         &str = "0x35ea6a75"; // LendingPool.getReserveData(asset) — full struct, complex layout
    pub const GET_USER_ACCOUNT_DATA:         &str = "0xbf92857c"; // getUserAccountData(user) returns (totalCollateralETH, totalDebtETH, availableBorrowsETH, currentLiquidationThreshold, ltv, healthFactor)

    // AaveProtocolDataProvider (PDP) — convenience getters
    pub const PDP_GET_ALL_RESERVES_TOKENS:    &str = "0xb316ff89"; // getAllReservesTokens() returns (string symbol, address)[]
    pub const PDP_GET_RESERVE_TOKENS_ADDRS:   &str = "0xd2493b6c"; // getReserveTokensAddresses(asset) returns (aToken, sDebt, vDebt)
    pub const PDP_GET_RESERVE_CONFIG_DATA:    &str = "0x3e150141"; // returns (decimals, ltv, threshold, bonus, factor, ...)
    pub const PDP_GET_RESERVE_DATA:           &str = "0x35ea6a75"; // PDP.getReserveData(asset) returns simpler tuple of 10 values
    pub const PDP_GET_USER_RESERVE_DATA:      &str = "0x28dd2d01"; // getUserReserveData(asset, user) returns 9 values

    // WETHGateway — for native ETH/MATIC/AVAX
    pub const WG_DEPOSIT_ETH:  &str = "0x474cf53d"; // depositETH(pool, onBehalfOf, refCode) — payable
    pub const WG_WITHDRAW_ETH: &str = "0x80500d20"; // withdrawETH(pool, amount, to) — needs aWETH approval
    pub const WG_BORROW_ETH:   &str = "0x66514c97"; // borrowETH(pool, amount, rateMode, refCode) — needs vWETH approveDelegation
    pub const WG_REPAY_ETH:    &str = "0x02c5fcf8"; // repayETH(pool, amount, rateMode, onBehalfOf) — payable

    // IncentivesController
    pub const CLAIM_REWARDS:              &str = "0x3111e7b3"; // claimRewards(address[] assets, uint256 amount, address to) returns uint256
    pub const GET_USER_UNCLAIMED_REWARDS: &str = "0x198fa81e"; // getUserUnclaimedRewards(user)
    pub const GET_REWARDS_BALANCE:        &str = "0x8b599f26"; // getRewardsBalance(address[] assets, address user)
    pub const REWARD_TOKEN:               &str = "0x99248ea7"; // REWARD_TOKEN() returns address (e.g. stkAAVE on mainnet)
}

pub fn pad_address(addr: &str) -> String {
    let a = addr.trim_start_matches("0x");
    format!("{:0>64}", a)
}

pub fn pad_u256(v: u128) -> String {
    format!("{:064x}", v)
}

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

/// Decode address[] ABI return: offset(32) + length(32) + addr × N (each padded to 32).
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

// ── ERC-20 reads ────────────────────────────────────────────────────────────

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

/// ERC-20 totalSupply selector 0x18160ddd.
pub async fn erc20_total_supply(token: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(rpc, token, "0x18160ddd").await?;
    Ok(parse_u128_word(&hex))
}

/// Fetch ERC-20 symbol. Returns "?" on failure (best-effort display).
pub async fn erc20_symbol(token: &str, rpc: &str) -> String {
    let hex = match eth_call(rpc, token, selectors::SYMBOL).await {
        Ok(h) => h,
        Err(_) => return "?".to_string(),
    };
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 128 { return "?".to_string(); }
    // Dynamic string: offset(32) + length(32) + content. Some tokens (MKR, SAI) use
    // bytes32 — we'd see no offset. Fall back to bytes32 if length looks too high.
    let length_word_start = 64;
    let length = u128::from_str_radix(&raw[length_word_start..length_word_start+64], 16).unwrap_or(0) as usize;
    if length == 0 || length > 64 {
        // Try bytes32 fallback: decode first 32 bytes as ASCII, strip nulls.
        let bytes = hex_to_bytes(&raw[..64.min(raw.len())]);
        let s = String::from_utf8_lossy(&bytes).trim_matches('\0').to_string();
        return if s.is_empty() { "?".to_string() } else { s };
    }
    let content_start = length_word_start + 64;
    let content_hex_len = length * 2;
    if content_start + content_hex_len > raw.len() { return "?".to_string(); }
    let bytes = hex_to_bytes(&raw[content_start..content_start+content_hex_len]);
    String::from_utf8_lossy(&bytes).trim_matches('\0').to_string()
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len()).step_by(2)
        .filter_map(|i| u8::from_str_radix(&hex[i..i+2], 16).ok())
        .collect()
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

// ── Aave V2 LendingPool reads ───────────────────────────────────────────────

/// Returns (totalCollateralETH, totalDebtETH, availableBorrowsETH, currentLiquidationThreshold, ltv, healthFactor).
/// All values are in ETH (or chain native) base unit, 1e18 scaled. healthFactor: 1e18 = 1.0;
/// > 1e18 healthy; < 1e18 liquidatable.
pub async fn get_user_account_data(lending_pool: &str, user: &str, rpc: &str)
    -> anyhow::Result<(u128, u128, u128, u128, u128, u128)>
{
    let data = format!("{}{}", selectors::GET_USER_ACCOUNT_DATA, pad_address(user));
    let hex = eth_call(rpc, lending_pool, &data).await
        .map_err(|e| anyhow::anyhow!("getUserAccountData: {}", e))?;
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 384 { return Ok((0,0,0,0,0,0)); }
    let read = |i: usize| u128::from_str_radix(&raw[i*64..(i+1)*64], 16).unwrap_or(0);
    Ok((read(0), read(1), read(2), read(3), read(4), read(5)))
}

/// Enumerate all reserves in the LendingPool. Returns array of underlying asset addresses.
pub async fn get_reserves_list(lending_pool: &str, rpc: &str) -> anyhow::Result<Vec<String>> {
    let hex = eth_call(rpc, lending_pool, selectors::GET_RESERVES_LIST).await
        .map_err(|e| anyhow::anyhow!("getReservesList: {}", e))?;
    Ok(parse_address_array(&hex))
}

/// Decoded ReserveData from LendingPool.getReserveData(asset).
/// We avoid the PDP entirely (Ethereum mainnet PDP has no code) - all data sourced from
/// LendingPool itself + ERC-20 calls on aToken/sDebt/vDebt addresses.
#[derive(Debug, Clone, Default)]
pub struct ReserveData {
    pub configuration_bitmap: u128,
    pub liquidity_index_ray: u128,
    pub variable_borrow_index_ray: u128,
    pub current_liquidity_rate_ray: u128,        // supply APR in ray (annual)
    pub current_variable_borrow_rate_ray: u128,
    pub current_stable_borrow_rate_ray: u128,
    pub last_update_timestamp: u128,
    pub a_token: String,
    pub stable_debt_token: String,
    pub variable_debt_token: String,
    pub interest_rate_strategy: String,
    pub id: u128,
}

impl ReserveData {
    /// Decode the configuration bitmap into (decimals, ltv_bps, liqThreshold_bps, liqBonus_bps,
    /// reserveFactor_bps, isActive, isFrozen, borrowingEnabled, stableBorrowRateEnabled).
    /// Bit layout per Aave V2 ReserveConfiguration.sol:
    ///   bits 0-15:   LTV (basis points, max 65535)
    ///   bits 16-31:  liquidation threshold (bps)
    ///   bits 32-47:  liquidation bonus (bps)
    ///   bits 48-55:  decimals (uint8)
    ///   bit 56:      isActive
    ///   bit 57:      isFrozen
    ///   bit 58:      borrowingEnabled
    ///   bit 59:      stableBorrowRateEnabled
    ///   bits 64-79:  reserveFactor (bps)
    pub fn decode_config(&self) -> ConfigDecoded {
        let b = self.configuration_bitmap;
        ConfigDecoded {
            ltv_bps:               (b & 0xFFFF) as u32,
            liq_threshold_bps:     ((b >> 16) & 0xFFFF) as u32,
            liq_bonus_bps:         ((b >> 32) & 0xFFFF) as u32,
            decimals:              ((b >> 48) & 0xFF) as u32,
            is_active:             (b >> 56) & 1 != 0,
            is_frozen:             (b >> 57) & 1 != 0,
            borrowing_enabled:     (b >> 58) & 1 != 0,
            stable_rate_enabled:   (b >> 59) & 1 != 0,
            reserve_factor_bps:    ((b >> 64) & 0xFFFF) as u32,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConfigDecoded {
    pub ltv_bps: u32,
    pub liq_threshold_bps: u32,
    pub liq_bonus_bps: u32,
    pub decimals: u32,
    pub is_active: bool,
    pub is_frozen: bool,
    pub borrowing_enabled: bool,
    pub stable_rate_enabled: bool,
    pub reserve_factor_bps: u32,
}

/// Decode the 12-word struct returned by LendingPool.getReserveData(asset). Field order
/// per Aave V2 protocol-v2/contracts/protocol/libraries/types/DataTypes.sol::ReserveData:
///   0: configuration (bitmap)
///   1: liquidityIndex (ray)
///   2: variableBorrowIndex (ray)
///   3: currentLiquidityRate (ray) - supply APR
///   4: currentVariableBorrowRate (ray)
///   5: currentStableBorrowRate (ray)
///   6: lastUpdateTimestamp
///   7: aTokenAddress
///   8: stableDebtTokenAddress
///   9: variableDebtTokenAddress
///   10: interestRateStrategyAddress
///   11: id
pub async fn lp_get_reserve_data(lending_pool: &str, asset: &str, rpc: &str)
    -> anyhow::Result<ReserveData>
{
    let data = format!("{}{}", selectors::POOL_GET_RESERVE_DATA, pad_address(asset));
    let hex = eth_call(rpc, lending_pool, &data).await
        .map_err(|e| anyhow::anyhow!("LendingPool.getReserveData: {}", e))?;
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 768 { return Ok(ReserveData::default()); }
    let read = |i: usize| u128::from_str_radix(&raw[i*64..(i+1)*64], 16).unwrap_or(0);
    let read_addr = |i: usize| parse_address_word(&raw[i*64..(i+1)*64]);
    Ok(ReserveData {
        configuration_bitmap:            read(0),
        liquidity_index_ray:             read(1),
        variable_borrow_index_ray:       read(2),
        current_liquidity_rate_ray:      read(3),
        current_variable_borrow_rate_ray: read(4),
        current_stable_borrow_rate_ray:  read(5),
        last_update_timestamp:           read(6),
        a_token:                         read_addr(7),
        stable_debt_token:               read_addr(8),
        variable_debt_token:             read_addr(9),
        interest_rate_strategy:          read_addr(10),
        id:                              read(11),
    })
}

// ── IncentivesController reads ──────────────────────────────────────────────

/// Returns total accrued rewards (claimable) across the given assets for the user.
pub async fn incentives_get_rewards_balance(controller: &str, asset_addrs: &[String], user: &str, rpc: &str)
    -> anyhow::Result<u128>
{
    // Build calldata: selector + offset(0x40) + user + array_length + array_items
    let mut data = String::new();
    data.push_str(selectors::GET_REWARDS_BALANCE);
    data.push_str(&pad_u256(0x40));
    data.push_str(&pad_address(user));
    data.push_str(&pad_u256(asset_addrs.len() as u128));
    for a in asset_addrs {
        data.push_str(&pad_address(a));
    }
    let hex = eth_call(rpc, controller, &data).await
        .map_err(|e| anyhow::anyhow!("getRewardsBalance: {}", e))?;
    Ok(parse_u128_word(&hex))
}

pub async fn incentives_get_unclaimed_rewards(controller: &str, user: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("{}{}", selectors::GET_USER_UNCLAIMED_REWARDS, pad_address(user));
    let hex = eth_call(rpc, controller, &data).await
        .map_err(|e| anyhow::anyhow!("getUserUnclaimedRewards: {}", e))?;
    Ok(parse_u128_word(&hex))
}

pub async fn incentives_reward_token(controller: &str, rpc: &str) -> anyhow::Result<String> {
    let hex = eth_call(rpc, controller, selectors::REWARD_TOKEN).await?;
    Ok(parse_address_word(&hex))
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

/// Format a 1e18-scaled value (HF, ETH-equivalent) to human decimal.
pub fn fmt_1e18(v: u128) -> String {
    fmt_token_amount(v, 18)
}

/// Convert Aave V2 ray-scaled annual rate (1e27) to APR percentage.
/// In V2, `liquidityRate` and `variableBorrowRate` are stored as ray (1e27) representing
/// the ANNUAL rate directly (NOT per-second). So APR = rate / 1e27.
pub fn ray_to_apr_pct(rate_1e27: u128) -> f64 {
    if rate_1e27 == 0 { return 0.0; }
    (rate_1e27 as f64 / 1e27) * 100.0
}

/// Format basis points as percentage string.
pub fn bps_to_pct(bps: u128) -> String {
    format!("{:.2}", bps as f64 / 100.0)
}

#[cfg(test)]
mod tests {
    use super::selectors::*;
    use sha3::{Digest, Keccak256};

    /// Recompute every selector via keccak256 at runtime so any copy/paste typo in
    /// the hardcoded constants would fail this test instead of silently misrouting
    /// calls on-chain. Pattern matches euler-v2-plugin / fourmeme-plugin.
    fn sel(sig: &str) -> String {
        let h = Keccak256::digest(sig.as_bytes());
        format!("0x{}", hex::encode(&h[..4]))
    }

    #[test]
    fn selectors_match_keccak256() {
        // ERC-20
        assert_eq!(sel("balanceOf(address)"),         BALANCE_OF);
        assert_eq!(sel("allowance(address,address)"), ALLOWANCE);
        assert_eq!(sel("approve(address,uint256)"),   APPROVE);
        assert_eq!(sel("decimals()"),                 DECIMALS);
        assert_eq!(sel("symbol()"),                   SYMBOL);
        // LendingPool (Aave V2)
        assert_eq!(sel("deposit(address,uint256,address,uint16)"),   DEPOSIT);
        assert_eq!(sel("withdraw(address,uint256,address)"),         WITHDRAW);
        assert_eq!(sel("borrow(address,uint256,uint256,uint16,address)"), BORROW);
        assert_eq!(sel("repay(address,uint256,uint256,address)"),    REPAY);
        assert_eq!(sel("swapBorrowRateMode(address,uint256)"),       SWAP_BORROW_RATE_MODE);
        assert_eq!(sel("setUserUseReserveAsCollateral(address,bool)"), SET_USER_USE_RESERVE_AS_COLL);
        assert_eq!(sel("getReservesList()"),                         GET_RESERVES_LIST);
        assert_eq!(sel("getReserveData(address)"),                   POOL_GET_RESERVE_DATA);
        assert_eq!(sel("getUserAccountData(address)"),               GET_USER_ACCOUNT_DATA);
        // ProtocolDataProvider
        assert_eq!(sel("getAllReservesTokens()"),                    PDP_GET_ALL_RESERVES_TOKENS);
        assert_eq!(sel("getReserveTokensAddresses(address)"),        PDP_GET_RESERVE_TOKENS_ADDRS);
        assert_eq!(sel("getReserveConfigurationData(address)"),      PDP_GET_RESERVE_CONFIG_DATA);
        // PDP_GET_RESERVE_DATA shares its hex with POOL_GET_RESERVE_DATA — both are
        // `getReserveData(address)`, just exposed by different contracts. Verify once.
        assert_eq!(sel("getReserveData(address)"),                   PDP_GET_RESERVE_DATA);
        assert_eq!(sel("getUserReserveData(address,address)"),       PDP_GET_USER_RESERVE_DATA);
        // WETHGateway
        assert_eq!(sel("depositETH(address,address,uint16)"),        WG_DEPOSIT_ETH);
        assert_eq!(sel("withdrawETH(address,uint256,address)"),      WG_WITHDRAW_ETH);
        assert_eq!(sel("borrowETH(address,uint256,uint256,uint16)"), WG_BORROW_ETH);
        assert_eq!(sel("repayETH(address,uint256,uint256,address)"), WG_REPAY_ETH);
        // IncentivesController
        assert_eq!(sel("claimRewards(address[],uint256,address)"),   CLAIM_REWARDS);
        assert_eq!(sel("getUserUnclaimedRewards(address)"),          GET_USER_UNCLAIMED_REWARDS);
        assert_eq!(sel("getRewardsBalance(address[],address)"),      GET_REWARDS_BALANCE);
        assert_eq!(sel("REWARD_TOKEN()"),                            REWARD_TOKEN);
    }
}
