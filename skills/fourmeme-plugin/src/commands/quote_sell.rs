/// `fourmeme-plugin quote-sell --token 0x... --amount <tokens>` — preview a sell.

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain, TOKEN_DECIMALS};

#[derive(Args)]
pub struct QuoteSellArgs {
    #[arg(long)]
    pub token: String,

    /// Amount of token to sell (e.g. "1000000" — whole tokens; auto-multiplied by 1e18)
    #[arg(long)]
    pub amount: Option<String>,

    /// Sell entire wallet balance
    #[arg(long)]
    pub all: bool,

    #[arg(long, default_value_t = 56)]
    pub chain: u64,
}

pub async fn run(args: QuoteSellArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("quote-sell"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: QuoteSellArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    if args.all && args.amount.is_some() {
        anyhow::bail!("Pass either --amount or --all, not both.");
    }
    let token = args.token.to_lowercase();

    let info = super::fetch_token_info(args.chain, &token).await?;
    if info.liquidity_added {
        anyhow::bail!(
            "Token has graduated to PancakeSwap. Sell via pancakeswap plugin, not fourmeme."
        );
    }

    let amount_raw = if args.all {
        let wallet = crate::onchainos::get_wallet_address(args.chain).await?;
        let bal = super::erc20_balance(args.chain, &token, &wallet).await?;
        if bal == 0 {
            anyhow::bail!("Wallet has no balance of {} — nothing to sell.", token);
        }
        bal
    } else {
        let s = args.amount.as_ref()
            .ok_or_else(|| anyhow::anyhow!("--amount or --all is required"))?;
        super::parse_human_amount(s, TOKEN_DECIMALS)?
    };

    let sym = super::erc20_symbol(args.chain, &token).await;
    let q = super::fetch_try_sell(args.chain, &token, amount_raw).await?;
    let funds_bnb = q.funds as f64 / 1e18;
    let fee_bnb   = q.fee as f64 / 1e18;
    let price_per_token_bnb = if amount_raw > 0 {
        (q.funds as f64 + q.fee as f64) / 1e18 / (amount_raw as f64 / 1e18)
    } else { 0.0 };

    let resp = serde_json::json!({
        "ok": true,
        "data": {
            "action": "quote-sell",
            "chain": chain_name(args.chain),
            "chain_id": args.chain,
            "token": token,
            "symbol": sym,
            "token_manager": q.token_manager,
            "quote": q.quote,
            "input": {
                "amount":     super::fmt_decimal(amount_raw, TOKEN_DECIMALS),
                "amount_raw": amount_raw.to_string(),
            },
            "output": {
                "estimated_funds_bnb": format!("{:.8}", funds_bnb),
                "estimated_funds_wei": q.funds.to_string(),
                "estimated_fee_bnb":   format!("{:.8}", fee_bnb),
                "estimated_fee_wei":   q.fee.to_string(),
                "effective_price_bnb_per_token": format!("{:.18}", price_per_token_bnb),
            },
            "tip": format!(
                "To execute: `fourmeme-plugin sell --token {} {}` (token approve happens automatically).",
                token,
                if args.all { "--all".to_string() }
                else { format!("--amount {}", args.amount.as_ref().unwrap()) }
            ),
        }
    });
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
