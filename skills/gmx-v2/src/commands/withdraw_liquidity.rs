use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct WithdrawLiquidityArgs {
    /// Market token address (GM token) to withdraw
    #[arg(long)]
    pub market_token: String,

    /// GM token amount to burn (in wei, 18 decimals)
    #[arg(long)]
    pub gm_amount: u128,

    /// Minimum long token amount to receive (slippage protection, 0 = accept any)
    #[arg(long, default_value_t = 0)]
    pub min_long_amount: u128,

    /// Minimum short token amount to receive (slippage protection, 0 = accept any)
    #[arg(long, default_value_t = 0)]
    pub min_short_amount: u128,

    /// Wallet address (defaults to logged-in wallet)
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(chain: &str, dry_run: bool, confirm: bool, args: WithdrawLiquidityArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;

    let wallet = args.from.clone().unwrap_or_else(|| {
        crate::onchainos::resolve_wallet(cfg.chain_id).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot determine wallet address. Pass --from or ensure onchainos is logged in.");
    }

    let execution_fee = cfg.execution_fee_wei;

    // Approve GM token to Router if needed
    if !dry_run {
        let allowance = crate::onchainos::check_allowance(
            cfg.rpc_url, &args.market_token, &wallet, cfg.router,
        ).await.unwrap_or(0);
        if allowance < args.gm_amount {
            eprintln!("WARNING: Approving {} GM token to {} -- approving exact amount only. Use --dry-run to preview.", args.gm_amount, cfg.router);
            let r = crate::onchainos::erc20_approve(
                cfg.chain_id, &args.market_token, cfg.router, args.gm_amount, Some(&wallet), false, confirm,
            ).await?;
            let approve_hash = crate::onchainos::extract_tx_hash(&r);
            eprintln!("Approval tx: {}", approve_hash);
            crate::onchainos::wait_for_tx(cfg.chain_id, approve_hash, &wallet, 60)?;
        }
    }

    // Build multicall: [sendWnt, sendTokens(gmToken), createWithdrawal]
    let send_wnt = crate::abi::encode_send_wnt(cfg.withdrawal_vault, execution_fee);
    let send_gm = crate::abi::encode_send_tokens(
        &args.market_token,
        cfg.withdrawal_vault,
        args.gm_amount,
    );
    let create_withdrawal = crate::abi::encode_create_withdrawal(
        &wallet,
        &args.market_token,
        args.min_long_amount,
        args.min_short_amount,
        execution_fee,
    );

    let multicall_hex = crate::abi::encode_multicall(&[send_wnt, send_gm, create_withdrawal]);
    let calldata = format!("0x{}", multicall_hex);

    eprintln!("=== Withdraw Liquidity Preview ===");
    eprintln!("Market token (GM): {}", args.market_token);
    eprintln!("GM amount to burn: {}", args.gm_amount);
    eprintln!("Min long token: {}", args.min_long_amount);
    eprintln!("Min short token: {}", args.min_short_amount);
    eprintln!("Execution fee: {} wei", execution_fee);
    eprintln!("⚠ GMX V2 keeper model: tokens returned 1-30s after tx lands.");
    eprintln!("Ask user to confirm before proceeding.");

    let result = crate::onchainos::wallet_contract_call_with_gas(
        cfg.chain_id,
        cfg.exchange_router,
        &calldata,
        Some(&wallet),
        Some(execution_fee),
        dry_run,
        confirm,
        Some(800_000),
    ).await?;

    let tx_hash = crate::onchainos::extract_tx_hash(&result);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "dry_run": dry_run,
            "chain": chain,
            "txHash": tx_hash,
            "marketToken": args.market_token,
            "gmAmountBurned": args.gm_amount.to_string(),
            "minLongAmount": args.min_long_amount.to_string(),
            "minShortAmount": args.min_short_amount.to_string(),
            "executionFeeWei": execution_fee,
            "note": "Underlying tokens returned within 1-30s after keeper executes",
            "calldata": if dry_run { Some(calldata.as_str()) } else { None }
        }))?
    );
    Ok(())
}
