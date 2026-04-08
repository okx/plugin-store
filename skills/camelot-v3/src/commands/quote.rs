use clap::Args;
use crate::config::{quoter, factory, resolve_token_address, rpc_url};
use crate::rpc::{factory_pool_by_pair, quoter_exact_input_single, get_decimals};

#[derive(Args)]
pub struct QuoteArgs {
    /// Input token (symbol or hex address, e.g. WETH or 0x82aF...)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or hex address)
    #[arg(long)]
    pub token_out: String,
    /// Amount in (raw units, e.g. 1000000000000000 for 0.001 WETH)
    #[arg(long)]
    pub amount_in: u128,
    /// Chain ID (default: 42161 Arbitrum)
    #[arg(long, default_value = "42161")]
    pub chain: u64,
}

pub async fn run(args: QuoteArgs) -> anyhow::Result<()> {
    let rpc = rpc_url(args.chain)?;
    let quoter_addr = quoter(args.chain)?;
    let factory_addr = factory(args.chain)?;
    let token_in = resolve_token_address(&args.token_in, args.chain);
    let token_out = resolve_token_address(&args.token_out, args.chain);

    // Check pool exists
    let pool_addr = factory_pool_by_pair(&token_in, &token_out, factory_addr, &rpc).await?;
    if pool_addr == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!(
            "No pool found for {} / {} on Camelot V3 (chain {})",
            token_in,
            token_out,
            args.chain
        );
    }

    let amount_out = quoter_exact_input_single(
        quoter_addr,
        &token_in,
        &token_out,
        args.amount_in,
        &rpc,
    )
    .await?;

    // Get decimals for display
    let dec_in = get_decimals(&token_in, &rpc).await.unwrap_or(18);
    let dec_out = get_decimals(&token_out, &rpc).await.unwrap_or(18);

    let amount_in_human = args.amount_in as f64 / 10f64.powi(dec_in as i32);
    let amount_out_human = amount_out as f64 / 10f64.powi(dec_out as i32);

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "pool": pool_addr,
            "token_in": token_in,
            "token_out": token_out,
            "amount_in": args.amount_in.to_string(),
            "amount_in_human": format!("{:.6}", amount_in_human),
            "amount_out": amount_out.to_string(),
            "amount_out_human": format!("{:.6}", amount_out_human),
            "chain_id": args.chain
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
