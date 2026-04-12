use clap::Args;
use crate::config::{info_url, exchange_url, now_ms, ARBITRUM_CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign_withdraw, resolve_wallet_with_chain};
use crate::api::get_clearinghouse_state;
use crate::signing::submit_exchange_request;

#[derive(Args)]
pub struct WithdrawArgs {
    /// USDC amount to withdraw (e.g. 10 for $10 USDC)
    #[arg(long)]
    pub amount: f64,

    /// Destination address on Arbitrum (defaults to your wallet address)
    #[arg(long)]
    pub destination: Option<String>,

    /// Dry run — show payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, shows a preview)
    #[arg(long)]
    pub confirm: bool,
}

/// Hyperliquid charges a fixed $1 USDC withdrawal fee on every withdrawal.
/// The fee is deducted from your balance — the recipient receives the full requested amount.
const WITHDRAWAL_FEE_USDC: f64 = 1.0;

pub async fn run(args: WithdrawArgs) -> anyhow::Result<()> {
    if args.amount <= 0.0 {
        anyhow::bail!("--amount must be positive (got {})", args.amount);
    }
    if args.amount < 2.0 {
        anyhow::bail!(
            "Minimum withdrawal is $2 USDC (got ${}).",
            args.amount
        );
    }

    let info = info_url();
    let exchange = exchange_url();
    let nonce = now_ms();

    let (wallet, sign_chain_id) = resolve_wallet_with_chain(ARBITRUM_CHAIN_ID)?;
    let destination = args.destination.clone().unwrap_or_else(|| wallet.clone());

    // Format amount as string with up to 8 decimal places, trimming trailing zeros
    let amount_str = format!("{}", args.amount);

    // Fetch withdrawable balance
    let state = get_clearinghouse_state(info, &wallet).await?;
    let withdrawable: f64 = state["withdrawable"]
        .as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);

    // Check balance covers amount + $1 fee
    let total_deducted = args.amount + WITHDRAWAL_FEE_USDC;
    if total_deducted > withdrawable {
        anyhow::bail!(
            "Insufficient balance: withdrawal ${:.2} + $1.00 fee = ${:.2} required, \
             but only ${:.2} USDC available.",
            args.amount, total_deducted, withdrawable
        );
    }

    if args.dry_run || !args.confirm {
        println!("{}", serde_json::json!({
            "ok": true,
            "preview": !args.confirm,
            "dry_run": args.dry_run,
            "action": "withdraw3",
            "wallet": wallet,
            "destination": destination,
            "amountToReceive_usd": args.amount,
            "withdrawalFee_usd": WITHDRAWAL_FEE_USDC,
            "totalDeducted_usd": total_deducted,
            "withdrawable": format!("{:.6}", withdrawable),
            "note": "A $1 USDC fee is deducted from your balance. Recipient receives the full amount. Add --confirm to execute."
        }));
        return Ok(());
    }

    println!(
        "Withdrawing {} USDC to {} (+ $1.00 fee deducted from balance)...",
        args.amount, destination
    );
    let signed = onchainos_hl_sign_withdraw(&destination, &amount_str, nonce, &wallet, sign_chain_id)?;
    let result = submit_exchange_request(exchange, signed).await?;

    if result["status"].as_str() == Some("err") {
        anyhow::bail!("Withdraw failed: {}", result["response"].as_str().unwrap_or("unknown error"));
    }

    println!("{}", serde_json::json!({
        "ok": true,
        "action": "withdraw3",
        "wallet": wallet,
        "destination": destination,
        "amountReceived_usd": args.amount,
        "feeDeducted_usd": WITHDRAWAL_FEE_USDC,
        "totalDeducted_usd": total_deducted,
        "result": result,
        "note": "USDC will arrive on Arbitrum in ~2-5 minutes. $1 fee was deducted from your Hyperliquid balance."
    }));

    Ok(())
}
