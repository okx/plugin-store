pub mod quickstart;
pub mod login;
pub mod get_token;
pub mod quote_buy;
pub mod quote_sell;
pub mod positions;
pub mod buy;
pub mod sell;
pub mod create_token;
pub mod config;
pub mod list_tokens;
pub mod tax_info;
pub mod agent_balance;
pub mod send;
pub mod agent_register;
pub mod events;

/// Build a structured error JSON for stdout (per GEN-001).
pub fn error_response(
    err: &anyhow::Error,
    context: Option<&str>,
    extra_hint: Option<&str>,
) -> String {
    let msg = format!("{:#}", err);
    let (error_code, mut suggestion) = classify_error(&msg, context);
    if let Some(h) = extra_hint {
        let h = h.trim();
        if !h.is_empty() {
            suggestion.push(' ');
            suggestion.push_str(h);
        }
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "ok": false,
        "error": msg,
        "error_code": error_code,
        "suggestion": suggestion,
    }))
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":{:?}}}"#, msg))
}

fn classify_error(msg: &str, ctx: Option<&str>) -> (&'static str, String) {
    let m = msg.to_lowercase();

    if m.contains("rpc request failed")
        || m.contains("error sending request")
        || m.contains("connection refused")
        || m.contains("certificate")
    {
        return (
            "NETWORK_UNREACHABLE",
            "Network request failed. Check internet connectivity and that bsc-rpc.publicnode.com is reachable.".into(),
        );
    }

    if m.contains("could not determine wallet address") || m.contains("wallet addresses") {
        return (
            "NO_WALLET",
            "No active onchainos wallet. Run `onchainos wallet status` to inspect, or `onchainos wallet add` to create one.".into(),
        );
    }

    if m.contains("chain") && m.contains("not supported") {
        return (
            "CHAIN_NOT_SUPPORTED",
            "fourmeme-plugin v0.1 supports BNB Chain only (chain 56). Pass `--chain 56` or omit the flag.".into(),
        );
    }

    if m.contains("not bnb-quoted") || m.contains("non-native quote") {
        return (
            "QUOTE_TOKEN_UNSUPPORTED",
            "This token is paired against an ERC-20 quote (BUSD/USDT/CAKE), not BNB. Support is planned for v0.2.".into(),
        );
    }

    if m.contains("insufficient") && m.contains("bnb") {
        return (
            "INSUFFICIENT_BNB",
            "Wallet doesn't have enough BNB for the trade size + gas. Top up BNB on BSC and retry.".into(),
        );
    }

    if m.contains("liquidity added") || m.contains("graduated") {
        return (
            "TOKEN_GRADUATED",
            "Token has already migrated to PancakeSwap — trade it through pancakeswap-v3-plugin instead of fourmeme-plugin.".into(),
        );
    }

    if m.contains("not confirmed within") || m.contains("execution reverted") || m.contains("reverted on-chain") {
        return (
            "TX_FAILED",
            "Transaction did not confirm successfully. Check BSCScan for the tx hash. \
             Common causes: slippage too tight, token graduated mid-tx, insufficient BNB.".into(),
        );
    }

    if m.contains("backend rejected") || m.contains("meme-web-access") || m.contains("auth-token") {
        return (
            "FOURMEME_AUTH_REQUIRED",
            "create-token requires a valid `meme-web-access` cookie. Log into four.meme in a browser, copy the cookie via DevTools, and pass it via --auth-token. Cookies are wallet-bound and expire ~30 days after issue.".into(),
        );
    }

    if m.contains("image upload") || m.contains("upload-image") || m.contains("upload image") {
        return (
            "IMAGE_UPLOAD_FAILED",
            "Image upload to four.meme CDN failed. Check the file exists and is a valid PNG/JPG/GIF/WEBP, or pre-upload via the web UI and pass --image-url instead.".into(),
        );
    }

    let default_code: &'static str = match ctx {
        Some("quickstart")   => "QUICKSTART_FAILED",
        Some("get-token")    => "GET_TOKEN_FAILED",
        Some("quote-buy")    => "QUOTE_BUY_FAILED",
        Some("quote-sell")   => "QUOTE_SELL_FAILED",
        Some("positions")    => "POSITIONS_FAILED",
        Some("buy")          => "BUY_FAILED",
        Some("sell")         => "SELL_FAILED",
        Some("create-token") => "CREATE_TOKEN_FAILED",
        Some("login")        => "LOGIN_FAILED",
        Some("config")       => "CONFIG_FAILED",
        Some("list-tokens")  => "LIST_TOKENS_FAILED",
        Some("tax-info")     => "TAX_INFO_FAILED",
        Some("agent-balance")=> "AGENT_BALANCE_FAILED",
        Some("send")         => "SEND_FAILED",
        Some("agent-register") => "AGENT_REGISTER_FAILED",
        Some("events")       => "EVENTS_FAILED",
        _                    => "UNKNOWN_ERROR",
    };
    (default_code, "See error field for details.".into())
}

