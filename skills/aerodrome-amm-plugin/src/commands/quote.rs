use clap::Args;
use crate::config::{factory, resolve_token_validated, rpc_url, token_symbol, router};
use crate::rpc::{amm_get_pool, format_amount, get_decimals, parse_human_amount, router_get_amounts_out};

#[derive(Args)]
pub struct QuoteArgs {
    /// Input token (symbol or address, e.g. WETH, USDC)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or address)
    #[arg(long)]
    pub token_out: String,
    /// Amount of token_in (human-readable, e.g. "0.1")
    #[arg(long)]
    pub amount_in: String,
    /// Force stable pool (default: try both, show best)
    #[arg(long)]
    pub stable: bool,
}

pub async fn run(args: QuoteArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let fac = factory();
    let rtr = router();
    let token_in  = resolve_token_validated(&args.token_in)?;
    let token_out = resolve_token_validated(&args.token_out)?;

    let sym_in  = resolve_symbol(&token_in, &args.token_in);
    let sym_out = resolve_symbol(&token_out, &args.token_out);

    let dec_in  = get_decimals(&token_in, rpc).await.unwrap_or(18);
    let dec_out = get_decimals(&token_out, rpc).await.unwrap_or(18);
    let amount_in_raw = parse_human_amount(&args.amount_in, dec_in)?;

    if amount_in_raw == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    let zero = "0x0000000000000000000000000000000000000000";
    let mut results = vec![];

    let pool_types: &[(bool, &str)] = if args.stable {
        &[(true, "stable")]
    } else {
        &[(false, "volatile"), (true, "stable")]
    };

    for (is_stable, label) in pool_types {
        let pool = amm_get_pool(fac, &token_in, &token_out, *is_stable, rpc).await?;
        if pool == zero { continue; }
        match router_get_amounts_out(rtr, fac, amount_in_raw, &token_in, &token_out, *is_stable, rpc).await {
            Ok(amount_out) if amount_out > 0 => {
                results.push(serde_json::json!({
                    "pool_type": label,
                    "pool": pool,
                    "token_in": sym_in,
                    "token_out": sym_out,
                    "amount_in": args.amount_in,
                    "amount_out": format_amount(amount_out, dec_out),
                    "chain": "Base (8453)"
                }));
            }
            _ => {}
        }
    }

    if results.is_empty() {
        anyhow::bail!(
            "No AMM pool found for {}/{} on Base. \
             Check if tokens have liquidity with `aerodrome-amm pools --token-a {} --token-b {}`",
            sym_in, sym_out, args.token_in, args.token_out
        );
    }

    // Sort by best output
    results.sort_by(|a, b| {
        let ao_a: f64 = a["amount_out"].as_str().unwrap_or("0").parse().unwrap_or(0.0);
        let ao_b: f64 = b["amount_out"].as_str().unwrap_or("0").parse().unwrap_or(0.0);
        ao_b.partial_cmp(&ao_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}

fn resolve_symbol(addr: &str, fallback: &str) -> String {
    let s = token_symbol(addr);
    if s != "UNKNOWN" { s.to_string() } else { fallback.to_string() }
}
