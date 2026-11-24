use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{
    build_add_liquidity_calldata, build_approve_calldata, factory_address,
    resolve_token_address, router_address, rpc_url, unix_now,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{factory_get_pool, get_allowance, get_erc20_decimals, parse_human_amount, router_quote_add_liquidity};

const CHAIN_ID: u64 = 10;

#[derive(Args)]
pub struct AddLiquidityArgs {
    /// Token A (symbol or hex address, e.g. WETH, USDC, 0x...)
    #[arg(long)]
    pub token_a: String,
    /// Token B (symbol or hex address)
    #[arg(long)]
    pub token_b: String,
    /// Use stable pool (omit for volatile, add flag for stable)
    #[arg(long, default_value_t = false)]
    pub stable: bool,
    /// Desired amount of token A (human-readable decimal, e.g. 0.0001 for 0.0001 WETH)
    #[arg(long)]
    pub amount_a_desired: String,
    /// Desired amount of token B (human-readable decimal, 0 = auto-quote)
    #[arg(long, default_value = "0")]
    pub amount_b_desired: String,
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

pub async fn run(args: AddLiquidityArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let token_a = resolve_token_address(&args.token_a);
    let token_b = resolve_token_address(&args.token_b);
    let factory = factory_address();
    let router = router_address();

    // --- 0. Parse amounts ---
    let decimals_a = get_erc20_decimals(&token_a, rpc).await?;
    let decimals_b = get_erc20_decimals(&token_b, rpc).await?;
    let amount_a_desired = parse_human_amount(&args.amount_a_desired, decimals_a)?;
    let amount_a_min = parse_human_amount(&args.amount_a_min, decimals_a)?;
    let amount_b_min = parse_human_amount(&args.amount_b_min, decimals_b)?;

    // --- 1. Verify pool exists ---
    let pool_addr = factory_get_pool(&token_a, &token_b, args.stable, factory, rpc).await?;
    if pool_addr == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!(
            "Pool does not exist for {}/{} stable={}. Deploy the pool first.",
            token_a, token_b, args.stable
        );
    }
    println!("Pool verified: {}", pool_addr);

    // --- 2. Auto-quote amount_b if not provided ---
    let amount_b_desired_raw = parse_human_amount(&args.amount_b_desired, decimals_b)?;
    let amount_b_desired = if amount_b_desired_raw == 0 {
        let (_, quoted_b, _) = router_quote_add_liquidity(
            router, &token_a, &token_b, args.stable, factory,
            amount_a_desired, u128::MAX / 2,
            rpc
        ).await.unwrap_or((0, 0, 0));
        println!("Auto-quoted amountBDesired: {}", quoted_b);
        quoted_b
    } else {
        amount_b_desired_raw
    };

    // Preview gate — emit structured preview and exit before any wallet/on-chain calls
    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "preview": true,
            "message": "Add --confirm to broadcast",
            "data": {
                "action": "add-liquidity",
                "token_a": token_a,
                "token_b": token_b,
                "stable": args.stable,
                "amount_a_desired": amount_a_desired,
                "amount_b_desired": amount_b_desired,
                "amount_a_min": amount_a_min,
                "amount_b_min": amount_b_min
            }
        }))?);
        return Ok(());
    }

    // --- 3. Resolve recipient ---
    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    // --- 4. Approve token A if needed ---
    if !args.dry_run {
        let allowance_a = get_allowance(&token_a, &recipient, router, rpc).await?;
        if allowance_a < amount_a_desired {
            println!("Approving tokenA ({}) for Router...", token_a);
            let approve_data = build_approve_calldata(router, amount_a_desired);
            let res = wallet_contract_call(CHAIN_ID, &token_a, &approve_data, args.confirm, false).await?;
            println!("Approve tokenA tx: {}", extract_tx_hash(&res));
            sleep(Duration::from_secs(5)).await;
        }

        // --- 5. Approve token B if needed ---
        let allowance_b = get_allowance(&token_b, &recipient, router, rpc).await?;
        if allowance_b < amount_b_desired {
            println!("Approving tokenB ({}) for Router...", token_b);
            let approve_data = build_approve_calldata(router, amount_b_desired);
            let res = wallet_contract_call(CHAIN_ID, &token_b, &approve_data, args.confirm, false).await?;
            println!("Approve tokenB tx: {}", extract_tx_hash(&res));
            sleep(Duration::from_secs(5)).await;
        }
    }

    // --- 6. Build addLiquidity calldata ---
    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = build_add_liquidity_calldata(
        &token_a,
        &token_b,
        args.stable,
        amount_a_desired,
        amount_b_desired,
        amount_a_min,
        amount_b_min,
        &recipient,
        deadline,
    );

    let result = wallet_contract_call(CHAIN_ID, router, &calldata, args.confirm, args.dry_run).await?;

    let tx_hash = extract_tx_hash(&result);
    println!(
        "{{\"ok\":true,\"txHash\":\"{}\",\"tokenA\":\"{}\",\"tokenB\":\"{}\",\"stable\":{},\"amountADesired\":{},\"amountBDesired\":{}}}",
        tx_hash, token_a, token_b, args.stable, amount_a_desired, amount_b_desired
    );

    Ok(())
}