// ─── Shared helpers used by multiple commands ──────────────────────────────────

use anyhow::Result;
use serde_json::Value;

/// Decoded `getTokenInfo(address)` result.
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub version:           u128,
    pub token_manager:     String,
    pub quote:             String,
    pub last_price:        u128,
    pub trading_fee_rate:  u128,
    pub min_trading_fee:   u128,
    pub launch_time:       u128,
    pub offers:            u128,
    pub max_offers:        u128,
    pub funds:             u128,
    pub max_funds:         u128,
    pub liquidity_added:   bool,
}

impl TokenInfo {
    pub fn is_bnb_quoted(&self) -> bool {
        self.quote.eq_ignore_ascii_case(crate::config::NATIVE_QUOTE)
    }

    /// Bonding-curve progress in percent based on tokens sold (`offers / maxOffers`).
    /// Note: graduation also requires hitting the `funds` target — display both metrics.
    pub fn progress_by_offers_pct(&self) -> f64 {
        if self.max_offers == 0 { return 0.0; }
        (self.offers as f64 / self.max_offers as f64) * 100.0
    }

    pub fn progress_by_funds_pct(&self) -> f64 {
        if self.max_funds == 0 { return 0.0; }
        (self.funds as f64 / self.max_funds as f64) * 100.0
    }
}

pub async fn fetch_token_info(chain_id: u64, token: &str) -> Result<TokenInfo> {
    let data = crate::calldata::build_get_token_info(token);
    let hex = crate::rpc::eth_call(chain_id, crate::config::addresses::TOKEN_MANAGER_HELPER3, &data).await?;
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 12 * 64 {
        anyhow::bail!(
            "getTokenInfo returned {} hex chars (expected 768). Token {} may not be a Four.meme token.",
            raw.len(), token
        );
    }
    let word = |i: usize| &raw[i * 64..(i + 1) * 64];
    let to_u128 = |w: &str| crate::rpc::parse_uint256_to_u128(&format!("0x{}", w));
    Ok(TokenInfo {
        version:          to_u128(word(0)),
        token_manager:    crate::rpc::parse_address(word(1)),
        quote:            crate::rpc::parse_address(word(2)),
        last_price:       to_u128(word(3)),
        trading_fee_rate: to_u128(word(4)),
        min_trading_fee:  to_u128(word(5)),
        launch_time:      to_u128(word(6)),
        offers:           to_u128(word(7)),
        max_offers:       to_u128(word(8)),
        funds:            to_u128(word(9)),
        max_funds:        to_u128(word(10)),
        liquidity_added:  to_u128(word(11)) != 0,
    })
}

#[derive(Debug, Clone)]
pub struct TryBuyQuote {
    pub token_manager:     String,
    pub quote:             String,
    pub estimated_amount:  u128,
    pub estimated_cost:    u128,
    pub estimated_fee:     u128,
    pub amount_msg_value:  u128,
    pub amount_approval:   u128,
    pub amount_funds:      u128,
}

