use clap::Args;
use crate::config::{factory, resolve_token_validated, rpc_url, token_symbol, CHAIN_ID};
use crate::onchainos::resolve_wallet;
use crate::rpc::{
    amm_get_pool, format_amount, get_balance_of, get_decimals,
    get_total_supply, pool_get_reserves, pool_token0,
};

#[derive(Args)]
pub struct PositionsArgs {
    /// First token of the pool (symbol or address)
    #[arg(long)]
    pub token_a: String,
    /// Second token of the pool (symbol or address)
    #[arg(long)]
    pub token_b: String,
    /// Query stable pool (default: checks both)
    #[arg(long)]
    pub stable: bool,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let fac = factory();
    let token_a = resolve_token_validated(&args.token_a)?;
    let token_b = resolve_token_validated(&args.token_b)?;

    let sym_a = resolve_symbol(&token_a, &args.token_a);
    let sym_b = resolve_symbol(&token_b, &args.token_b);

    let wallet = resolve_wallet(CHAIN_ID)?;
    let zero   = "0x0000000000000000000000000000000000000000";

    let pool_types: &[(bool, &str)] = if args.stable {
        &[(true, "stable")]
    } else {
        &[(false, "volatile"), (true, "stable")]
    };

    let mut results = vec![];

    for (is_stable, label) in pool_types {
        let pool = amm_get_pool(fac, &token_a, &token_b, *is_stable, rpc).await?;
        if pool == zero { continue; }

        let lp_balance   = get_balance_of(&pool, &wallet, rpc).await.unwrap_or(0);
        if lp_balance == 0 { continue; }

        let total_supply = get_total_supply(&pool, rpc).await.unwrap_or(1);
        let t0           = pool_token0(&pool, rpc).await.unwrap_or_default();
        let (r0, r1)     = pool_get_reserves(&pool, rpc).await.unwrap_or((0, 0));

        let (sym_0, sym_1, res_0, res_1, dec_0, dec_1) =
            if t0.to_lowercase() == token_a.to_lowercase() {
                let d0 = get_decimals(&token_a, rpc).await.unwrap_or(18);
                let d1 = get_decimals(&token_b, rpc).await.unwrap_or(18);
                (sym_a.clone(), sym_b.clone(), r0, r1, d0, d1)
            } else {
                let d0 = get_decimals(&token_b, rpc).await.unwrap_or(18);
                let d1 = get_decimals(&token_a, rpc).await.unwrap_or(18);
                (sym_b.clone(), sym_a.clone(), r0, r1, d0, d1)
            };

        // Share of pool = lp_balance / total_supply
        // Underlying = share * reserve
        let share   = lp_balance as f64 / total_supply as f64;
        let under_0 = (res_0 as f64 * share) as u128;
        let under_1 = (res_1 as f64 * share) as u128;
        let share_pct = share * 100.0;

        results.push(serde_json::json!({
            "pool": pool,
            "pool_type": label,
            "wallet": wallet,
            "lp_balance": format_amount(lp_balance, 18),
            "pool_share_pct": format!("{:.6}%", share_pct),
            "underlying": {
                sym_0: format_amount(under_0, dec_0),
                sym_1: format_amount(under_1, dec_1),
            },
            "chain": "Base (8453)",
            "tip": "Run `aerodrome-amm claim-fees` to collect accrued trading fees."
        }));
    }

    if results.is_empty() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "positions": [],
            "wallet": wallet,
            "token_a": sym_a,
            "token_b": sym_b,
            "message": "No LP positions found. Use `aerodrome-amm add-liquidity` to provide liquidity."
        }))?);
    } else {
        println!("{}", serde_json::to_string_pretty(&results)?);
    }
    Ok(())
}

fn resolve_symbol(addr: &str, fallback: &str) -> String {
    let s = token_symbol(addr);
    if s != "UNKNOWN" { s.to_string() } else { fallback.to_string() }
}
