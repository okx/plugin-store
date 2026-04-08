use clap::Args;
use crate::config::{factory_address, resolve_token_address, rpc_url};
use crate::rpc::factory_get_pool;

/// SushiSwap V3 fee tiers: 100 (0.01%), 500 (0.05%), 3000 (0.3%), 10000 (1%)
const ALL_FEE_TIERS: &[u32] = &[100, 500, 3000, 10000];

#[derive(Args)]
pub struct GetPoolsArgs {
    /// Token 0 (symbol or hex address)
    #[arg(long)]
    pub token0: String,
    /// Token 1 (symbol or hex address)
    #[arg(long)]
    pub token1: String,
    /// Fee tier (100/500/3000/10000). Omit to query all tiers.
    #[arg(long)]
    pub fee: Option<u32>,
    /// Chain ID (default: 8453 = Base)
    #[arg(long, default_value = "8453")]
    pub chain: u64,
}

pub async fn run(args: GetPoolsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url(args.chain)?;
    let token0 = resolve_token_address(&args.token0, args.chain);
    let token1 = resolve_token_address(&args.token1, args.chain);
    let factory = factory_address(args.chain);

    let fees_to_check: Vec<u32> = if let Some(f) = args.fee {
        vec![f]
    } else {
        ALL_FEE_TIERS.to_vec()
    };

    let mut pools = Vec::new();

    for fee in fees_to_check {
        let pool_addr = factory_get_pool(&token0, &token1, fee, factory, &rpc).await?;
        let deployed = pool_addr != "0x0000000000000000000000000000000000000000";
        println!(
            "  fee={}: {} {}",
            fee,
            pool_addr,
            if deployed { "(deployed)" } else { "(not deployed)" }
        );
        pools.push(serde_json::json!({
            "fee": fee,
            "feePct": format!("{:.2}%", fee as f64 / 10000.0),
            "address": pool_addr,
            "deployed": deployed,
        }));
    }

    println!(
        "{{\"ok\":true,\"token0\":\"{}\",\"token1\":\"{}\",\"chain\":{},\"pools\":{}}}",
        token0,
        token1,
        args.chain,
        serde_json::to_string(&pools)?
    );

    Ok(())
}
