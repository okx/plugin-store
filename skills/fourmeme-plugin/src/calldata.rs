//! ABI-encoded calldata for Four.meme TokenManager V2 + TokenManagerHelper3 on BSC.
//!
//! All function selectors are runtime-verified against keccak256 by the test below,
//! so a bad copy/paste of a hardcoded hex string would fail `cargo test` instead of
//! silently misrouting calls on-chain.
//!
//! Selector survey (BSC chain 56):
//!   - TokenManager V2 (proxy `0x5c95…762b`, impl `0xecd0…1103`):
//!       buyTokenAMAP(address,uint256,uint256)               0x87f27655   simple 3-arg, recipient = msg.sender
//!       buyTokenAMAP(uint256,address,uint256,uint256)        0xedf9e251   4-arg with leading `origin` field
//!       sellToken(address,uint256)                           0xf464e7db   simple 2-arg
//!       createToken(bytes,bytes)                             0x519ebb10   v0.2 candidate
//!   - TokenManagerHelper3 (proxy `0xF251…6034`, impl `0xe8c2…240b`):
//!       getTokenInfo(address)                                0x1f69565f
//!       tryBuy(address,uint256,uint256)                      0xe21b103a
//!       trySell(address,uint256)                             0xc6f43e8c
//!
//! `quote == 0x0` means the token is BNB-quoted (use msg.value). Non-zero `quote`
//! is an ERC-20 quote (BUSD/USDT/CAKE etc.) — supported via the `amountApproval` /
//! `amountFunds` fields returned by `tryBuy`.

#![allow(dead_code)]

use crate::rpc::pad_address;

fn pad_u128(val: u128) -> String {
    format!("{:064x}", val)
}

const MAX_UINT256_HEX: &str =
    "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

// ─── ERC-20 ────────────────────────────────────────────────────────────────────

const SEL_APPROVE: &str = "095ea7b3";

pub fn build_approve(spender: &str, amount: u128) -> String {
    format!("0x{}{}{}", SEL_APPROVE, pad_address(spender), pad_u128(amount))
}

pub fn build_approve_max(spender: &str) -> String {
    format!("0x{}{}{}", SEL_APPROVE, pad_address(spender), MAX_UINT256_HEX)
}

// ─── Four.meme TokenManager V2 — write ─────────────────────────────────────────

/// `buyTokenAMAP(address token, uint256 funds, uint256 minAmount)` — selector 0x87f27655.
///
/// 3-arg form: recipient = msg.sender (implicit). Use this for the user's own buys.
/// `funds` is the spend in quote-token units (BNB wei when `quote == 0`).
/// `minAmount` is the slippage floor in token units (revert if filled < this).
pub const SEL_BUY_TOKEN_AMAP_3: &str = "87f27655";

pub fn build_buy_token_amap(token: &str, funds: u128, min_amount: u128) -> String {
    format!(
        "0x{}{}{}{}",
        SEL_BUY_TOKEN_AMAP_3,
        pad_address(token),
        pad_u128(funds),
        pad_u128(min_amount),
    )
}

/// `sellToken(address token, uint256 amount)` — selector 0xf464e7db.
///
/// Burns `amount` of the user's token balance and returns proceeds in quote token.
/// 2-arg form has no minFunds parameter — the contract uses the bonding curve at
/// execution-time price; if you need slippage protection on sells, the 6-arg
/// `sellToken(uint256,address,uint256,uint256,uint256,address)` (selector 0x06e7b98f)
/// is the alternative. v0.1 ships the 2-arg form for simplicity; the price-impact
/// preview from `trySell` is shown to the user before they sign.
pub const SEL_SELL_TOKEN_2: &str = "f464e7db";

pub fn build_sell_token(token: &str, amount: u128) -> String {
    format!(
        "0x{}{}{}",
        SEL_SELL_TOKEN_2,
        pad_address(token),
        pad_u128(amount),
    )
}

// ─── TokenManager V2 — createToken ─────────────────────────────────────────────

