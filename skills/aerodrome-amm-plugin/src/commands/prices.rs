use clap::Args;
use crate::config::{factory, resolve_token_validated, rpc_url, token_symbol};
use crate::rpc::{amm_get_pool, get_decimals, pool_get_reserves, pool_token0};

#[derive(Args)]
pub struct PricesArgs {
    /// Token to price (symbol or address, e.g. WETH)
    #[arg(long)]
    pub token: String,
    /// Quote currency (default: USDC)
    #[arg(long, default_value = "USDC")]
    pub quote: String,
}

pub async fn run(args: PricesArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let fac = factory();
    let token     = resolve_token_validated(&args.token)?;
    let quote_tok = resolve_token_validated(&args.quote)?;

    let sym_tok   = resolve_symbol(&token, &args.token);
    let sym_quote = resolve_symbol(&quote_tok, &args.quote);

    let zero = "0x0000000000000000000000000000000000000000";
    let dec_tok   = get_decimals(&token, rpc).await.unwrap_or(18);
    let dec_quote = get_decimals(&quote_tok, rpc).await.unwrap_or(6);

    let mut prices = vec![];

    for is_stable in [false, true] {
        let pool = amm_get_pool(fac, &token, &quote_tok, is_stable, rpc).await?;
        if pool == zero { continue; }

        let t0 = pool_token0(&pool, rpc).await.unwrap_or_default();
        let (r0, r1) = pool_get_reserves(&pool, rpc).await.unwrap_or((0, 0));

        let (tok_reserve, quote_reserve, tok_dec, quote_dec) =
            if t0.to_lowercase() == token.to_lowercase() {
                (r0, r1, dec_tok, dec_quote)
            } else {
                (r1, r0, dec_tok, dec_quote)
            };

        if tok_reserve == 0 { continue; }
        let price = (quote_reserve as f64 / 10f64.powi(quote_dec as i32))
            / (tok_reserve as f64 / 10f64.powi(tok_dec as i32));

        prices.push(serde_json::json!({
            "token": sym_tok,
            "quote": sym_quote,
            "price": format!("{:.6}", price),
            "pool_type": if is_stable { "stable" } else { "volatile" },
            "pool": pool,
            "chain": "Base (8453)"
        }));
    }

    if prices.is_empty() {
        anyhow::bail!(
            "No {}/{} pool found on Aerodrome AMM Base. \
             Try a different quote token (e.g. --quote WETH).",
            sym_tok, sym_quote
        );
    }

    println!("{}", serde_json::to_string_pretty(&prices)?);
    Ok(())
}

fn resolve_symbol(addr: &str, fallback: &str) -> String {
    let s = token_symbol(addr);
    if s != "UNKNOWN" { s.to_string() } else { fallback.to_string() }
}
