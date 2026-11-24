use clap::Args;
use crate::api::{get_clearinghouse_state, get_spot_clearinghouse_state};
use crate::config::{info_url, exchange_url, now_ms, CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign_usd_class_transfer, resolve_wallet_with_chain};
use crate::signing::{build_spot_transfer_action, submit_exchange_request};

#[derive(Args)]
pub struct TransferArgs {
    /// Amount of USDC to transfer
    #[arg(long)]
    pub amount: f64,

    /// Direction: perp-to-spot or spot-to-perp
    #[arg(long, value_parser = ["perp-to-spot", "spot-to-perp"])]
    pub direction: String,

    /// Override the HL account address used for balance queries
    #[arg(long)]
    pub account: Option<String>,

    /// Dry run — show payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, shows a preview)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: TransferArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();

    if args.amount <= 0.0 {
        println!("{}", super::error_response(
            &format!("--amount must be positive (got {})", args.amount),
            "INVALID_ARGUMENT",
            "Provide a positive USDC amount with --amount."
        ));
        return Ok(());
    }

    let to_perp = args.direction == "spot-to-perp";
    let nonce = now_ms();

    let (default_wallet, sign_chain_id) = match resolve_wallet_with_chain(CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "WALLET_NOT_FOUND", "Run onchainos wallet addresses to verify login."));
            return Ok(());
        }
    };
    let wallet = args.account.clone().unwrap_or(default_wallet);

    let (from_label, to_label) = if to_perp { ("spot", "perp") } else { ("perp", "spot") };

    let (perp_state, spot_state) = match tokio::try_join!(
        get_clearinghouse_state(info, &wallet),
        get_spot_clearinghouse_state(info, &wallet),
    ) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };

    let perp_withdrawable: f64 = perp_state["withdrawable"]
        .as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);

    let spot_usdc: f64 = spot_state["balances"]
        .as_array().unwrap_or(&vec![])
        .iter()
        .find(|b| b["token"].as_u64() == Some(0))
        .and_then(|b| b["total"].as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    if !to_perp && args.amount > perp_withdrawable {
        println!("{}", super::error_response(
            &format!("Insufficient perp balance: requested {:.6} USDC, withdrawable {:.6} USDC", args.amount, perp_withdrawable),
            "INSUFFICIENT_BALANCE",
            "Ensure you have enough USDC in your perp account before transferring."
        ));
        return Ok(());
    }
    if to_perp && args.amount > spot_usdc {
        println!("{}", super::error_response(
            &format!("Insufficient spot USDC balance: requested {:.6} USDC, available {:.6} USDC", args.amount, spot_usdc),
            "INSUFFICIENT_BALANCE",
            "Ensure you have enough USDC in your spot account before transferring."
        ));
        return Ok(());
    }

    let action = build_spot_transfer_action(args.amount, to_perp, nonce);

    if args.dry_run || !args.confirm {
        println!("{}", serde_json::json!({
            "ok": true,
            "preview": !args.confirm,
            "dry_run": args.dry_run,
            "action": "usdClassTransfer",
            "from": from_label,
            "to": to_label,
            "amount_usd": args.amount,
            "balanceBefore": {
                "perp_withdrawable": format!("{:.6}", perp_withdrawable),
                "spot_usdc": format!("{:.6}", spot_usdc)
            },
            "nonce": nonce,
            "note": if args.confirm { "" } else { "Add --confirm to execute" }
        }));
        return Ok(());
    }

    let signed = match onchainos_hl_sign_usd_class_transfer(
        &action, nonce, &wallet, sign_chain_id, true, false
    ) {
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
            &format!("Transfer failed: {}", result["response"].as_str().unwrap_or("unknown error")),
            "TX_SUBMIT_FAILED",
            "Retry the command. If the issue persists, check onchainos status."
        ));
        return Ok(());
    }

    println!("{}", serde_json::json!({
        "ok": true,
        "action": "usdClassTransfer",
        "from": from_label,
        "to": to_label,
        "amount_usd": args.amount,
        "result": result
    }));

    Ok(())
}
