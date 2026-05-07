use clap::Args;
use crate::config::{factory_address, resolve_token_address, router_address, rpc_url};
use crate::rpc::{factory_get_pool, get_erc20_decimals, parse_human_amount, router_get_amounts_out};

#[derive(Args)]
pub struct QuoteArgs {
    /// Input token (symbol or hex address, e.g. USDC, WETH, 0x...)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or hex address)
    #[arg(long)]
    pub token_out: String,
    /// Amount in (human-readable decimal, e.g. 0.0001 for 0.0001 WETH, 1 for 1 USDC)
    #[arg(long)]
    pub amount_in: String,
    /// Use stable pool (false = volatile, true = stable). If omitted, tries both and returns best.
    #[arg(long)]
    pub stable: Option<bool>,
}

pub async fn run(args: QuoteArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let token_in = resolve_token_address(&args.token_in);
    let token_out = resolve_token_address(&args.token_out);
    let factory = factory_address();
    let router = router_address();

    // Parse human-readable amount
    let decimals_in = get_erc20_decimals(&token_in, rpc).await?;
    let amount_in = parse_human_amount(&args.amount_in, decimals_in)?;

    let stable_options: Vec<bool> = match args.stable {
        Some(s) => vec![s],
        None => vec![false, true],
    };

    let mut best_amount_out: u128 = 0;
    let mut best_stable: bool = false;
    let mut best_pool: String = String::new();

    for stable in stable_options {
        let pool_addr = factory_get_pool(&token_in, &token_out, stable, factory, rpc).await?;
        if pool_addr == "0x0000000000000000000000000000000000000000" {
            println!("  stable={}: pool not deployed, skipping", stable);
            continue;
        }

        match router_get_amounts_out(router, amount_in, &token_in, &token_out, stable, factory, rpc).await {
            Ok(amount_out) => {
                println!("  stable={}: pool={} amountOut={}", stable, pool_addr, amount_out);
                if amount_out > best_amount_out {
                    best_amount_out = amount_out;
                    best_stable = stable;
                    best_pool = pool_addr;
                }
            }
            Err(e) => {
                println!("  stable={}: quote failed: {}", stable, e);
            }
        }
    }

    if best_amount_out == 0 {
        println!("{{\"ok\":false,\"error\":\"No valid quote found for any pool type\"}}");
    } else {
        println!(
            "{{\"ok\":true,\"tokenIn\":\"{}\",\"tokenOut\":\"{}\",\"amountIn\":{},\"stable\":{},\"pool\":\"{}\",\"amountOut\":{}}}",
            token_in, token_out, amount_in, best_stable, best_pool, best_amount_out
        );
    }

    Ok(())
}
