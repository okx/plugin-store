// commands/lp_balance.rs — get user LP token balance for a pair
use anyhow::Result;
use serde_json::json;

use crate::config::{chain_config, resolve_token_address, is_native};
use crate::onchainos;
use crate::rpc;

pub struct LpBalanceArgs {
    pub chain_id: u64,
    pub token_a: String,
    pub token_b: String,
    pub wallet: Option<String>,
    pub rpc_url: Option<String>,
}

pub async fn run(args: LpBalanceArgs) -> Result<serde_json::Value> {
    let cfg = chain_config(args.chain_id)?;
    let rpc = args.rpc_url.as_deref().unwrap_or(cfg.rpc_url);

    let token_a_addr = if is_native(&args.token_a) {
        cfg.weth.to_string()
    } else {
        resolve_token_address(&args.token_a, args.chain_id)?
    };
    let token_b_addr = if is_native(&args.token_b) {
        cfg.weth.to_string()
    } else {
        resolve_token_address(&args.token_b, args.chain_id)?
    };

    // Resolve wallet
    let wallet = args.wallet
        .unwrap_or_else(|| onchainos::resolve_wallet(args.chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --wallet or ensure onchainos is logged in.");
    }

    // Get pair
    let pair_addr = rpc::factory_get_pair(cfg.factory, &token_a_addr, &token_b_addr, rpc).await?;
    if pair_addr == "0x0000000000000000000000000000000000000000" {
        return Ok(json!({
            "ok": true,
            "data": {
                "pair": null,
                "lpBalance": "0",
                "message": "No V2 pair found for this token pair."
            }
        }));
    }

    let lp_balance = rpc::erc20_balance_of(&pair_addr, &wallet, rpc).await.unwrap_or(0);
    let total_supply = rpc::erc20_total_supply(&pair_addr, rpc).await.unwrap_or(1);

    let (reserve0, reserve1, _) = rpc::pair_get_reserves(&pair_addr, rpc).await?;
    let token0 = rpc::pair_token0(&pair_addr, rpc).await?;
    let (reserve_a, reserve_b) = if token0.to_lowercase() == token_a_addr.to_lowercase() {
        (reserve0, reserve1)
    } else {
        (reserve1, reserve0)
    };

    // User's share of pool
    let share = if total_supply > 0 {
        lp_balance as f64 / total_supply as f64
    } else {
        0.0
    };

    let decimals_a = rpc::erc20_decimals(&token_a_addr, rpc).await.unwrap_or(18);
    let decimals_b = rpc::erc20_decimals(&token_b_addr, rpc).await.unwrap_or(18);

    let token_a_owned = reserve_a as f64 * share / 10f64.powi(decimals_a as i32);
    let token_b_owned = reserve_b as f64 * share / 10f64.powi(decimals_b as i32);

    Ok(json!({
        "ok": true,
        "data": {
            "pair": pair_addr,
            "wallet": wallet,
            "lpBalance": lp_balance.to_string(),
            "lpBalanceHuman": format!("{:.18}", lp_balance as f64 / 1e18),
            "totalSupply": total_supply.to_string(),
            "poolShare": format!("{:.6}%", share * 100.0),
            "tokenA": token_a_addr,
            "tokenB": token_b_addr,
            "tokenAOwned": format!("{:.6}", token_a_owned),
            "tokenBOwned": format!("{:.6}", token_b_owned),
            "chain": args.chain_id
        }
    }))
}
