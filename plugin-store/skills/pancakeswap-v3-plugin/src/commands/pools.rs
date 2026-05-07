/// `pancakeswap pools` — list pools for a token pair via PancakeV3Factory.

use anyhow::Result;

pub struct PoolsArgs {
    pub token0: String,
    pub token1: String,
    pub chain: u64,
}

pub async fn run(args: PoolsArgs) -> Result<()> {
    let cfg = crate::config::get_chain_config(args.chain)?;

    let addr0 = crate::config::resolve_token_address(&args.token0, args.chain)?;
    let addr1 = crate::config::resolve_token_address(&args.token1, args.chain)?;

    let sym0 = crate::rpc::get_symbol(&addr0, cfg.rpc_url).await.unwrap_or_else(|_| args.token0.clone());
    let sym1 = crate::rpc::get_symbol(&addr1, cfg.rpc_url).await.unwrap_or_else(|_| args.token1.clone());

    println!("Pools for {}/{} on chain {} (factory: {})", sym0, sym1, args.chain, cfg.factory);
    println!("{:<8} {:<44} {:>14} {:>12}", "Fee", "Pool Address", "Liquidity", "sqrtPrice");
    println!("{}", "-".repeat(80));

    let fee_tiers = [100u32, 500, 2500, 10000];
    let mut found = 0;

    for fee in fee_tiers {
        match crate::rpc::get_pool_address(cfg.factory, &addr0, &addr1, fee, cfg.rpc_url).await {
            Ok(pool_addr) => {
                found += 1;
                let fee_label = format!("{:.2}%", fee as f64 / 10000.0);

                // Query slot0 and liquidity — surface RPC errors explicitly
                // so agents don't mistake rate-limit failures for tick=0 bugs.
                let slot0 = crate::rpc::get_slot0(&pool_addr, cfg.rpc_url).await;
                let liq = crate::rpc::get_pool_liquidity(&pool_addr, cfg.rpc_url).await;

                match (slot0, liq) {
                    (Ok((sqrt_price, tick)), Ok(liquidity)) => {
                        let price = if sqrt_price > 0 {
                            let sq = sqrt_price as f64 / 2f64.powi(96);
                            format!("{:.4}", sq * sq)
                        } else {
                            "N/A".to_string()
                        };
                        println!(
                            "{:<8} {:<44} {:>14} {:>12}",
                            fee_label, pool_addr, liquidity, price,
                        );
                        println!("         tick: {}", tick);
                    }
                    (slot0_res, liq_res) => {
                        let err = slot0_res.err()
                            .or(liq_res.err())
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "unknown error".to_string());
                        println!(
                            "{:<8} {:<44} [RPC error — try again or check rate limits]",
                            fee_label, pool_addr,
                        );
                        println!("         error: {}", err);
                    }
                }
            }
            Err(_) => {
                // Pool doesn't exist for this fee tier — skip silently
            }
        }
    }

    if found == 0 {
        println!("No pools found for this token pair on chain {}.", args.chain);
        println!("Verify the token addresses are correct.");
    } else {
        println!("\nFound {} pool(s).", found);
    }

    Ok(())
}
