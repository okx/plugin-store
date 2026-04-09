// commands/get_reserves.rs — get pair reserves and price ratio
use anyhow::Result;
use serde_json::json;

use crate::config::{chain_config, resolve_token_address, is_native};
use crate::rpc;

pub struct GetReservesArgs {
    pub chain_id: u64,
    pub token_a: String,
    pub token_b: String,
    pub rpc_url: Option<String>,
}

pub async fn run(args: GetReservesArgs) -> Result<serde_json::Value> {
    let cfg = chain_config(args.chain_id)?;
    let rpc = args.rpc_url.as_deref().unwrap_or(cfg.rpc_url);

    let token_a_addr = if is_native(&args.token_a) {
        cfg.weth.to_string()
    } else {
        resolve_token_address(&args.token_a, args.chain_id)
    };
    let token_b_addr = if is_native(&args.token_b) {
        cfg.weth.to_string()
    } else {
        resolve_token_address(&args.token_b, args.chain_id)
    };

    let pair_addr = rpc::factory_get_pair(cfg.factory, &token_a_addr, &token_b_addr, rpc).await?;
    if pair_addr == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!("No V2 pair found for {} / {}.", args.token_a, args.token_b);
    }

    let token0 = rpc::pair_token0(&pair_addr, rpc).await?;
    let (reserve0, reserve1, ts) = rpc::pair_get_reserves(&pair_addr, rpc).await?;

    let decimals_a = rpc::erc20_decimals(&token_a_addr, rpc).await.unwrap_or(18);
    let decimals_b = rpc::erc20_decimals(&token_b_addr, rpc).await.unwrap_or(18);

    // Map token0/token1 to tokenA/tokenB
    let (reserve_a, reserve_b) = if token0.to_lowercase() == token_a_addr.to_lowercase() {
        (reserve0, reserve1)
    } else {
        (reserve1, reserve0)
    };

    let reserve_a_human = reserve_a as f64 / 10f64.powi(decimals_a as i32);
    let reserve_b_human = reserve_b as f64 / 10f64.powi(decimals_b as i32);

    // Price: how much tokenB per tokenA
    let price_b_per_a = if reserve_a > 0 {
        (reserve_b as f64 / 10f64.powi(decimals_b as i32))
            / (reserve_a as f64 / 10f64.powi(decimals_a as i32))
    } else {
        0.0
    };

    Ok(json!({
        "ok": true,
        "data": {
            "pair": pair_addr,
            "token0": token0,
            "tokenA": token_a_addr,
            "tokenB": token_b_addr,
            "reserveA": reserve_a.to_string(),
            "reserveB": reserve_b.to_string(),
            "reserveAHuman": format!("{:.6}", reserve_a_human),
            "reserveBHuman": format!("{:.6}", reserve_b_human),
            "priceBPerA": format!("{:.8}", price_b_per_a),
            "blockTimestampLast": ts,
            "swapFee": "0.25%",
            "chain": args.chain_id
        }
    }))
}
