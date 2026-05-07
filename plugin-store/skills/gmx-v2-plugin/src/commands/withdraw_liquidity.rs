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
    let execution_fee_eth = execution_fee as f64 / 1e18;
    let gm_fmt = crate::api::format_token_amount(args.gm_amount, 18);

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

    // Pre-flight: ETH balance for execution fee + gas
    let eth_balance = crate::rpc::get_eth_balance(&wallet, cfg.rpc_url).await;
    let gas_margin: u128 = 200_000_000_000_000; // 0.0002 ETH
    let eth_required = execution_fee.saturating_add(gas_margin);
    if eth_balance < eth_required {
        println!("{}", serde_json::to_string_pretty(&json!({
            "ok": false,
            "error": "INSUFFICIENT_ETH_FOR_EXECUTION",
            "reason": "Wallet does not have enough ETH to cover execution fee + gas.",
            "eth_balance": format!("{:.8}", eth_balance as f64 / 1e18),
            "execution_fee_eth": format!("{:.8}", execution_fee as f64 / 1e18),
            "gas_buffer_eth": format!("{:.8}", gas_margin as f64 / 1e18),
            "eth_required": format!("{:.8}", eth_required as f64 / 1e18),
            "suggestion": format!("Top up wallet {} with at least {:.6} ETH.", wallet, (eth_required.saturating_sub(eth_balance)) as f64 / 1e18)
        }))?);
        return Ok(());
    }

    // Pre-flight: GM token balance
    let gm_bal = crate::rpc::check_erc20_balance(cfg.rpc_url, &args.market_token, &wallet).await.unwrap_or(u128::MAX);
    if gm_bal < args.gm_amount {
        println!("{}", serde_json::to_string_pretty(&json!({
            "ok": false,
            "error": "INSUFFICIENT_GM_TOKEN_BALANCE",
            "reason": "Wallet GM token balance is less than --gm-amount.",
            "token": args.market_token,
            "wallet_balance": gm_bal.to_string(),
            "wallet_balance_formatted": crate::api::format_token_amount(gm_bal, 18),
            "required_amount": args.gm_amount.to_string(),
            "required_formatted": gm_fmt.clone(),
            "suggestion": format!("Reduce --gm-amount to at most {} ({} GM tokens).", gm_bal, crate::api::format_token_amount(gm_bal, 18))
        }))?);
        return Ok(());
    }

    eprintln!("=== Withdraw Liquidity Preview ===");
    eprintln!("Market token (GM): {}", args.market_token);
    eprintln!("GM amount to burn: {}", gm_fmt);
    eprintln!("Min long token: {}", args.min_long_amount);
    eprintln!("Min short token: {}", args.min_short_amount);
    if args.min_long_amount == 0 && args.min_short_amount == 0 {
        eprintln!("⚠ Both min amounts are 0 — no slippage protection on tokens received.");
    }
    eprintln!("Execution fee: {:.6} ETH", execution_fee_eth);
    eprintln!("⚠ GMX V2 keeper model: tokens returned 1-30s after tx lands.");
    if !confirm { eprintln!("Add --confirm to broadcast."); }

    if !confirm && !dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "status": "preview",
                "message": "Add --confirm to broadcast this transaction",
                "chain": chain,
                "marketToken": args.market_token,
                "gmAmountBurned": gm_fmt,
                "minLongAmount": args.min_long_amount.to_string(),
                "minShortAmount": args.min_short_amount.to_string(),
                "executionFee_eth": format!("{:.6}", execution_fee_eth),
                "calldata": calldata
            }))?
        );
        return Ok(());
    }

    // Approve GM token to Router only when about to execute
    if confirm && !dry_run {
        let allowance = crate::onchainos::check_allowance(
            cfg.rpc_url, &args.market_token, &wallet, cfg.router,
        ).await.unwrap_or(0);
        if allowance < args.gm_amount {
            eprintln!("Approving GM token to router...");
            let r = crate::onchainos::erc20_approve(
                cfg.chain_id, &args.market_token, cfg.router, args.gm_amount, Some(&wallet), false, confirm,
            ).await?;
            let approve_hash = crate::onchainos::extract_tx_hash(&r);
            eprintln!("Approval tx: {}", approve_hash);
            crate::onchainos::wait_for_tx(cfg.chain_id, approve_hash, &wallet, 60)?;
        }
    }

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
            "gmAmountBurned": gm_fmt,
            "minLongAmount": args.min_long_amount.to_string(),
            "minShortAmount": args.min_short_amount.to_string(),
            "executionFee_eth": format!("{:.6}", execution_fee_eth),
            "note": "Underlying tokens returned within 1-30s after keeper executes",
            "calldata": if dry_run { Some(calldata.as_str()) } else { None }
        }))?
    );
    Ok(())
}
