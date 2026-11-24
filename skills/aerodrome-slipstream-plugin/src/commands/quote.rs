use clap::Args;
use crate::config::{cl_factory, common_tick_spacings, quoter, resolve_token, rpc_url, token_symbol};
use crate::rpc::{cl_get_pool, get_decimals, format_amount, parse_human_amount};

#[derive(Args)]
pub struct QuoteArgs {
    /// Input token (symbol or address)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or address)
    #[arg(long)]
    pub token_out: String,
    /// Amount in (human-readable, e.g. "0.1" for 0.1 WETH)
    #[arg(long)]
    pub amount_in: String,
    /// Specific tick spacing to quote (default: auto-select best)
    #[arg(long)]
    pub tick_spacing: Option<i32>,
}

/// Quoter.quoteExactInputSingle(QuoteExactInputSingleParams memory params)
/// Struct fields (all static → encoded inline, no offset pointer):
///   address tokenIn, address tokenOut, uint256 amountIn, int24 tickSpacing, uint160 sqrtPriceLimitX96
/// Selector: keccak("quoteExactInputSingle((address,address,uint256,int24,uint160))") = 0x9e7defe6
pub async fn get_quote(
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    tick_spacing: i32,
    rpc: &str,
) -> anyhow::Result<u128> {
    let quoter_addr = quoter();
    let ta = format!("{:0>64}", token_in.trim_start_matches("0x").to_lowercase());
    let tb = format!("{:0>64}", token_out.trim_start_matches("0x").to_lowercase());
    let amt = format!("{:0>64x}", amount_in);
    let ts = format!("{:0>64x}", tick_spacing as u64);
    let limit = format!("{:0>64x}", 0u64); // sqrtPriceLimitX96 = 0 means no limit
    let data = format!("0x9e7defe6{}{}{}{}{}", ta, tb, amt, ts, limit);
    let hex = crate::rpc::eth_call(quoter_addr, &data, rpc).await?;
    // Returns (uint256 amountOut, ...) — first 32 bytes is amountOut
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 64 {
        anyhow::bail!("Quoter returned no data for this pool/amount");
    }
    Ok(u128::from_str_radix(&clean[..64], 16).unwrap_or(0))
}

pub async fn run(args: QuoteArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let factory = cl_factory();
    let token_in = resolve_token(&args.token_in);
    let token_out = resolve_token(&args.token_out);

    let sym_in  = if token_symbol(&token_in) != "UNKNOWN"  { token_symbol(&token_in).to_string()  } else { args.token_in.clone() };
    let sym_out = if token_symbol(&token_out) != "UNKNOWN" { token_symbol(&token_out).to_string() } else { args.token_out.clone() };

    let dec_in  = get_decimals(&token_in, rpc).await.unwrap_or(18);
    let dec_out = get_decimals(&token_out, rpc).await.unwrap_or(6);
    let amount_raw = parse_human_amount(&args.amount_in, dec_in)?;

    if amount_raw == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    let zero = "0x0000000000000000000000000000000000000000";
    let tick_spacings: Vec<i32> = match args.tick_spacing {
        Some(ts) => vec![ts],
        None => common_tick_spacings().to_vec(),
    };

    let mut best_out: u128 = 0;
    let mut best_ts = 0i32;

    for ts in &tick_spacings {
        // Verify pool exists before trying to quote
        let pool = cl_get_pool(factory, &token_in, &token_out, *ts, rpc).await?;
        if pool == zero { continue; }

        match get_quote(&token_in, &token_out, amount_raw, *ts, rpc).await {
            Ok(out) if out > best_out => {
                best_out = out;
                best_ts = *ts;
            }
            _ => {}
        }
    }

    if best_out == 0 {
        anyhow::bail!(
            "No quote available for {} {} → {}. Check that a CL pool exists and has sufficient liquidity.",
            args.amount_in, sym_in, sym_out
        );
    }

    let amount_out_human = format_amount(best_out, dec_out);
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "token_in": sym_in,
        "token_out": sym_out,
        "amount_in": args.amount_in,
        "amount_out": amount_out_human,
        "amount_out_raw": best_out.to_string(),
        "tick_spacing": best_ts,
        "chain": "Base (8453)"
    }))?);
    Ok(())
}