pub async fn fetch_try_buy(chain_id: u64, token: &str, amount: u128, funds: u128) -> Result<TryBuyQuote> {
    let data = crate::calldata::build_try_buy(token, amount, funds);
    let hex = crate::rpc::eth_call(chain_id, crate::config::addresses::TOKEN_MANAGER_HELPER3, &data).await?;
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 8 * 64 {
        anyhow::bail!("tryBuy returned {} hex chars (expected 512)", raw.len());
    }
    let word = |i: usize| &raw[i * 64..(i + 1) * 64];
    let to_u128 = |w: &str| crate::rpc::parse_uint256_to_u128(&format!("0x{}", w));
    Ok(TryBuyQuote {
        token_manager:    crate::rpc::parse_address(word(0)),
        quote:            crate::rpc::parse_address(word(1)),
        estimated_amount: to_u128(word(2)),
        estimated_cost:   to_u128(word(3)),
        estimated_fee:    to_u128(word(4)),
        amount_msg_value: to_u128(word(5)),
        amount_approval:  to_u128(word(6)),
        amount_funds:     to_u128(word(7)),
    })
}

#[derive(Debug, Clone)]
pub struct TrySellQuote {
    pub token_manager: String,
    pub quote:         String,
    pub funds:         u128,
    pub fee:           u128,
}

pub async fn fetch_try_sell(chain_id: u64, token: &str, amount: u128) -> Result<TrySellQuote> {
    let data = crate::calldata::build_try_sell(token, amount);
    let hex = crate::rpc::eth_call(chain_id, crate::config::addresses::TOKEN_MANAGER_HELPER3, &data).await?;
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 4 * 64 {
        anyhow::bail!("trySell returned {} hex chars (expected 256)", raw.len());
    }
    let word = |i: usize| &raw[i * 64..(i + 1) * 64];
    let to_u128 = |w: &str| crate::rpc::parse_uint256_to_u128(&format!("0x{}", w));
    Ok(TrySellQuote {
        token_manager: crate::rpc::parse_address(word(0)),
        quote:         crate::rpc::parse_address(word(1)),
        funds:         to_u128(word(2)),
        fee:           to_u128(word(3)),
    })
}

/// Read ERC-20 balance.
pub async fn erc20_balance(chain_id: u64, token: &str, owner: &str) -> Result<u128> {
    let data = crate::rpc::build_address_call(crate::calldata::SEL_BALANCE_OF, owner);
    let hex = crate::rpc::eth_call(chain_id, token, &data).await?;
    Ok(crate::rpc::parse_uint256_to_u128(&hex))
}

/// Read ERC-20 symbol (best-effort, returns `?` on failure).
pub async fn erc20_symbol(chain_id: u64, token: &str) -> String {
    let data = format!("0x{}", crate::calldata::SEL_SYMBOL);
    match crate::rpc::eth_call(chain_id, token, &data).await {
        Ok(hex) => decode_string_return(&hex).unwrap_or_else(|| "?".to_string()),
        Err(_) => "?".to_string(),
    }
}

/// Decode an ABI-encoded `string` return value (offset+length+data).
fn decode_string_return(hex: &str) -> Option<String> {
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 128 { return None; }
    // [0..64] = offset (always 0x20 for a single string)
    // [64..128] = length in bytes
    let length = u128::from_str_radix(&raw[64..128], 16).ok()? as usize;
    let data_hex = raw.get(128..128 + length * 2)?;
    let bytes = hex::decode(data_hex).ok()?;
    String::from_utf8(bytes).ok()
}

/// Convert raw u128 token amount → human-readable decimal string with 6 places.
pub fn fmt_decimal(raw: u128, decimals: u32) -> String {
    let f = raw as f64 / 10f64.powi(decimals as i32);
    format!("{:.6}", f)
}

/// Parse a human "0.01" amount → raw u128 with 18-decimal scaling (Four.meme token + BNB).
pub fn parse_human_amount(s: &str, decimals: u32) -> Result<u128> {
    let f: f64 = s.parse()
        .map_err(|_| anyhow::anyhow!("Invalid amount '{}'", s))?;
    if f <= 0.0 {
        anyhow::bail!("amount must be positive");
    }
    Ok((f * 10f64.powi(decimals as i32)).round() as u128)
}

/// Apply slippage_bps (basis points) to a raw amount, returning amount × (1 - slippage).
pub fn apply_slippage_floor(raw: u128, slippage_bps: u64) -> u128 {
    let bps = slippage_bps.min(10_000);
    let mult = 10_000u128.saturating_sub(bps as u128);
    raw.saturating_mul(mult) / 10_000
}

#[allow(dead_code)]
pub fn _force_value_use(_: Value) {}
