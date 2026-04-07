use clap::Args;
use crate::config::{factory_address, quoter_v2_address, resolve_token_address, rpc_url};
use crate::rpc::{factory_get_pool, quoter_exact_input_single};

/// SushiSwap V3 fee tiers: 100 (0.01%), 500 (0.05%), 3000 (0.3%), 10000 (1%)
const FEE_TIERS: &[u32] = &[100, 500, 3000, 10000];

#[derive(Args)]
pub struct QuoteArgs {
    /// Input token (symbol or hex address, e.g. WETH or 0x4200...)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or hex address)
    #[arg(long)]
    pub token_out: String,
    /// Amount to swap in smallest token unit (e.g. wei for ETH)
    #[arg(long)]
    pub amount_in: u128,
    /// Fee tier (100/500/3000/10000). Omit to auto-select best.
    #[arg(long)]
    pub fee: Option<u32>,
    /// Chain ID (default: 8453 = Base)
    #[arg(long, default_value = "8453")]
    pub chain: u64,
}

pub async fn run(args: QuoteArgs) -> anyhow::Result<()> {
    let rpc = rpc_url(args.chain)?;
    let token_in = resolve_token_address(&args.token_in, args.chain);
    let token_out = resolve_token_address(&args.token_out, args.chain);
    let factory = factory_address(args.chain);
    let quoter = quoter_v2_address(args.chain);

    let fees_to_check: Vec<u32> = if let Some(f) = args.fee {
        vec![f]
    } else {
        FEE_TIERS.to_vec()
    };

    let mut best_amount_out: u128 = 0;
    let mut best_fee: u32 = 0;

    for fee in fees_to_check {
        let pool_addr = factory_get_pool(&token_in, &token_out, fee, factory, &rpc).await?;
        if pool_addr == "0x0000000000000000000000000000000000000000" {
            println!("  fee={}: pool not deployed, skipping", fee);
            continue;
        }
        match quoter_exact_input_single(quoter, &token_in, &token_out, args.amount_in, fee, &rpc).await {
            Ok(amount_out) => {
                println!("  fee={}: amountOut={}", fee, amount_out);
                if amount_out > best_amount_out {
                    best_amount_out = amount_out;
                    best_fee = fee;
                }
            }
            Err(e) => {
                println!("  fee={}: quote failed: {}", fee, e);
            }
        }
    }

    if best_amount_out == 0 {
        println!("{{\"ok\":false,\"error\":\"No valid quote found for any fee tier\"}}");
    } else {
        println!(
            "{{\"ok\":true,\"tokenIn\":\"{}\",\"tokenOut\":\"{}\",\"amountIn\":{},\"bestFee\":{},\"amountOut\":{}}}",
            token_in, token_out, args.amount_in, best_fee, best_amount_out
        );
    }

    Ok(())
}
