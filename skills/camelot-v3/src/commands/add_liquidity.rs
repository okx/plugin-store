use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{
    encode_tick, nfpm, pad_address, pad_u256, resolve_token_address, rpc_url, unix_now,
};
use crate::onchainos::{erc20_approve, extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{factory_pool_by_pair, get_allowance};
use crate::config::factory;

#[derive(Args)]
pub struct AddLiquidityArgs {
    /// Token0 (symbol or hex address)
    #[arg(long)]
    pub token0: String,
    /// Token1 (symbol or hex address)
    #[arg(long)]
    pub token1: String,
    /// Amount of token0 desired (raw units)
    #[arg(long, default_value = "0")]
    pub amount0: u128,
    /// Amount of token1 desired (raw units)
    #[arg(long, default_value = "0")]
    pub amount1: u128,
    /// Lower tick (default: full range)
    #[arg(long, default_value = "-887200", allow_hyphen_values = true)]
    pub tick_lower: i32,
    /// Upper tick (default: full range)
    #[arg(long, default_value = "887200", allow_hyphen_values = true)]
    pub tick_upper: i32,
    /// Minimum amount0 acceptable (slippage protection, 0 = no min)
    #[arg(long, default_value = "0")]
    pub amount0_min: u128,
    /// Minimum amount1 acceptable (slippage protection, 0 = no min)
    #[arg(long, default_value = "0")]
    pub amount1_min: u128,
    /// Transaction deadline in minutes from now
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Chain ID (default: 42161 Arbitrum)
    #[arg(long, default_value = "42161")]
    pub chain: u64,
    /// Dry run — build calldata but do not broadcast
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: AddLiquidityArgs) -> anyhow::Result<()> {
    let rpc = rpc_url(args.chain)?;
    let nfpm_addr = nfpm(args.chain)?;
    let factory_addr = factory(args.chain)?;
    let token0 = resolve_token_address(&args.token0, args.chain);
    let token1 = resolve_token_address(&args.token1, args.chain);

    // Verify pool exists
    let pool_addr = factory_pool_by_pair(&token0, &token1, factory_addr, &rpc).await?;
    if pool_addr == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!(
            "No pool found for {} / {} on Camelot V3 (chain {})",
            token0,
            token1,
            args.chain
        );
    }

    // Resolve recipient
    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(args.chain)?
    };

    let deadline = unix_now() + args.deadline_minutes * 60;

    // Build NFPM.mint calldata
    // MintParams: (address token0, address token1, int24 tickLower, int24 tickUpper,
    //              uint256 amount0Desired, uint256 amount1Desired,
    //              uint256 amount0Min, uint256 amount1Min,
    //              address recipient, uint256 deadline)
    // Selector: 0xa232240b (verified)
    let calldata = format!(
        "0xa232240b{}{}{}{}{}{}{}{}{}{}",
        pad_address(&token0),
        pad_address(&token1),
        encode_tick(args.tick_lower),
        encode_tick(args.tick_upper),
        pad_u256(args.amount0),
        pad_u256(args.amount1),
        pad_u256(args.amount0_min),
        pad_u256(args.amount1_min),
        pad_address(&recipient),
        pad_u256(deadline as u128)
    );

    eprintln!(
        "Add liquidity: {}/{} tick=[{},{}] amount0={} amount1={}",
        token0, token1, args.tick_lower, args.tick_upper, args.amount0, args.amount1
    );
    eprintln!("Ask user to confirm before proceeding with add-liquidity.");

    // Approve tokens if needed
    if !args.dry_run {
        if args.amount0 > 0 {
            let allowance0 = get_allowance(&token0, &recipient, nfpm_addr, &rpc).await?;
            if allowance0 < args.amount0 {
                eprintln!("Approving token0 ({}) for NFPM...", token0);
                let res = erc20_approve(args.chain, &token0, nfpm_addr, u128::MAX, false).await?;
                eprintln!("token0 approve tx: {}", extract_tx_hash(&res));
                sleep(Duration::from_secs(5)).await;
            }
        }
        if args.amount1 > 0 {
            let allowance1 = get_allowance(&token1, &recipient, nfpm_addr, &rpc).await?;
            if allowance1 < args.amount1 {
                eprintln!("Approving token1 ({}) for NFPM...", token1);
                let res = erc20_approve(args.chain, &token1, nfpm_addr, u128::MAX, false).await?;
                eprintln!("token1 approve tx: {}", extract_tx_hash(&res));
                sleep(Duration::from_secs(5)).await;
            }
        }
    }

    // Execute mint
    let result = wallet_contract_call(args.chain, nfpm_addr, &calldata, true, args.dry_run).await?;
    let tx_hash = extract_tx_hash(&result);

    let output = serde_json::json!({
        "ok": result["ok"].as_bool().unwrap_or(false),
        "dry_run": args.dry_run,
        "data": {
            "txHash": tx_hash,
            "token0": token0,
            "token1": token1,
            "tick_lower": args.tick_lower,
            "tick_upper": args.tick_upper,
            "amount0_desired": args.amount0.to_string(),
            "amount1_desired": args.amount1.to_string(),
            "calldata": calldata,
            "chain_id": args.chain
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