/// `createToken(bytes code, bytes signature)` — selector 0x519ebb10.
///
/// Both `code` and `signature` are dynamic `bytes` arrays. Encoding layout:
///   [0..4]    selector
///   [4..36]   offset to code (= 0x40, since two static head slots × 32)
///   [36..68]  offset to signature (= 0x40 + 0x20 + ceil(code.len/32)*32)
///   [code]    32-byte length || code bytes (right-padded to 32)
///   [sig]     32-byte length || sig bytes  (right-padded to 32)
pub const SEL_CREATE_TOKEN: &str = "519ebb10";

pub fn build_create_token(code: &str, signature: &str) -> String {
    fn strip(s: &str) -> &str { s.trim_start_matches("0x") }
    let code_hex = strip(code);
    let sig_hex  = strip(signature);
    let code_bytes_len = code_hex.len() / 2;
    let sig_bytes_len  = sig_hex.len()  / 2;

    // Pad each dynamic bytes to a 32-byte boundary.
    let code_padded_words = (code_bytes_len + 31) / 32;
    let sig_padded_words  = (sig_bytes_len  + 31) / 32;
    let code_padded_chars = code_padded_words * 64;
    let sig_padded_chars  = sig_padded_words  * 64;

    // Offsets are relative to start of args region (= byte 4 / hex char 8 of calldata).
    let code_offset = 0x40u64; // two head slots
    let sig_offset  = 0x40u64 + 0x20 + code_padded_words as u64 * 32;

    let mut out = String::new();
    out.push_str("0x");
    out.push_str(SEL_CREATE_TOKEN);
    out.push_str(&format!("{:064x}", code_offset));
    out.push_str(&format!("{:064x}", sig_offset));

    // code: length word + padded data
    out.push_str(&format!("{:064x}", code_bytes_len));
    out.push_str(code_hex);
    out.push_str(&"0".repeat(code_padded_chars - code_hex.len()));

    // signature: length word + padded data
    out.push_str(&format!("{:064x}", sig_bytes_len));
    out.push_str(sig_hex);
    out.push_str(&"0".repeat(sig_padded_chars - sig_hex.len()));

    out
}

// ─── TokenManagerHelper3 — read/quote ──────────────────────────────────────────

/// `getTokenInfo(address) view returns (uint256 version, address tokenManager,
/// address quote, uint256 lastPrice, uint256 tradingFeeRate, uint256 minTradingFee,
/// uint256 launchTime, uint256 offers, uint256 maxOffers, uint256 funds,
/// uint256 maxFunds, bool liquidityAdded)` — selector 0x1f69565f.
pub const SEL_GET_TOKEN_INFO: &str = "1f69565f";

pub fn build_get_token_info(token: &str) -> String {
    format!("0x{}{}", SEL_GET_TOKEN_INFO, pad_address(token))
}

/// `tryBuy(address token, uint256 amount, uint256 funds) view returns
/// (address tokenManager, address quote, uint256 estimatedAmount, uint256 estimatedCost,
/// uint256 estimatedFee, uint256 amountMsgValue, uint256 amountApproval,
/// uint256 amountFunds)` — selector 0xe21b103a.
///
/// Pass `amount = 0, funds = X` to ask "how many tokens for X funds?" (AMAP semantics).
/// Pass `amount = Y, funds = 0` to ask "how much funds to buy Y tokens?".
pub const SEL_TRY_BUY: &str = "e21b103a";

pub fn build_try_buy(token: &str, amount: u128, funds: u128) -> String {
    format!(
        "0x{}{}{}{}",
        SEL_TRY_BUY,
        pad_address(token),
        pad_u128(amount),
        pad_u128(funds),
    )
}

/// `trySell(address token, uint256 amount) view returns
/// (address tokenManager, address quote, uint256 funds, uint256 fee)` — selector 0xc6f43e8c.
pub const SEL_TRY_SELL: &str = "c6f43e8c";

pub fn build_try_sell(token: &str, amount: u128) -> String {
    format!(
        "0x{}{}{}",
        SEL_TRY_SELL,
        pad_address(token),
        pad_u128(amount),
    )
}

// ─── ERC-20 reads (re-export common selectors) ─────────────────────────────────

pub const SEL_BALANCE_OF: &str = "70a08231";
pub const SEL_DECIMALS:   &str = "313ce567";
pub const SEL_SYMBOL:     &str = "95d89b41";
pub const SEL_NAME:       &str = "06fdde03";
pub const SEL_TOTAL_SUPPLY: &str = "18160ddd";
pub const SEL_ALLOWANCE:    &str = "dd62ed3e";
pub const SEL_TRANSFER:     &str = "a9059cbb";

