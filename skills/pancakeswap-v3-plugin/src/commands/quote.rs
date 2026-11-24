/// `pancakeswap quote` — get swap quote via QuoterV2 eth_call.

use anyhow::Result;

pub struct QuoteArgs {
    pub from: String,
    pub to: String,
    pub amount: String,
    pub chain: u64,
}

pub async fn run(args: QuoteArgs) -> Result<()> {
    let cfg = crate::config::get_chain_config(args.chain)?;

    // Resolve token symbols to addresses
    let from_addr = crate::config::resolve_token_address(&args.from, args.chain)?;
    let to_addr = crate::config::resolve_token_address(&args.to, args.chain)?;

    if from_addr == to_addr {
        anyhow::bail!("tokenIn and tokenOut must be different tokens.");
    }

    // Resolve decimals for the input token
    let decimals_in = crate::rpc::get_decimals(&from_addr, cfg.rpc_url).await.unwrap_or(18);
    let decimals_out = crate::rpc::get_decimals(&to_addr, cfg.rpc_url).await.unwrap_or(18);
    let symbol_in = crate::rpc::get_symbol(&from_addr, cfg.rpc_url).await.unwrap_or_else(|_| args.from.clone());
    let symbol_out = crate::rpc::get_symbol(&to_addr, cfg.rpc_url).await.unwrap_or_else(|_| args.to.clone());

    let amount_in = crate::config::human_to_minimal(&args.amount, decimals_in)?;

    // Try fee tiers in order of liquidity popularity
    let fee_tiers = [500u32, 100, 2500, 10000];
    let mut best_amount_out = 0u128;
    let mut best_fee = 500u32;
    let mut errors = Vec::new();

    for fee in fee_tiers {
        match crate::rpc::quote_exact_input_single(
            cfg.quoter_v2,
            &from_addr,
            &to_addr,
            amount_in,
            fee,
            cfg.rpc_url,
        ).await {
            Ok(amount_out) if amount_out > best_amount_out => {
                best_amount_out = amount_out;
                best_fee = fee;
            }
            Ok(_) => {}
            Err(e) => errors.push(format!("fee={}: {}", fee, e)),
        }
    }

    if best_amount_out == 0 {
        anyhow::bail!("No liquidity path found for this token pair on chain {}. Use `pancakeswap pools` to verify pools exist.", args.chain);
    }

    let amount_out_human = best_amount_out as f64 / 10f64.powi(decimals_out as i32);
    let amount_in_human: f64 = args.amount.parse().unwrap_or(0.0);

    println!("Quote (chain {}):", args.chain);
    println!("  Input:      {} {}", args.amount, symbol_in);
    println!("  Output:     {:.6} {}", amount_out_human, symbol_out);
    println!("  Fee tier:   {}%", best_fee as f64 / 10000.0);
    println!(
        "  Rate:       1 {} = {:.6} {}",
        symbol_in,
        amount_out_human / amount_in_human,
        symbol_out
    );
    println!("  QuoterV2:   {}", cfg.quoter_v2);

    Ok(())
}
