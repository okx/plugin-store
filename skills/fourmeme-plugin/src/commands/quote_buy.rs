/// `fourmeme-plugin quote-buy --token 0x... --funds 0.01` — preview a buy.
///
/// Wraps `TokenManagerHelper3.tryBuy(token, 0, fundsWei)` for AMAP semantics
/// (spend the requested BNB, get whatever token amount the curve fills).

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain, TOKEN_DECIMALS};

#[derive(Args)]
pub struct QuoteBuyArgs {
    #[arg(long)]
    pub token: String,

    /// Amount of BNB to spend (e.g. "0.01" — quoted in whole BNB)
    #[arg(long)]
    pub funds: String,

    #[arg(long, default_value_t = 56)]
    pub chain: u64,
}

pub async fn run(args: QuoteBuyArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("quote-buy"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: QuoteBuyArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    let token = args.token.to_lowercase();
    let funds_wei = super::parse_human_amount(&args.funds, 18)?; // BNB has 18 dec

    let info = super::fetch_token_info(args.chain, &token).await?;
    if info.liquidity_added {
        anyhow::bail!(
            "Token has graduated to PancakeSwap (liquidityAdded = true). Use the pancakeswap plugin instead."
        );
    }
    if !info.is_bnb_quoted() {
        anyhow::bail!(
            "Token uses a non-native quote ({}). v0.1 only supports BNB-quoted tokens.",
            info.quote
        );
    }
    let sym = super::erc20_symbol(args.chain, &token).await;

    let q = super::fetch_try_buy(args.chain, &token, 0, funds_wei).await?;

    let estimated_amount_h = super::fmt_decimal(q.estimated_amount, TOKEN_DECIMALS);
    let estimated_cost_bnb = q.estimated_cost as f64 / 1e18;
    let fee_bnb = q.estimated_fee as f64 / 1e18;
    let price_per_token_bnb = if q.estimated_amount > 0 {
        (q.estimated_cost as f64 + q.estimated_fee as f64) / 1e18
            / (q.estimated_amount as f64 / 1e18)
    } else { 0.0 };

    let resp = serde_json::json!({
        "ok": true,
        "data": {
            "action": "quote-buy (AMAP)",
            "chain": chain_name(args.chain),
            "chain_id": args.chain,
            "token": token,
            "symbol": sym,
            "token_manager": q.token_manager,
            "quote": q.quote,
            "is_bnb_quoted": true,
            "input": {
                "funds_bnb": args.funds,
                "funds_wei": funds_wei.to_string(),
            },
            "output": {
                "estimated_amount":     estimated_amount_h,
                "estimated_amount_raw": q.estimated_amount.to_string(),
                "estimated_cost_bnb":   format!("{:.8}", estimated_cost_bnb),
                "estimated_cost_wei":   q.estimated_cost.to_string(),
                "estimated_fee_bnb":    format!("{:.8}", fee_bnb),
                "estimated_fee_wei":    q.estimated_fee.to_string(),
                "amount_msg_value_wei": q.amount_msg_value.to_string(),
                "amount_approval_raw":  q.amount_approval.to_string(),
                "amount_funds_raw":     q.amount_funds.to_string(),
                "effective_price_bnb_per_token": format!("{:.18}", price_per_token_bnb),
            },
            "tip": format!(
                "To execute: `fourmeme-plugin buy --token {} --funds {}` (default 1% slippage). \
                 Pass `--slippage-bps 200` for 2% if the curve is thin.",
                token, args.funds
            ),
        }
    });
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
