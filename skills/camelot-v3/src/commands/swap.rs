use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{
    factory, pad_address, pad_u256,
    quoter, resolve_token_address, rpc_url, swap_router, unix_now,
};
use crate::onchainos::{erc20_approve, extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{factory_pool_by_pair, get_allowance, get_decimals, quoter_exact_input_single};

#[derive(Args)]
pub struct SwapArgs {
    /// Input token (symbol or hex address)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or hex address)
    #[arg(long)]
    pub token_out: String,
    /// Amount in (raw units, e.g. 1000000 for 1 USDT)
    #[arg(long)]
    pub amount_in: u128,
    /// Slippage tolerance in percent (e.g. 0.5 = 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage: f64,
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

pub async fn run(args: SwapArgs) -> anyhow::Result<()> {
    let rpc = rpc_url(args.chain)?;
    let router = swap_router(args.chain)?;
    let quoter_addr = quoter(args.chain)?;
    let factory_addr = factory(args.chain)?;
    let token_in = resolve_token_address(&args.token_in, args.chain);
    let token_out = resolve_token_address(&args.token_out, args.chain);

    // 1. Verify pool exists
    let pool_addr = factory_pool_by_pair(&token_in, &token_out, factory_addr, &rpc).await?;
    if pool_addr == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!(
            "No pool found for {} / {} on Camelot V3 (chain {})",
            token_in,
            token_out,
            args.chain
        );
    }

    // 2. Get quote
    let amount_out = quoter_exact_input_single(
        quoter_addr,
        &token_in,
        &token_out,
        args.amount_in,
        &rpc,
    )
    .await?;

    if amount_out == 0 {
        anyhow::bail!("Quote returned 0 amountOut — pool may have no liquidity");
    }

    let slippage_factor = 1.0 - (args.slippage / 100.0);
    let amount_out_min = (amount_out as f64 * slippage_factor) as u128;
    let deadline = unix_now() + args.deadline_minutes * 60;

    // 3. Resolve recipient
    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(args.chain)?
    };

    // 4. Get decimals for display
    let dec_in = get_decimals(&token_in, &rpc).await.unwrap_or(18);
    let dec_out = get_decimals(&token_out, &rpc).await.unwrap_or(18);
    let amount_in_human = args.amount_in as f64 / 10f64.powi(dec_in as i32);
    let amount_out_human = amount_out as f64 / 10f64.powi(dec_out as i32);

    eprintln!(
        "Swap: {} → {} | amountIn={:.6} amountOut≈{:.6} amountOutMin={}",
        token_in, token_out, amount_in_human, amount_out_human, amount_out_min
    );
    eprintln!("Please confirm the swap before proceeding (auto-proceeding in non-interactive mode).");

    // 5. Build exactInputSingle calldata
    // Algebra V1 ExactInputSingleParams:
    // (address tokenIn, address tokenOut, address recipient, uint256 deadline,
    //  uint256 amountIn, uint256 amountOutMinimum, uint160 limitSqrtPrice)
    // selector: 0xbc651188
    let token_in_p = pad_address(&token_in);
    let token_out_p = pad_address(&token_out);
    let recipient_p = pad_address(&recipient);
    let deadline_p = pad_u256(deadline as u128);
    let amount_in_p = pad_u256(args.amount_in);
    let amount_out_min_p = pad_u256(amount_out_min);
    let limit_sqrt_p = pad_u256(0); // no limit

    let calldata = format!(
        "0xbc651188{}{}{}{}{}{}{}",
        token_in_p, token_out_p, recipient_p, deadline_p,
        amount_in_p, amount_out_min_p, limit_sqrt_p
    );

    // 6. Check allowance and approve if needed
    if !args.dry_run {
        let allowance = get_allowance(&token_in, &recipient, router, &rpc).await?;
        if allowance < args.amount_in {
            eprintln!("Approving {} for SwapRouter...", token_in);
            let approve_res = erc20_approve(args.chain, &token_in, router, u128::MAX, false).await?;
            if !approve_res["ok"].as_bool().unwrap_or(false) {
                anyhow::bail!("Approve failed: {}", approve_res);
            }
            eprintln!("Approve tx: {}", extract_tx_hash(&approve_res));
            sleep(Duration::from_secs(3)).await;
        }
    }

    // 7. Execute swap
    let result = wallet_contract_call(args.chain, router, &calldata, true, args.dry_run).await?;

    let tx_hash = extract_tx_hash(&result);
    let output = serde_json::json!({
        "ok": result["ok"].as_bool().unwrap_or(false),
        "dry_run": args.dry_run,
        "data": {
            "txHash": tx_hash,
            "token_in": token_in,
            "token_out": token_out,
            "amount_in": args.amount_in.to_string(),
            "amount_in_human": format!("{:.6}", amount_in_human),
            "amount_out_estimated": amount_out.to_string(),
            "amount_out_human": format!("{:.6}", amount_out_human),
            "amount_out_min": amount_out_min.to_string(),
            "calldata": calldata,
            "chain_id": args.chain
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