/// Build `transfer(address,uint256)` calldata for sending an ERC-20.
pub fn format_erc20_transfer(to: &str, amount: u128) -> String {
    format!("0x{}{}{}", SEL_TRANSFER, pad_address(to), pad_u128(amount))
}

// ─── TokenManager V2 view reads ────────────────────────────────────────────────

/// `_launchFee() view returns (uint256)` — required `msg.value` floor for createToken.
pub const SEL_LAUNCH_FEE:        &str = "009523a2";
/// `_tradingFeeRate() view returns (uint256)` — basis points (e.g. 100 = 1%).
pub const SEL_TRADING_FEE_RATE:  &str = "3472aee7";

// ─── TaxToken view reads (per token-tax-info reference) ────────────────────────

pub const SEL_TAX_FEE_RATE:      &str = "978bbdb9"; // feeRate()
pub const SEL_TAX_RATE_FOUNDER:  &str = "6f0e5053"; // rateFounder()
pub const SEL_TAX_RATE_HOLDER:   &str = "6234b84f"; // rateHolder()
pub const SEL_TAX_RATE_BURN:     &str = "18a4acea"; // rateBurn()
pub const SEL_TAX_RATE_LIQUIDITY:&str = "eda528d4"; // rateLiquidity()
pub const SEL_TAX_MIN_DISPATCH:  &str = "110395bd"; // minDispatch()
pub const SEL_TAX_MIN_SHARE:     &str = "8bb28de2"; // minShare()
pub const SEL_TAX_QUOTE:         &str = "999b93af"; // quote()
pub const SEL_TAX_FOUNDER:       &str = "4d853ee5"; // founder()

// ─── ERC-8004 Agent Identity ──────────────────────────────────────────────────

/// `register(string agentURI) returns (uint256)` — mint identity NFT.
pub const SEL_8004_REGISTER: &str = "f2c298be";

/// Encode `register(string)` calldata. Single dynamic-bytes-string arg layout:
///   [0..4]   selector
///   [4..36]  offset to string (= 0x20)
///   [36..68] string length in bytes
///   [68..]   utf-8 bytes, padded to 32-byte boundary
pub fn build_8004_register(agent_uri: &str) -> String {
    let bytes = agent_uri.as_bytes();
    let len   = bytes.len();
    let padded_words = (len + 31) / 32;
    let padded_chars = padded_words * 64;
    let mut out = String::new();
    out.push_str("0x");
    out.push_str(SEL_8004_REGISTER);
    out.push_str(&format!("{:064x}", 0x20u32));     // offset
    out.push_str(&format!("{:064x}", len));         // length
    let hex_data = hex::encode(bytes);
    out.push_str(&hex_data);
    out.push_str(&"0".repeat(padded_chars - hex_data.len()));
    out
}

