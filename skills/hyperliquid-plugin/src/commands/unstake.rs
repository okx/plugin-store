use clap::Args;
use crate::api;
use crate::config::{info_url, exchange_url, now_ms, ARBITRUM_CHAIN_ID};
use crate::onchainos::{resolve_wallet_with_chain, onchainos_hl_sign_token_delegate};
use crate::signing::submit_exchange_request;
use super::error_response;

#[derive(Args)]
pub struct UnstakeArgs {
    /// Amount of HYPE to unstake (e.g. "100" or "100.5")
    #[arg(long)]
    pub amount: String,

    /// Validator address to undelegate from (0x-prefixed)
    #[arg(long)]
    pub validator: String,

    /// Submit the transaction on-chain (dry-run preview by default)
    #[arg(long, default_value_t = false)]
    pub confirm: bool,

    /// Wallet address override (defaults to onchainos connected wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: UnstakeArgs) -> anyhow::Result<()> {
    // Parse amount → atomic units
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

    // S-1: Check that the user has sufficient stake with this validator
    let delegations = match api::get_delegations(url, &wallet).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "API_ERROR",
                "Check your connection and retry.",
            ));
            return Ok(());
        }
    };

    let empty = vec![];
    let delegations_arr = delegations.as_array().unwrap_or(&empty);
    let validator_lower = args.validator.to_lowercase();
    let staked_raw: u64 = delegations_arr.iter()
        .filter(|d| d["validator"].as_str().map(|s| s.to_lowercase()) == Some(validator_lower.clone()))
        .filter_map(|d| d["amount"].as_str().and_then(|s| api::parse_hype_amount(s).ok()))
        .sum();

    if staked_raw == 0 {
        println!("{}", error_response(
            &format!("No HYPE staked with validator {}", args.validator),
            "NOT_STAKING",
            "Run staking-info to see your current delegations.",
        ));
        return Ok(());
    }

    if amount_raw > staked_raw {
        println!("{}", error_response(
            &format!(
                "Cannot unstake {}: only {} staked with this validator.",
                api::format_hype_amount(amount_raw),
                api::format_hype_amount(staked_raw)
            ),
            "INSUFFICIENT_STAKE",
            "Run staking-info to see your current delegations.",
        ));
        return Ok(());
    }

    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "preview": true,
            "action": "unstake",
            "wallet": wallet,
            "validator": args.validator,
            "amount": api::format_hype_amount(amount_raw),
            "amount_raw": amount_raw.to_string(),
            "current_stake": api::format_hype_amount(staked_raw),
            "current_stake_raw": staked_raw.to_string(),
            "note": "Dry-run preview — add --confirm to sign and submit. Unbonding period applies.",
        }))?);
        return Ok(());
    }

    // Sign + submit (user-signed tokenDelegate with isUndelegate=true)
    let signed = match onchainos_hl_sign_token_delegate(&args.validator, amount_raw, true, nonce, &wallet, chain_id) {
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

    let exchange = exchange_url();
    let result = match submit_exchange_request(exchange, signed).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "TX_SUBMIT_FAILED",
                "Retry the command. If the issue persists, check onchainos status.",
            ));
            return Ok(());
        }
    };

    if result["status"].as_str() == Some("err") {
        println!("{}", error_response(
            &format!("Unstake failed: {}", result["response"].as_str().unwrap_or("unknown error")),
            "UNSTAKE_FAILED",
            "Check validator address and staked balance, then retry.",
        ));
        return Ok(());
    }

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "action": "unstake",
        "wallet": wallet,
        "validator": args.validator,
        "amount": api::format_hype_amount(amount_raw),
        "amount_raw": amount_raw.to_string(),
        "result": result,
        "note": "Undelegation started. Unbonding period applies before HYPE is returned to your spot account. Use unbonding to track progress.",
    }))?);
    Ok(())
}
