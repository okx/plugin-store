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
        println!("{}", super::error_response(
            &format!("--amount must be positive (got {})", args.amount),
            "INVALID_ARGUMENT",
            "Provide a positive USDC amount with --amount."
        ));
        return Ok(());
    }
    if args.amount < 2.0 {
        println!("{}", super::error_response(
            &format!("Minimum withdrawal is $2 USDC (got ${}).", args.amount),
            "INVALID_ARGUMENT",
            "Provide an amount of at least $2 USDC."
        ));
        return Ok(());
    }

    let info = info_url();
    let exchange = exchange_url();
    let nonce = now_ms();

    let (wallet, sign_chain_id) = match resolve_wallet_with_chain(ARBITRUM_CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "WALLET_NOT_FOUND", "Run onchainos wallet addresses to verify login."));
            return Ok(());
        }
    };
    let destination = args.destination.clone().unwrap_or_else(|| wallet.clone());

    // Format amount as string with up to 8 decimal places, trimming trailing zeros
    let amount_str = format!("{}", args.amount);

    // Fetch withdrawable balance
    let state = match get_clearinghouse_state(info, &wallet).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };
    let withdrawable: f64 = state["withdrawable"]
        .as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);

    // Check balance covers amount + $1 fee
    let total_deducted = args.amount + WITHDRAWAL_FEE_USDC;
    if total_deducted > withdrawable {
        println!("{}", super::error_response(
            &format!(
                "Insufficient balance: withdrawal ${:.2} + $1.00 fee = ${:.2} required, \
                 but only ${:.2} USDC available.",
                args.amount, total_deducted, withdrawable
            ),
            "INSUFFICIENT_BALANCE",
            "Ensure your Hyperliquid perp balance covers the withdrawal amount plus the $1 fee."
        ));
        return Ok(());
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
    let signed = match onchainos_hl_sign_withdraw(&destination, &amount_str, nonce, &wallet, sign_chain_id) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "SIGNING_FAILED", "Retry the command. If the issue persists, check onchainos status."));
            return Ok(());
        }
    };
    let result = match submit_exchange_request(exchange, signed).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "TX_SUBMIT_FAILED", "Retry the command. If the issue persists, check onchainos status."));
            return Ok(());
        }
    };

    if result["status"].as_str() == Some("err") {
        println!("{}", super::error_response(
            &format!("Withdraw failed: {}", result["response"].as_str().unwrap_or("unknown error")),
            "TX_SUBMIT_FAILED",
            "Retry the command. If the issue persists, check onchainos status."
        ));
        return Ok(());
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
