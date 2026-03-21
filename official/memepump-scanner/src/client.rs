//! ScannerClient — onchainos CLI wrapper for memepump scanner on Solana.
//!
//! Uses `crate::onchainos` for all operations. Wallet address comes from
//! `onchainos::get_sol_address()`.
//! No local Solana signing — onchainos handles auth / TEE signing / broadcast.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use super::engine::{safe_float, CHAIN_INDEX, SOL_DECIMALS, SOL_NATIVE};
use crate::onchainos;

pub struct ScannerClient {
    pub wallet: String,
}

pub struct SwapResult {
    pub tx_hash: Option<String>,
    pub amount_out: f64,
}

/// Map from JSON param key → onchainos CLI flag name.
fn param_key_to_flag(key: &str) -> Option<&'static str> {
    match key {
        "chainIndex" => None, // handled by --chain
        "stage" => None,      // handled by --stage
        "minMarketCapUsd" => Some("--min-market-cap"),
        "maxMarketCapUsd" => Some("--max-market-cap"),
        "minHolders" => Some("--min-holders"),
        "maxTop10HoldingsPercent" => Some("--max-top10-holdings-percent"),
        "maxDevHoldingsPercent" => Some("--max-dev-holdings-percent"),
        "maxInsidersPercent" => Some("--max-insiders-percent"),
        "maxBundlersPercent" => Some("--max-bundlers-percent"),
        "minTokenAge" => Some("--min-token-age"),
        "maxTokenAge" => Some("--max-token-age"),
        "minBuyTxCount" => Some("--min-buy-tx-count"),
        "minVolume" => Some("--min-volume"),
        "maxSnipersPercent" => Some("--max-snipers-percent"),
        "maxFreshWalletPercent" => Some("--max-fresh-wallets-percent"),
        "minTxCount" => Some("--min-tx-count"),
        _ => None,
    }
}

impl ScannerClient {
    /// Create a fully authenticated client.
    /// Wallet resolved from onchainos agent wallet.
    pub fn new() -> Result<Self> {
        let wallet = onchainos::get_sol_address()
            .context("onchainos wallet not available — please login first")?;
        Ok(Self { wallet })
    }

    /// Read-only client (no wallet needed for data queries).
    pub fn new_read_only() -> Result<Self> {
        let wallet = onchainos::get_sol_address().unwrap_or_default();
        Ok(Self { wallet })
    }

    // ── Trenches API ────────────────────────────────────────────────

    /// Fetch memepump token list with server-side filters.
    /// `params` is a JSON object whose keys are translated to onchainos CLI flags.
    pub async fn get_memepump_list(&self, params: &Value) -> Result<Vec<Value>> {
        // Extract chain and stage from params (or use defaults)
        let chain = params["chainIndex"]
            .as_str()
            .unwrap_or(CHAIN_INDEX);
        let stage = params["stage"]
            .as_str()
            .unwrap_or("1");

        // Build CLI filter pairs from the remaining JSON keys
        let mut filter_strings: Vec<(String, String)> = Vec::new();

        if let Some(map) = params.as_object() {
            for (key, val) in map {
                if let Some(flag) = param_key_to_flag(key) {
                    let s = val
                        .as_str()
                        .map(|s| s.to_string())
                        .or_else(|| {
                            if val.is_number() {
                                Some(val.to_string())
                            } else {
                                None
                            }
                        });
                    if let Some(s) = s {
                        if !s.is_empty() {
                            filter_strings.push((flag.to_string(), s));
                        }
                    }
                }
            }
        }

        let filters: Vec<(&str, &str)> = filter_strings
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let data = onchainos::memepump_tokens(chain, stage, &filters)?;

        match data {
            Value::Array(arr) => Ok(arr),
            _ => Ok(data.as_array().cloned().unwrap_or_default()),
        }
    }

    /// Fetch dev info for a token (rug history, total launched, holdings).
    pub async fn get_dev_info(&self, token_addr: &str) -> Result<Value> {
        let data = onchainos::memepump_dev_info(token_addr, CHAIN_INDEX)?;

        match data {
            Value::Array(arr) if !arr.is_empty() => Ok(arr[0].clone()),
            Value::Object(_) => Ok(data),
            _ => Ok(json!({})),
        }
    }

