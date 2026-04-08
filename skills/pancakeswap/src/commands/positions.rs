/// `pancakeswap positions` — view LP positions for a wallet address.
/// Uses TheGraph subgraph for BSC; on-chain enumeration fallback for Base or if subgraph fails.

use anyhow::Result;

pub struct PositionsArgs {
    pub owner: String,
    pub chain: u64,
}

pub async fn run(args: PositionsArgs) -> Result<()> {
    let cfg = crate::config::get_chain_config(args.chain)?;

    println!("LP Positions for {} on chain {}:", args.owner, args.chain);
    println!();

    // Try subgraph first
    match query_subgraph(cfg, &args.owner).await {
        Ok(true) => return Ok(()),
        Ok(false) => {
            println!("No positions found via subgraph. Trying on-chain enumeration...");
        }
        Err(e) => {
            eprintln!("Subgraph query failed: {}. Falling back to on-chain enumeration.", e);
        }
    }

    // On-chain fallback: enumerate via NonfungiblePositionManager
    query_onchain(cfg, &args.owner).await
}

async fn query_subgraph(
    cfg: &crate::config::ChainConfig,
    owner: &str,
) -> Result<bool> {
    let resp = crate::rpc::query_positions_subgraph(cfg.subgraph_url, owner).await?;

    let positions = resp["data"]["positions"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Unexpected subgraph response format"))?;

    if positions.is_empty() {
        return Ok(false);
    }

    println!("Found {} position(s) via subgraph:\n", positions.len());

    for pos in positions {
        let id = pos["id"].as_str().unwrap_or("?");
        let sym0 = pos["token0"]["symbol"].as_str().unwrap_or("?");
        let sym1 = pos["token1"]["symbol"].as_str().unwrap_or("?");
        let fee = pos["feeTier"].as_str().unwrap_or("?");
        let liquidity = pos["liquidity"].as_str().unwrap_or("0");
        let tick_lower = pos["tickLower"]["tickIdx"].as_str().unwrap_or("?");
        let tick_upper = pos["tickUpper"]["tickIdx"].as_str().unwrap_or("?");
        let dep0 = pos["depositedToken0"].as_str().unwrap_or("0");
        let dep1 = pos["depositedToken1"].as_str().unwrap_or("0");
        let fee0 = pos["collectedFeesToken0"].as_str().unwrap_or("0");
        let fee1 = pos["collectedFeesToken1"].as_str().unwrap_or("0");

        println!("  Position #{}", id);
        println!("    Pair:       {}/{}", sym0, sym1);
        println!("    Fee tier:   {}%", fee.parse::<f64>().unwrap_or(0.0) / 10000.0);
        println!("    Tick range: {} to {}", tick_lower, tick_upper);
        println!("    Liquidity:  {}", liquidity);
        println!("    Deposited:  {} {} / {} {}", dep0, sym0, dep1, sym1);
        println!("    Fees coll.: {} {} / {} {}", fee0, sym0, fee1, sym1);
        println!();
    }

    Ok(true)
}

async fn query_onchain(
    cfg: &crate::config::ChainConfig,
    owner: &str,
) -> Result<()> {
    let token_ids = crate::rpc::get_token_ids_for_owner(cfg.npm, owner, cfg.rpc_url).await?;

    if token_ids.is_empty() {
        println!("No LP positions found for {} on chain {}.", owner, cfg.chain_id);
        return Ok(());
    }

    println!("Found {} position(s) on-chain:\n", token_ids.len());

    for token_id in token_ids {
        match crate::rpc::get_position(cfg.npm, token_id, cfg.rpc_url).await {
            Ok(pos) => {
                let sym0 = crate::rpc::get_symbol(&pos.token0, cfg.rpc_url).await.unwrap_or_else(|_| pos.token0.clone());
                let sym1 = crate::rpc::get_symbol(&pos.token1, cfg.rpc_url).await.unwrap_or_else(|_| pos.token1.clone());

                println!("  Position #{}", token_id);
                println!("    Pair:         {}/{}", sym0, sym1);
                println!("    Fee tier:     {}%", pos.fee as f64 / 10000.0);
                println!("    Tick range:   {} to {}", pos.tick_lower, pos.tick_upper);
                println!("    Liquidity:    {}", pos.liquidity);
                println!("    Owed fees:    {} {} / {} {}", pos.tokens_owed0, sym0, pos.tokens_owed1, sym1);
                println!();
            }
            Err(e) => {
                eprintln!("  Error fetching position #{}: {}", token_id, e);
            }
        }
    }

    Ok(())
}
