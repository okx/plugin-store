/// `fourmeme-plugin tax-info --token 0x...` — read fee/tax config of a TaxToken.
///
/// Only valid for tokens of type "TaxToken" (creatorType 5). Calls 9 view methods
/// in parallel via reqwest::futures::join_all.

use anyhow::Result;
use clap::Args;
use futures::future::try_join_all;

use crate::config::is_supported_chain;
use crate::rpc::{eth_call, parse_address, parse_uint256_to_u128};

#[derive(Args)]
pub struct TaxInfoArgs {
    #[arg(long)]
    pub token: String,

    #[arg(long, default_value_t = 56)]
    pub chain: u64,
}

pub async fn run(args: TaxInfoArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("tax-info"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: TaxInfoArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    let token = args.token.to_lowercase();

    use crate::calldata::*;
    let calls = [
        ("feeRate",       SEL_TAX_FEE_RATE),
        ("rateFounder",   SEL_TAX_RATE_FOUNDER),
        ("rateHolder",    SEL_TAX_RATE_HOLDER),
        ("rateBurn",      SEL_TAX_RATE_BURN),
        ("rateLiquidity", SEL_TAX_RATE_LIQUIDITY),
        ("minDispatch",   SEL_TAX_MIN_DISPATCH),
        ("minShare",      SEL_TAX_MIN_SHARE),
        ("quote",         SEL_TAX_QUOTE),
        ("founder",       SEL_TAX_FOUNDER),
    ];

    let futs = calls.iter().map(|(_name, sel)| {
        let data = build_no_args(sel);
        let token = token.clone();
        async move { eth_call(args.chain, &token, &data).await }
    });
    let results = try_join_all(futs).await
        .map_err(|e| anyhow::anyhow!("tax-info eth_calls failed (token may not be a TaxToken): {}", e))?;

    let fee_rate       = parse_uint256_to_u128(&results[0]);
    let rate_founder   = parse_uint256_to_u128(&results[1]);
    let rate_holder    = parse_uint256_to_u128(&results[2]);
    let rate_burn      = parse_uint256_to_u128(&results[3]);
    let rate_liquidity = parse_uint256_to_u128(&results[4]);
    let min_dispatch   = parse_uint256_to_u128(&results[5]);
    let min_share      = parse_uint256_to_u128(&results[6]);
    let quote          = parse_address(&results[7]);
    let founder        = parse_address(&results[8]);

    let zero = "0x0000000000000000000000000000000000000000";
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "token": token,
            "fee_rate_bps":     fee_rate.to_string(),
            "fee_rate_percent": format!("{:.4}", fee_rate as f64 / 100.0),
            "rate_founder":     rate_founder.to_string(),
            "rate_holder":      rate_holder.to_string(),
            "rate_burn":        rate_burn.to_string(),
            "rate_liquidity":   rate_liquidity.to_string(),
            "min_dispatch":     min_dispatch.to_string(),
            "min_share":        min_share.to_string(),
            "quote":            if quote == zero { serde_json::Value::Null } else { serde_json::Value::String(quote) },
            "founder":          if founder == zero { serde_json::Value::Null } else { serde_json::Value::String(founder) },
        }
    }))?);
    Ok(())
}