    /// Fetch bundle info for a token (bundler ATH %, bundler count).
    pub async fn get_bundle_info(&self, token_addr: &str) -> Result<Value> {
        let data = onchainos::memepump_bundle_info(token_addr, CHAIN_INDEX)?;

        match data {
            Value::Array(arr) if !arr.is_empty() => Ok(arr[0].clone()),
            Value::Object(_) => Ok(data),
            _ => Ok(json!({})),
        }
    }

    /// Fetch 1-minute candles for volume analysis.
    pub async fn get_candles(&self, token_addr: &str, limit: u32) -> Result<Value> {
        let limit_str = limit.to_string();
        onchainos::market_kline(token_addr, CHAIN_INDEX, "1m", &limit_str)
    }

    /// Fetch price info (MC, holders, price, top10, etc.).
    pub async fn get_price_info(&self, token_addr: &str) -> Result<Value> {
        let data = onchainos::token_price_info(token_addr, CHAIN_INDEX)?;

        match data {
            Value::Array(arr) if !arr.is_empty() => Ok(arr[0].clone()),
            Value::Object(_) => Ok(data),
            _ => bail!("unexpected price-info response"),
        }
    }

    /// Fetch SOL balance for the wallet.
    pub async fn fetch_sol_balance(&self) -> Result<f64> {
        if self.wallet.is_empty() {
            bail!("onchainos wallet not available — please login first");
        }

        let data = onchainos::portfolio_all_balances(&self.wallet, CHAIN_INDEX)?;

        let assets = if let Some(arr) = data.as_array() {
            arr.first()
                .and_then(|item| item["tokenAssets"].as_array())
                .cloned()
                .unwrap_or_default()
        } else {
            data["tokenAssets"].as_array().cloned().unwrap_or_default()
        };

        for b in &assets {
            let sym = b["symbol"].as_str().unwrap_or("");
            let contract = b["tokenContractAddress"].as_str().unwrap_or("");
            if sym == "SOL" || contract == SOL_NATIVE {
                return Ok(safe_float(&b["balance"], 0.0));
            }
        }
        Ok(0.0)
    }

    // ── Swap execution ──────────────────────────────────────────────

    /// Execute a swap via onchainos CLI (sign + broadcast handled by onchainos).
    /// `from_token` and `to_token` are Solana token addresses.
    /// `amount_raw` is in minimal units (lamports for SOL, raw for SPL).
    pub async fn execute_swap(
        &self,
        from_token: &str,
        to_token: &str,
        amount_raw: &str,
        slippage_pct: &str,
    ) -> Result<SwapResult> {
        if self.wallet.is_empty() {
            bail!("onchainos wallet not available — please login first");
        }

        let (tx_hash, swap_data) = onchainos::execute_solana_swap(
            from_token,
            to_token,
            amount_raw,
            &self.wallet,
            slippage_pct,
        )
        .await?;

        let amount_out = safe_float(
            &swap_data["routerResult"]["toTokenAmount"],
            safe_float(&swap_data["toTokenAmount"], 0.0),
        );

        Ok(SwapResult {
            tx_hash: if tx_hash.is_empty() {
                None
            } else {
                Some(tx_hash)
            },
            amount_out,
        })
    }

    /// Buy a token with SOL.
    pub async fn buy_token(
        &self,
        token_addr: &str,
        sol_amount: f64,
        slippage_pct: u32,
    ) -> Result<SwapResult> {
        let amount_raw = format!("{}", (sol_amount * 10f64.powi(SOL_DECIMALS as i32)) as u64);
        let slippage_str = slippage_pct.to_string();
        self.execute_swap(SOL_NATIVE, token_addr, &amount_raw, &slippage_str)
            .await
    }

    /// Sell a token for SOL. `amount_raw` is the raw token amount.
    pub async fn sell_token(
        &self,
        token_addr: &str,
        amount_raw: &str,
        slippage_pct: u32,
    ) -> Result<SwapResult> {
        let slippage_str = slippage_pct.to_string();
        self.execute_swap(token_addr, SOL_NATIVE, amount_raw, &slippage_str)
            .await
    }
}
