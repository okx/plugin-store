// commands/get_pair.rs — look up V2 pair contract address
use anyhow::Result;
use serde_json::json;

use crate::config::{chain_config, resolve_token_address, is_native};
use crate::rpc;

pub struct GetPairArgs {
    pub chain_id: u64,
    pub token_a: String,
    pub token_b: String,
    pub rpc_url: Option<String>,
}

pub async fn run(args: GetPairArgs) -> Result<serde_json::Value> {
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

    let pair_addr = rpc::factory_get_pair(cfg.factory, &token_a_addr, &token_b_addr, rpc).await?;

    let exists = pair_addr != "0x0000000000000000000000000000000000000000";

    Ok(json!({
        "ok": true,
        "data": {
            "pair": pair_addr,
            "exists": exists,
            "tokenA": token_a_addr,
            "tokenB": token_b_addr,
            "factory": cfg.factory,
            "chain": args.chain_id
        }
    }))
}
