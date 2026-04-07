use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{
    build_approve_calldata, factory_address, pad_address, pad_u256,
    quoter_v2_address, resolve_token_address, rpc_url, swap_router, unix_now,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{factory_get_pool, get_allowance, quoter_exact_input_single};

const FEE_TIERS: &[u32] = &[100, 500, 3000, 10000];

#[derive(Args)]
pub struct SwapArgs {
    /// Input token (symbol or hex address, e.g. WETH or 0x4200...)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or hex address)
    #[arg(long)]
    pub token_out: String,
    /// Amount to swap in smallest token unit (e.g. wei)
    #[arg(long)]
    pub amount_in: u128,
    /// Slippage tolerance in percent (e.g. 0.5 = 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage: f64,
    /// Fee tier (100/500/3000/10000). Omit to auto-select best.
    #[arg(long)]
    pub fee: Option<u32>,
    /// Transaction deadline in minutes from now
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Chain ID (default: 8453 = Base)
    #[arg(long, default_value = "8453")]
    pub chain: u64,
    /// Dry run — build and print calldata without broadcasting
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: SwapArgs) -> anyhow::Result<()> {
    let rpc = rpc_url(args.chain)?;
    let token_in = resolve_token_address(&args.token_in, args.chain);
    let token_out = resolve_token_address(&args.token_out, args.chain);
    let factory = factory_address(args.chain);
    let quoter = quoter_v2_address(args.chain);
    let router = swap_router(args.chain);

    // --- 1. Find best fee tier via QuoterV2 ---
    let fees_to_check: Vec<u32> = if let Some(f) = args.fee {
        vec![f]
    } else {
        FEE_TIERS.to_vec()
    };

    let mut best_amount_out: u128 = 0;
    let mut best_fee: u32 = 0;

    for fee in &fees_to_check {
        let pool_addr = factory_get_pool(&token_in, &token_out, *fee, factory, &rpc).await?;
        if pool_addr == "0x0000000000000000000000000000000000000000" {
            continue;
        }
        match quoter_exact_input_single(quoter, &token_in, &token_out, args.amount_in, *fee, &rpc).await {
            Ok(amount_out) if amount_out > best_amount_out => {
                best_amount_out = amount_out;
                best_fee = *fee;
            }
            _ => {}
        }
    }

    if best_amount_out == 0 {
        anyhow::bail!("No valid pool or quote found. Check token addresses and fee tiers.");
    }

    let slippage_factor = 1.0 - (args.slippage / 100.0);
    let amount_out_minimum = (best_amount_out as f64 * slippage_factor) as u128;

    println!(
        "Quote: tokenIn={} tokenOut={} amountIn={} fee={} amountOut={} amountOutMin={}",
        token_in, token_out, args.amount_in, best_fee, best_amount_out, amount_out_minimum
    );
    println!("Please confirm the swap above before proceeding. (Proceeding automatically in non-interactive mode)");

    // --- 2. Resolve recipient ---
    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(args.chain)?
    };

    // --- 3. Check ERC-20 allowance and approve if needed ---
    if !args.dry_run {
        let allowance = get_allowance(&token_in, &recipient, router, &rpc).await?;
        if allowance < args.amount_in {
            println!("Approving {} for SwapRouter...", token_in);
            let approve_data = build_approve_calldata(router, u128::MAX);
            let approve_result =
                wallet_contract_call(args.chain, &token_in, &approve_data, None, None, true, false).await?;
            println!("Approve tx: {}", extract_tx_hash(&approve_result));
            // Wait for approve nonce to clear before swap
            sleep(Duration::from_secs(3)).await;
        }
    }

    // --- 4. Build exactInputSingle calldata ---
    // SushiSwap V3 SwapRouter uses same interface as Uniswap V3:
    // exactInputSingle((address tokenIn, address tokenOut, uint24 fee, address recipient,
    //   uint256 deadline, uint256 amountIn, uint256 amountOutMinimum, uint160 sqrtPriceLimitX96))
    // Selector: 0x414bf389
    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = format!(
        "0x414bf389{}{}{}{}{}{}{}{}",
        pad_address(&token_in),
        pad_address(&token_out),
        pad_u256(best_fee as u128),
        pad_address(&recipient),
        pad_u256(deadline as u128),
        pad_u256(args.amount_in),
        pad_u256(amount_out_minimum),
        pad_u256(0), // sqrtPriceLimitX96 = 0 (no price limit)
    );

    let result =
        wallet_contract_call(args.chain, router, &calldata, None, None, true, args.dry_run).await?;

    let tx_hash = extract_tx_hash(&result);
    println!(
        "{{\"ok\":true,\"txHash\":\"{}\",\"tokenIn\":\"{}\",\"tokenOut\":\"{}\",\"amountIn\":{},\"fee\":{},\"amountOutMin\":{}}}",
        tx_hash, token_in, token_out, args.amount_in, best_fee, amount_out_minimum
    );

    Ok(())
}
