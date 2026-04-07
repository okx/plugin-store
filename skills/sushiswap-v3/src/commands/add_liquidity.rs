use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{
    build_approve_calldata, encode_tick, factory_address, nfpm_address,
    pad_address, pad_u256, resolve_token_address, rpc_url, unix_now,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{factory_get_pool, get_allowance};

#[derive(Args)]
pub struct AddLiquidityArgs {
    /// Token 0 (symbol or hex address)
    #[arg(long)]
    pub token0: String,
    /// Token 1 (symbol or hex address)
    #[arg(long)]
    pub token1: String,
    /// Fee tier (100/500/3000/10000)
    #[arg(long)]
    pub fee: u32,
    /// Lower tick of the position range (can be negative)
    #[arg(long, allow_hyphen_values = true)]
    pub tick_lower: i32,
    /// Upper tick of the position range (can be negative)
    #[arg(long, allow_hyphen_values = true)]
    pub tick_upper: i32,
    /// Desired amount of token0 (smallest unit)
    #[arg(long)]
    pub amount0_desired: u128,
    /// Desired amount of token1 (smallest unit)
    #[arg(long)]
    pub amount1_desired: u128,
    /// Minimum acceptable amount0 (0 for no minimum)
    #[arg(long, default_value = "0")]
    pub amount0_min: u128,
    /// Minimum acceptable amount1 (0 for no minimum)
    #[arg(long, default_value = "0")]
    pub amount1_min: u128,
    /// Transaction deadline in minutes from now
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Chain ID (default: 8453 = Base)
    #[arg(long, default_value = "8453")]
    pub chain: u64,
    /// Dry run — build calldata without broadcasting
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: AddLiquidityArgs) -> anyhow::Result<()> {
    let rpc = rpc_url(args.chain)?;
    let token0 = resolve_token_address(&args.token0, args.chain);
    let token1 = resolve_token_address(&args.token1, args.chain);
    let factory = factory_address(args.chain);
    let nfpm = nfpm_address(args.chain);

    // --- 1. Verify pool exists ---
    let pool_addr = factory_get_pool(&token0, &token1, args.fee, factory, &rpc).await?;
    if pool_addr == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!(
            "Pool does not exist for {}/{} fee={}. Deploy the pool first.",
            token0, token1, args.fee
        );
    }
    println!("Pool verified: {}", pool_addr);

    // --- 2. Resolve recipient ---
    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(args.chain)?
    };

    println!(
        "Adding liquidity: {}/{} fee={} tickLower={} tickUpper={} amount0={} amount1={}",
        token0, token1, args.fee, args.tick_lower, args.tick_upper,
        args.amount0_desired, args.amount1_desired
    );
    println!("Please confirm the add-liquidity parameters above before proceeding. (Proceeding automatically in non-interactive mode)");

    // --- 3. Approve token0 for NonfungiblePositionManager if needed ---
    if !args.dry_run {
        let allowance0 = get_allowance(&token0, &recipient, nfpm, &rpc).await?;
        if allowance0 < args.amount0_desired {
            println!("Approving token0 ({}) for NonfungiblePositionManager...", token0);
            let approve_data = build_approve_calldata(nfpm, u128::MAX);
            let res =
                wallet_contract_call(args.chain, &token0, &approve_data, None, None, true, false).await?;
            println!("Approve token0 tx: {}", extract_tx_hash(&res));
            sleep(Duration::from_secs(5)).await;
        }

        // --- 4. Approve token1 if needed ---
        let allowance1 = get_allowance(&token1, &recipient, nfpm, &rpc).await?;
        if allowance1 < args.amount1_desired {
            println!("Approving token1 ({}) for NonfungiblePositionManager...", token1);
            let approve_data = build_approve_calldata(nfpm, u128::MAX);
            let res =
                wallet_contract_call(args.chain, &token1, &approve_data, None, None, true, false).await?;
            println!("Approve token1 tx: {}", extract_tx_hash(&res));
            sleep(Duration::from_secs(5)).await;
        }
    }

    // --- 5. Build mint calldata ---
    // mint((address token0, address token1, uint24 fee, int24 tickLower, int24 tickUpper,
    //   uint256 amount0Desired, uint256 amount1Desired, uint256 amount0Min, uint256 amount1Min,
    //   address recipient, uint256 deadline))
    // Selector: 0x88316456
    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = format!(
        "0x88316456{}{}{}{}{}{}{}{}{}{}{}",
        pad_address(&token0),
        pad_address(&token1),
        pad_u256(args.fee as u128),
        encode_tick(args.tick_lower),
        encode_tick(args.tick_upper),
        pad_u256(args.amount0_desired),
        pad_u256(args.amount1_desired),
        pad_u256(args.amount0_min),
        pad_u256(args.amount1_min),
        pad_address(&recipient),
        pad_u256(deadline as u128),
    );

    let result =
        wallet_contract_call(args.chain, nfpm, &calldata, None, None, true, args.dry_run).await?;

    let tx_hash = extract_tx_hash(&result);
    println!(
        "{{\"ok\":true,\"txHash\":\"{}\",\"token0\":\"{}\",\"token1\":\"{}\",\"fee\":{},\"tickLower\":{},\"tickUpper\":{}}}",
        tx_hash, token0, token1, args.fee, args.tick_lower, args.tick_upper
    );

    Ok(())
}
