//! SniperClient — onchainos CLI wrappers for token data + swap execution on Solana.
//!
//! All network traffic goes through the onchainos CLI binary.
//! Swap flow: onchainos handles signing + broadcast internally.

use anyhow::{bail, Context, Result};
use serde_json::Value;

use super::engine::{safe_float, CHAIN_INDEX, SLIPPAGE_PCT, SOL_DECIMALS, SOL_NATIVE};
use plugin_store_core::onchainos;

pub struct SniperClient {
    pub wallet: String,
}

pub struct SwapResult {
    pub tx_hash: Option<String>,
    pub amount_out: f64,
    pub raw_response: Value,
}

impl SniperClient {
    /// Create a fully authenticated client.
    /// Resolves wallet from onchainos agent wallet.
    pub fn new() -> Result<Self> {
        let wallet = onchainos::get_sol_address()
            .context("onchainos wallet not available — please login first")?;
        Ok(Self { wallet })
    }

    /// Create client for read-only operations (no wallet needed for data queries).
    pub fn new_read_only() -> Result<Self> {
        let wallet = onchainos::get_sol_address().unwrap_or_default();
        Ok(Self { wallet })
    }

    // ── Data queries ────────────────────────────────────────────────

    /// Fetch Solana top tokens by 24h price change (trending).
    pub async fn fetch_ranking(&self, top_n: usize) -> Result<Vec<Value>> {
        let data = onchainos::token_trending(CHAIN_INDEX, "2", "1")?;

        let tokens = match data {
            Value::Array(arr) => arr,
            _ => data.as_array().cloned().unwrap_or_default(),
        };

        Ok(tokens.into_iter().take(top_n).collect())
    }

    /// Fetch advanced token info for safety checks.
    pub async fn fetch_advanced_info(&self, token_addr: &str) -> Result<Value> {
        let data = onchainos::token_advanced_info(token_addr, "solana")?;

        match data {
            Value::Array(arr) if !arr.is_empty() => Ok(arr[0].clone()),
            Value::Object(_) => Ok(data),
            _ => bail!("unexpected advanced-info response format"),
        }
    }

    /// Fetch current token price in USD.
    pub async fn fetch_price(&self, token_addr: &str) -> Result<f64> {
        let data = onchainos::token_price_info(token_addr, "solana")?;

        let item = match &data {
            Value::Array(arr) if !arr.is_empty() => &arr[0],
            _ => &data,
        };

        let price = safe_float(&item["price"], 0.0);
        if price <= 0.0 {
            bail!("invalid price for {token_addr}");
        }
        Ok(price)
    }

    /// Fetch holder data filtered by tag (6=Suspicious, 8=Phishing).
    pub async fn fetch_holder_risk(
        &self,
        token_addr: &str,
        tag_filter: &str,
    ) -> Result<Vec<Value>> {
        let data = onchainos::token_holders(token_addr, "solana", Some(tag_filter))?;

        match data {
            Value::Array(arr) => Ok(arr),
            _ => Ok(data.as_array().cloned().unwrap_or_default()),
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
            raw_response: swap_data,
        })
    }

    /// Buy a token with SOL.
    pub async fn buy_token(&self, token_addr: &str, sol_amount: f64) -> Result<SwapResult> {
        let amount_raw = format!("{}", (sol_amount * 10f64.powi(SOL_DECIMALS as i32)) as u64);
        self.execute_swap(SOL_NATIVE, token_addr, &amount_raw).await
    }

    /// Sell a token for SOL. `amount_raw` is the raw token amount.
    pub async fn sell_token(&self, token_addr: &str, amount_raw: &str) -> Result<SwapResult> {
        self.execute_swap(token_addr, SOL_NATIVE, amount_raw).await
    }
}
