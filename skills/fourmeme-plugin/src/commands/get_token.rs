/// `fourmeme-plugin get-token --address 0x...` — full on-chain state for a token.

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain, TOKEN_DECIMALS};

#[derive(Args)]
pub struct GetTokenArgs {
    #[arg(long)]
    pub address: String,

    #[arg(long, default_value_t = 56)]
    pub chain: u64,
}

pub async fn run(args: GetTokenArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("get-token"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: GetTokenArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    let token = args.address.to_lowercase();

    let info = super::fetch_token_info(args.chain, &token).await?;
    let sym = super::erc20_symbol(args.chain, &token).await;

    // Average BNB-per-whole-token from the curve so far. Defined as BNB-funds-raised /
    // tokens-sold; reliable. The raw `lastPrice` field uses an internal scaling we
    // haven't fully reversed — surface it for debugging but don't humanize it.
    let avg_price_bnb_per_token = if info.offers > 0 {
        (info.funds as f64 / 1e18) / (info.offers as f64 / 10f64.powi(TOKEN_DECIMALS as i32))
    } else { 0.0 };

    let resp = serde_json::json!({
        "ok": true,
        "data": {
            "chain": chain_name(args.chain),
            "chain_id": args.chain,
            "token": token,
            "symbol": sym,
            "version": info.version.to_string(),
            "token_manager": info.token_manager,
            "quote": info.quote,
            "is_bnb_quoted": info.is_bnb_quoted(),
            "avg_price_bnb_per_token": format!("{:.18}", avg_price_bnb_per_token),
            "last_price_raw": info.last_price.to_string(),
            "trading_fee_rate_bps": info.trading_fee_rate.to_string(),
            "min_trading_fee_raw":  info.min_trading_fee.to_string(),
            "launch_time_unix": info.launch_time.to_string(),
            "offers":         super::fmt_decimal(info.offers, TOKEN_DECIMALS),
            "offers_raw":     info.offers.to_string(),
            "max_offers":     super::fmt_decimal(info.max_offers, TOKEN_DECIMALS),
            "max_offers_raw": info.max_offers.to_string(),
            "funds_bnb":      format!("{:.6}", info.funds as f64 / 1e18),
            "funds_raw":      info.funds.to_string(),
            "max_funds_bnb":  format!("{:.6}", info.max_funds as f64 / 1e18),
            "max_funds_raw":  info.max_funds.to_string(),
            "progress_by_offers_pct": format!("{:.2}", info.progress_by_offers_pct()),
            "progress_by_funds_pct":  format!("{:.2}", info.progress_by_funds_pct()),
            "graduated": info.liquidity_added,
            "tip": if info.liquidity_added {
                "Token has graduated to PancakeSwap — trade it via the pancakeswap-v3 plugin."
            } else if !info.is_bnb_quoted() {
                "Token has a non-BNB quote (BUSD/USDT/CAKE). v0.1 only supports BNB-quoted tokens."
            } else {
                "Pre-graduate, BNB-quoted. Use `quote-buy --token <addr> --funds 0.005` for a live preview."
            },
        }
    });
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
