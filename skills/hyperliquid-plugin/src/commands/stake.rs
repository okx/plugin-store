use clap::Args;
use crate::api;
use crate::config::{info_url, exchange_url, now_ms, ARBITRUM_CHAIN_ID};
use crate::onchainos::{resolve_wallet_with_chain, onchainos_hl_sign_c_deposit, onchainos_hl_sign_token_delegate};
use crate::signing::submit_exchange_request;
use super::error_response;

#[derive(Args)]
pub struct StakeArgs {
    /// Amount of HYPE to stake (e.g. "100" or "100.5")
    #[arg(long)]
    pub amount: String,

    /// Validator address to delegate to (0x-prefixed)
    #[arg(long)]
    pub validator: String,

    /// Submit the transaction on-chain (dry-run preview by default)
    #[arg(long, default_value_t = false)]
    pub confirm: bool,

    /// Wallet address override (defaults to onchainos connected wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: StakeArgs) -> anyhow::Result<()> {
    // Parse amount → atomic units (S-4 numeric-encoding)
    let amount_raw = match api::parse_hype_amount(&args.amount) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "INVALID_ARGUMENT",
                "Provide a valid HYPE amount, e.g. --amount 100",
            ));
            return Ok(());
        }
    };
    if amount_raw == 0 {
        println!("{}", error_response(
            "Amount must be greater than 0",
            "INVALID_ARGUMENT",
            "Provide a positive HYPE amount, e.g. --amount 100",
        ));
        return Ok(());
    }

    // Resolve wallet
    let (wallet, chain_id) = match resolve_wallet_with_chain(ARBITRUM_CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "WALLET_NOT_FOUND",
                "Run onchainos wallet addresses to verify login.",
            ));
            return Ok(());
        }
    };
    let wallet = args.address.unwrap_or(wallet);

    let url = info_url();
    let nonce = now_ms();

    // S-1: balance pre-check before submitting
    let hype_bal = match api::get_spot_hype_balance(url, &wallet).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "BALANCE_QUERY_FAILED",
                "Check your connection and retry.",
            ));
            return Ok(());
        }
    };

    if hype_bal < amount_raw {
        let gap_pct = (amount_raw - hype_bal) as f64 / amount_raw as f64 * 100.0;
        if gap_pct > 1.0 {
            println!("{}", error_response(
                &format!(
                    "Insufficient HYPE balance: need {}, have {}. Add HYPE to your spot account first.",
                    api::format_hype_amount(amount_raw),
                    api::format_hype_amount(hype_bal)
                ),
                "INSUFFICIENT_HYPE_BALANCE",
                "Transfer HYPE to your spot account before staking.",
            ));
            return Ok(());
        }
        // <= 1% gap: cap down silently
        eprintln!("[skill] HYPE balance gap {:.4}%, adjusting stake amount down.", gap_pct);
    }

    let effective_amount = amount_raw.min(hype_bal);

    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "preview": true,
            "action": "stake",
            "wallet": wallet,
            "validator": args.validator,
            "amount": api::format_hype_amount(effective_amount),
            "amount_raw": effective_amount.to_string(),
            "hype_spot_balance": api::format_hype_amount(hype_bal),
            "hype_spot_balance_raw": hype_bal.to_string(),
            "steps": [
                "1. cDeposit: move HYPE from spot balance into staking balance",
                "2. tokenDelegate: delegate staking balance to the validator",
            ],
            "note": "Dry-run preview — add --confirm to sign and submit both steps.",
        }))?);
        return Ok(());
    }

    let exchange = exchange_url();

    // Step 1: cDeposit (spot → staking balance)
    let signed_deposit = match onchainos_hl_sign_c_deposit(effective_amount, nonce, &wallet, chain_id) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "EIP712_SIGN_FAILED",
                "Retry the command. If the issue persists, check onchainos status.",
            ));
            return Ok(());
        }
    };
    let deposit_result = match submit_exchange_request(exchange, signed_deposit).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "TX_SUBMIT_FAILED",
                "cDeposit step failed before any funds moved. Safe to retry the stake command.",
            ));
            return Ok(());
        }
    };
    if deposit_result["status"].as_str() == Some("err") {
        println!("{}", error_response(
            &format!("Stake step 1 (cDeposit) failed: {}", deposit_result["response"].as_str().unwrap_or("unknown error")),
            "STAKE_DEPOSIT_FAILED",
            "No funds moved. Check HYPE balance and retry the stake command.",
        ));
        return Ok(());
    }

    // Step 2: tokenDelegate (staking balance → validator), nonce + 1 to ensure ordering
    let nonce2 = nonce + 1;
    let signed_delegate = match onchainos_hl_sign_token_delegate(&args.validator, effective_amount, false, nonce2, &wallet, chain_id) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "status": "partial",
                "action": "stake",
                "wallet": wallet,
                "validator": args.validator,
                "amount": api::format_hype_amount(effective_amount),
                "amount_raw": effective_amount.to_string(),
                "step_1_cDeposit_result": deposit_result,
                "error": format!("{:#}", e),
                "error_code": "STAKE_DELEGATE_SIGN_FAILED",
                "note": "PARTIAL: cDeposit SUCCEEDED — HYPE is already in your staking balance. The tokenDelegate step failed to sign. DO NOT re-run `stake` (that would double-deposit). Run `stake` is NOT safe to retry; instead delegate the already-deposited funds manually, or contact support.",
            }))?);
            return Ok(());
        }
    };
    let delegate_result = match submit_exchange_request(exchange, signed_delegate).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "status": "partial",
                "action": "stake",
                "wallet": wallet,
                "validator": args.validator,
                "amount": api::format_hype_amount(effective_amount),
                "amount_raw": effective_amount.to_string(),
                "step_1_cDeposit_result": deposit_result,
                "error": format!("{:#}", e),
                "error_code": "STAKE_DELEGATE_SUBMIT_FAILED",
                "note": "PARTIAL: cDeposit SUCCEEDED — HYPE is already in your staking balance. The tokenDelegate submission failed. DO NOT re-run `stake` (that would double-deposit). Delegate the already-deposited funds manually, or contact support.",
            }))?);
            return Ok(());
        }
    };
    if delegate_result["status"].as_str() == Some("err") {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": false,
            "status": "partial",
            "action": "stake",
            "wallet": wallet,
            "validator": args.validator,
            "amount": api::format_hype_amount(effective_amount),
            "amount_raw": effective_amount.to_string(),
            "step_1_cDeposit_result": deposit_result,
            "step_2_tokenDelegate_result": delegate_result,
            "error": format!("tokenDelegate failed: {}", delegate_result["response"].as_str().unwrap_or("unknown error")),
            "error_code": "STAKE_DELEGATE_FAILED",
            "note": "PARTIAL: cDeposit SUCCEEDED — HYPE is already in your staking balance. The tokenDelegate step returned an error. DO NOT re-run `stake` (that would double-deposit). Verify the validator address and delegate the already-deposited funds manually, or contact support.",
        }))?);
        return Ok(());
    }

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "action": "stake",
        "wallet": wallet,
        "validator": args.validator,
        "staked_amount": api::format_hype_amount(effective_amount),
        "staked_amount_raw": effective_amount.to_string(),
        "step_1_cDeposit_result": deposit_result,
        "step_2_tokenDelegate_result": delegate_result,
        "note": "HYPE deposited to staking balance and delegated to validator. Use staking-info to see your updated stake.",
    }))?);
    Ok(())
}
