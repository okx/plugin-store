//! SignalClient — onchainos CLI wrappers for signal data + swap execution on Solana.
//!
//! All network traffic goes through the onchainos CLI binary.
//! Swap flow: onchainos handles signing + broadcast internally.

use anyhow::{bail, Context, Result};
use serde_json::Value;

use super::engine::{safe_float, CHAIN_INDEX, SLIPPAGE_PCT, SOL_DECIMALS, SOL_NATIVE};
use super::engine::{MIN_LIQUIDITY, MIN_MCAP, MIN_WALLET_COUNT, SIGNAL_LABELS};
use crate::onchainos;

pub struct SignalClient {
    pub wallet: String,
}

pub struct SwapResult {
    pub tx_hash: Option<String>,
    pub amount_out: f64,
}

impl SignalClient {
    /// Create a fully authenticated client.
    /// Resolves wallet from onchainos agent wallet.
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

    // ── Signal API ────────────────────────────────────────────────

    /// Fetch smart money signals from OKX Signal API.
    pub async fn fetch_signals(&self) -> Result<Vec<Value>> {
        let data = onchainos::signal_list(
            "solana",
            Some(SIGNAL_LABELS),
            Some(&MIN_WALLET_COUNT.to_string()),
            Some(&format!("{MIN_MCAP:.0}")),
            Some(&format!("{MIN_LIQUIDITY:.0}")),
        )?;

        match data {
            Value::Array(arr) => Ok(arr),
            _ => Ok(data.as_array().cloned().unwrap_or_default()),
        }
    }

    // ── Market Data ───────────────────────────────────────────────

    /// Fetch price info (MC, Liq, Holders, Price, Top10).
    pub async fn fetch_price_info(&self, token_addr: &str) -> Result<Value> {
        let data = onchainos::token_price_info(token_addr, "solana")?;

        match data {
            Value::Array(arr) if !arr.is_empty() => Ok(arr[0].clone()),
            Value::Object(_) => Ok(data),
            _ => bail!("unexpected price-info response"),
        }
    }

    /// Fetch 1-minute candles for pump check.
    pub async fn fetch_candles_1m(&self, token_addr: &str) -> Result<Value> {
        onchainos::market_kline(token_addr, "solana", "1m", "5")
    }

    /// Fetch dev info from Trenches API.
    pub async fn fetch_dev_info(&self, token_addr: &str) -> Result<Value> {
        let data = onchainos::memepump_dev_info(token_addr, "solana")?;

        match data {
            Value::Array(arr) if !arr.is_empty() => Ok(arr[0].clone()),
            Value::Object(_) => Ok(data),
            _ => Ok(serde_json::json!({})),
        }
    }

    /// Fetch bundle info from Trenches API.
    pub async fn fetch_bundle_info(&self, token_addr: &str) -> Result<Value> {
        let data = onchainos::memepump_bundle_info(token_addr, "solana")?;

        match data {
            Value::Array(arr) if !arr.is_empty() => Ok(arr[0].clone()),
            Value::Object(_) => Ok(data),
            _ => Ok(serde_json::json!({})),
        }
    }

    /// Fetch current token price in USD.
    pub async fn fetch_price(&self, token_addr: &str) -> Result<f64> {
        let info = self.fetch_price_info(token_addr).await?;
        let price = safe_float(&info["price"], 0.0);
        if price <= 0.0 {
            bail!("invalid price for {token_addr}");
        }
        Ok(price)
    }

    /// Fetch SOL balance for the wallet.
    pub async fn fetch_sol_balance(&self) -> Result<f64> {
        if self.wallet.is_empty() {
            bail!("onchainos wallet not available — please login first");
        }
        let data = onchainos::portfolio_all_balances(&self.wallet, CHAIN_INDEX)?;

        // Response: [{"tokenAssets": [{"symbol":"SOL","balance":"1.09",...}, ...]}]
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

    /// Fetch quote (for honeypot detection).
    pub async fn fetch_quote(&self, token_addr: &str, amount_sol: f64) -> Result<Value> {
        let amount_raw = format!("{}", (amount_sol * 10f64.powi(SOL_DECIMALS as i32)) as u64);
        let data = onchainos::swap_quote(
            SOL_NATIVE,
            token_addr,
            &amount_raw,
            "solana",
            Some(SLIPPAGE_PCT),
        )?;

        match data {
            Value::Array(arr) if !arr.is_empty() => Ok(arr[0].clone()),
            _ => Ok(data),
        }
    }

    // ── Swap Execution ──────────────────────────────────────────────

    /// Execute a swap via onchainos on Solana.
    /// onchainos handles signing and broadcast internally.
    pub async fn execute_swap(
        &self,
        from_token: &str,
        to_token: &str,
        amount_raw: &str,
    ) -> Result<SwapResult> {
        if self.wallet.is_empty() {
            bail!("onchainos wallet not available — please login first");
        }

        let (tx_hash, swap_data) = onchainos::execute_solana_swap(
            from_token,
            to_token,
            amount_raw,
            &self.wallet,
            SLIPPAGE_PCT,
        )
        .await?;

        let amount_out = safe_float(&swap_data["routerResult"]["toTokenAmount"], 0.0);

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
    pub async fn buy_token(&self, token_addr: &str, sol_amount: f64) -> Result<SwapResult> {
        let amount_raw = format!("{}", (sol_amount * 10f64.powi(SOL_DECIMALS as i32)) as u64);
        self.execute_swap(SOL_NATIVE, token_addr, &amount_raw).await
    }

    /// Sell a token for SOL.
    pub async fn sell_token(&self, token_addr: &str, amount_raw: &str) -> Result<SwapResult> {
        self.execute_swap(token_addr, SOL_NATIVE, amount_raw).await
    }
}
