// commands/quote.rs — getAmountsOut quote (read-only)
use anyhow::Result;
use serde_json::json;

use crate::config::{chain_config, resolve_token_address, is_native};
use crate::rpc;

pub struct QuoteArgs {
    pub chain_id: u64,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: String,
    pub rpc_url: Option<String>,
}

pub async fn run(args: QuoteArgs) -> Result<serde_json::Value> {
    let cfg = chain_config(args.chain_id)?;
    let rpc = args.rpc_url.as_deref().unwrap_or(cfg.rpc_url);

    let token_in = resolve_token_address(&args.token_in, args.chain_id);
    let token_out = resolve_token_address(&args.token_out, args.chain_id);

    if token_in == token_out {
        anyhow::bail!("tokenIn and tokenOut must be different tokens.");
    }

    // Handle native BNB/ETH: map to WBNB/WETH for routing
    let token_in_addr = if is_native(&args.token_in) {
        cfg.weth.to_string()
    } else {
        token_in.clone()
    };
    let token_out_addr = if is_native(&args.token_out) {
        cfg.weth.to_string()
    } else {
        token_out.clone()
    };

    // Resolve decimals for tokenIn and parse human-readable amount
    let decimals_in = rpc::erc20_decimals(&token_in_addr, rpc).await.unwrap_or(18);
    let amount_in_raw = rpc::parse_human_amount(&args.amount_in, decimals_in)?;
    if amount_in_raw == 0 {
        anyhow::bail!("Amount must be greater than 0.");
    }

    // Determine path: try direct pair first, then route via WETH/WBNB
    let path = determine_path(
        &token_in_addr,
        &token_out_addr,
        cfg.factory,
        cfg.weth,
        rpc,
    )
    .await?;

    let path_refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    let amounts = rpc::router_get_amounts_out(cfg.router02, amount_in_raw, &path_refs, rpc).await?;

    let amount_out = *amounts.last().unwrap_or(&0);

    // Get symbols for display
    let symbol_in = rpc::erc20_symbol(&token_in_addr, rpc).await.unwrap_or_else(|_| args.token_in.clone());
    let symbol_out = rpc::erc20_symbol(&token_out_addr, rpc).await.unwrap_or_else(|_| args.token_out.clone());
    let decimals_out = rpc::erc20_decimals(&token_out_addr, rpc).await.unwrap_or(18);

    let amount_out_human = amount_out as f64 / 10f64.powi(decimals_out as i32);

    Ok(json!({
        "ok": true,
        "data": {
            "tokenIn": token_in_addr,
            "tokenOut": token_out_addr,
            "symbolIn": symbol_in,
            "symbolOut": symbol_out,
            "amountIn": amount_in_raw.to_string(),
            "amountOut": amount_out.to_string(),
            "amountOutHuman": format!("{:.6}", amount_out_human),
            "path": path,
            "fee": "0.25%",
            "chain": args.chain_id
        }
    }))
}

/// Determine the best swap path: direct or via WETH/WBNB
pub async fn determine_path(
    token_in: &str,
    token_out: &str,
    factory: &str,
    weth: &str,
    rpc_url: &str,
) -> Result<Vec<String>> {
    // Try direct pair
    let direct_pair = rpc::factory_get_pair(factory, token_in, token_out, rpc_url).await?;
    if direct_pair != "0x0000000000000000000000000000000000000000" {
        return Ok(vec![token_in.to_string(), token_out.to_string()]);
    }

    // Try via WETH/WBNB
    if token_in.to_lowercase() != weth.to_lowercase()
        && token_out.to_lowercase() != weth.to_lowercase()
    {
        let hop1 = rpc::factory_get_pair(factory, token_in, weth, rpc_url).await?;
        let hop2 = rpc::factory_get_pair(factory, weth, token_out, rpc_url).await?;
        if hop1 != "0x0000000000000000000000000000000000000000"
            && hop2 != "0x0000000000000000000000000000000000000000"
        {
            return Ok(vec![
                token_in.to_string(),
                weth.to_string(),
                token_out.to_string(),
            ]);
        }
    }

    anyhow::bail!(
        "No V2 liquidity path found between {} and {}",
        token_in,
        token_out
    )
}