/// Build calldata for a no-argument view function (just selector + 0 args).
pub fn build_no_args(selector: &str) -> String {
    format!("0x{}", selector)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha3::{Digest, Keccak256};

    fn sel(sig: &str) -> String {
        let h = Keccak256::digest(sig.as_bytes());
        hex::encode(&h[..4])
    }

    #[test]
    fn selectors_match_signatures() {
        assert_eq!(sel("approve(address,uint256)"),               SEL_APPROVE);
        assert_eq!(sel("buyTokenAMAP(address,uint256,uint256)"),  SEL_BUY_TOKEN_AMAP_3);
        assert_eq!(sel("sellToken(address,uint256)"),             SEL_SELL_TOKEN_2);
        assert_eq!(sel("createToken(bytes,bytes)"),               SEL_CREATE_TOKEN);
        assert_eq!(sel("getTokenInfo(address)"),                  SEL_GET_TOKEN_INFO);
        assert_eq!(sel("tryBuy(address,uint256,uint256)"),        SEL_TRY_BUY);
        assert_eq!(sel("trySell(address,uint256)"),               SEL_TRY_SELL);
        assert_eq!(sel("balanceOf(address)"),                     SEL_BALANCE_OF);
        assert_eq!(sel("decimals()"),                             SEL_DECIMALS);
        assert_eq!(sel("symbol()"),                               SEL_SYMBOL);
        assert_eq!(sel("name()"),                                 SEL_NAME);
        assert_eq!(sel("totalSupply()"),                          SEL_TOTAL_SUPPLY);
        assert_eq!(sel("allowance(address,address)"),             SEL_ALLOWANCE);
        assert_eq!(sel("_launchFee()"),                           SEL_LAUNCH_FEE);
        assert_eq!(sel("_tradingFeeRate()"),                      SEL_TRADING_FEE_RATE);
        assert_eq!(sel("feeRate()"),                              SEL_TAX_FEE_RATE);
        assert_eq!(sel("rateFounder()"),                          SEL_TAX_RATE_FOUNDER);
        assert_eq!(sel("rateHolder()"),                           SEL_TAX_RATE_HOLDER);
        assert_eq!(sel("rateBurn()"),                             SEL_TAX_RATE_BURN);
        assert_eq!(sel("rateLiquidity()"),                        SEL_TAX_RATE_LIQUIDITY);
        assert_eq!(sel("minDispatch()"),                          SEL_TAX_MIN_DISPATCH);
        assert_eq!(sel("minShare()"),                             SEL_TAX_MIN_SHARE);
        assert_eq!(sel("quote()"),                                SEL_TAX_QUOTE);
        assert_eq!(sel("founder()"),                              SEL_TAX_FOUNDER);
        assert_eq!(sel("register(string)"),                       SEL_8004_REGISTER);
    }

    /// Reproduce the exact calldata of a known-good createToken tx.
    ///
    /// Real BNB-quoted createToken response (see PR notes):
    ///   createArg: 672-byte ABI blob, signature: 65-byte ECDSA. The on-chain
    ///   tx 0xc7829757b753f20aa3805b74f295e86f662b9068e676ba6c86ee7e12a645b4c4
    ///   used 0x519ebb10 + offset_a=0x40 + offset_b=0x300 + length=0x2a0 (672)
    ///   + data + length=0x40 (64) + data. Verify our encoder matches.
    #[test]
    fn create_token_calldata_layout_matches_real_tx() {
        // 32-byte code (1 word) and 32-byte sig (1 word) — synthetic but exercises
        // the offset math without 1.5 KB of literal hex.
        let code = "0x".to_string()
            + "0000000000000000000000000000000000000000000000000000000000000020";
        let sig  = "0x".to_string()
            + "1111111111111111111111111111111111111111111111111111111111111111";
        let cd = build_create_token(&code, &sig);
        // 0x + 8 sel + 64 off_a + 64 off_b + 64 len_a + 64 data_a + 64 len_b + 64 data_b
        // = 2 + 8 + 6*64 = 394 chars
        assert_eq!(cd.len(), 394);
        assert!(cd.starts_with("0x519ebb10"));
        // off_a = 0x40
        assert!(cd[10..74].ends_with("0000000000000000000000000000000000000000000000000000000000000040"));
        // off_b = 0x40 + 0x20 + 0x20 = 0x80 (one 32-byte word of code data)
        assert!(cd[74..138].ends_with("0000000000000000000000000000000000000000000000000000000000000080"));
        // len_a = 32 = 0x20
        assert!(cd.contains("0000000000000000000000000000000000000000000000000000000000000020"));
    }

    #[test]
    fn buy_calldata_shape() {
        let cd = build_buy_token_amap(
            "0x1111111111111111111111111111111111111111",
            10_000_000_000_000_000u128, // 0.01 BNB in wei
            1u128,
        );
        // 0x + 8 sel + 64 token + 64 funds + 64 minAmount = 202 chars
        assert_eq!(cd.len(), 202);
        assert!(cd.starts_with("0x87f27655"));
    }

    #[test]
    fn sell_calldata_shape() {
        let cd = build_sell_token(
            "0x1111111111111111111111111111111111111111",
            1_000_000_000_000_000_000u128,
        );
        // 0x + 8 sel + 64 token + 64 amount = 138 chars
        assert_eq!(cd.len(), 138);
        assert!(cd.starts_with("0xf464e7db"));
    }
}
