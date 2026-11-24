use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{
    build_approve_calldata, build_remove_liquidity_calldata, factory_address,
    resolve_token_address, router_address, rpc_url, unix_now,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{factory_get_pool, get_allowance, get_balance, get_erc20_decimals, parse_human_amount};

const CHAIN_ID: u64 = 10;

#[derive(Args)]
pub struct RemoveLiquidityArgs {
    /// Token A (symbol or hex address)
    #[arg(long)]
    pub token_a: String,
    /// Token B (symbol or hex address)
    #[arg(long)]
    pub token_b: String,
    /// Use stable pool (omit for volatile, add flag for stable)
    #[arg(long, default_value_t = false)]
    pub stable: bool,
    /// Amount of LP tokens to remove (human-readable decimal). If omitted, removes all LP tokens.
    #[arg(long)]
    pub liquidity: Option<String>,
    /// Minimum acceptable amount of token A (human-readable decimal, 0 = no minimum)
    #[arg(long, default_value = "0")]
    pub amount_a_min: String,
    /// Minimum acceptable amount of token B (human-readable decimal, 0 = no minimum)
    #[arg(long, default_value = "0")]
    pub amount_b_min: String,
    /// Transaction deadline in minutes from now
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Dry run -- build calldata but do not broadcast
    #[arg(long)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction (without this flag, prints a preview only)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: RemoveLiquidityArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let token_a = resolve_token_address(&args.token_a);
    let token_b = resolve_token_address(&args.token_b);
    let factory = factory_address();
    let router = router_address();

    // --- 1. Look up pool ---
    let pool_addr = factory_get_pool(&token_a, &token_b, args.stable, factory, rpc).await?;
    if pool_addr == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!(
            "Pool does not exist for {}/{} stable={}",
            token_a, token_b, args.stable
        );
    }
    println!("Pool: {}", pool_addr);

    // --- 2. Parse min amounts (LP tokens have 18 decimals; token mins use their own decimals) ---
    let decimals_a = get_erc20_decimals(&token_a, rpc).await?;
    let decimals_b = get_erc20_decimals(&token_b, rpc).await?;
    let amount_a_min = parse_human_amount(&args.amount_a_min, decimals_a)?;
    let amount_b_min = parse_human_amount(&args.amount_b_min, decimals_b)?;

    // LP tokens always have 18 decimals
    let lp_decimals: u8 = 18;

    // --- 3. Resolve wallet and LP balance ---
    let wallet = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    let lp_balance = if args.dry_run {
        args.liquidity.as_deref().map(|s| parse_human_amount(s, lp_decimals)).transpose()?.unwrap_or(1_000_000_000_000_000_000u128) // mock 1 LP for dry run
    } else {
        get_balance(&pool_addr, &wallet, rpc).await?
    };

    let liquidity_to_remove = match args.liquidity.as_deref() {
        Some(s) => parse_human_amount(s, lp_decimals)?,
        None => lp_balance,
    };

    if !args.dry_run && liquidity_to_remove == 0 {
        println!("{{\"ok\":false,\"error\":\"No LP token balance to remove\"}}");
        return Ok(());
    }

    // Preview gate — emit structured preview and exit before any on-chain writes
    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "preview": true,
            "message": "Add --confirm to broadcast",
            "data": {
                "action": "remove-liquidity",
                "pool": pool_addr,
                "token_a": token_a,
                "token_b": token_b,
                "stable": args.stable,
                "lp_balance": lp_balance,
                "liquidity_to_remove": liquidity_to_remove
            }
        }))?);
        return Ok(());
    }

    // --- 4. Approve LP token -> Router ---
    if !args.dry_run {
        let lp_allowance = get_allowance(&pool_addr, &wallet, router, rpc).await?;
        if lp_allowance < liquidity_to_remove {
            println!("Approving LP token ({}) for Router...", pool_addr);
            let approve_data = build_approve_calldata(router, liquidity_to_remove);
            let res = wallet_contract_call(CHAIN_ID, &pool_addr, &approve_data, args.confirm, false).await?;
            println!("Approve LP tx: {}", extract_tx_hash(&res));
            sleep(Duration::from_secs(3)).await;
        }
    }

    // --- 5. Build removeLiquidity calldata ---
    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = build_remove_liquidity_calldata(
        &token_a,
        &token_b,
        args.stable,
        liquidity_to_remove,
        amount_a_min,
        amount_b_min,
        &wallet,
        deadline,
    );

    let result = wallet_contract_call(CHAIN_ID, router, &calldata, args.confirm, args.dry_run).await?;

    let tx_hash = extract_tx_hash(&result);
    println!(
        "{{\"ok\":true,\"txHash\":\"{}\",\"pool\":\"{}\",\"tokenA\":\"{}\",\"tokenB\":\"{}\",\"stable\":{},\"liquidityRemoved\":{}}}",
        tx_hash, pool_addr, token_a, token_b, args.stable, liquidity_to_remove
    );

    Ok(())
}
