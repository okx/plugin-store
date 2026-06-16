use clap::Args;
use crate::api;
use crate::config::{info_url, exchange_url, now_ms, ARBITRUM_CHAIN_ID};
use crate::onchainos::{resolve_wallet_with_chain, onchainos_hl_sign};
use crate::signing::{build_redelegate_actions, submit_exchange_request};
use super::error_response;

#[derive(Args)]
pub struct RedelegateArgs {
    /// Amount of HYPE to redelegate (e.g. "100" or "100.5")
    #[arg(long)]
    pub amount: String,

    /// Source validator address to undelegate from (0x-prefixed)
    #[arg(long)]
    pub from_validator: String,

    /// Destination validator address to delegate to (0x-prefixed)
    #[arg(long)]
    pub to_validator: String,

    /// Submit the transaction on-chain (dry-run preview by default)
    #[arg(long, default_value_t = false)]
    pub confirm: bool,

    /// Wallet address override (defaults to onchainos connected wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: RedelegateArgs) -> anyhow::Result<()> {
    // Guard: from and to must differ
    if args.from_validator.to_lowercase() == args.to_validator.to_lowercase() {
        println!("{}", error_response(
            "Source and destination validators must be different",
            "SAME_VALIDATOR",
            "Provide different --from-validator and --to-validator addresses.",
        ));
        return Ok(());
    }

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

    // S-1: Check that user has sufficient stake with from_validator
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
    let from_lower = args.from_validator.to_lowercase();
    let staked_raw: u64 = delegations_arr.iter()
        .filter(|d| d["validator"].as_str().map(|s| s.to_lowercase()) == Some(from_lower.clone()))
        .filter_map(|d| d["amount"].as_str().and_then(|s| s.parse::<u64>().ok()))
        .sum();

    if staked_raw == 0 {
        println!("{}", error_response(
            &format!("No HYPE staked with from-validator {}", args.from_validator),
            "NOT_STAKING",
            "Run staking-info to see your current delegations.",
        ));
        return Ok(());
    }

    if amount_raw > staked_raw {
        println!("{}", error_response(
            &format!(
                "Cannot redelegate {}: only {} staked with from-validator.",
                api::format_hype_amount(amount_raw),
                api::format_hype_amount(staked_raw)
            ),
            "INSUFFICIENT_STAKE",
            "Run staking-info to see your current delegations.",
        ));
        return Ok(());
    }

    let (undelegate_action, delegate_action) = build_redelegate_actions(
        &args.from_validator,
        &args.to_validator,
        amount_raw,
        nonce,
    );

    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "preview": true,
            "action": "redelegate",
            "wallet": wallet,
            "from_validator": args.from_validator,
            "to_validator": args.to_validator,
            "amount": api::format_hype_amount(amount_raw),
            "amount_raw": amount_raw.to_string(),
            "step_1_undelegate": undelegate_action,
            "step_2_delegate": delegate_action,
            "note": "Dry-run preview — add --confirm to sign and submit both steps.",
        }))?);
        return Ok(());
    }

    let exchange = exchange_url();

    // Step 1: Undelegate from source
    let signed1 = match onchainos_hl_sign(&undelegate_action, nonce, &wallet, chain_id, true, false) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "EIP712_SIGN_FAILED",
                "Retry the command.",
            ));
            return Ok(());
        }
    };
    let result1 = match submit_exchange_request(exchange, signed1).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "TX_SUBMIT_FAILED",
                "Undelegate step failed. Check status with staking-info before retrying.",
            ));
            return Ok(());
        }
    };
    if result1["status"].as_str() == Some("err") {
        println!("{}", error_response(
            &format!("Redelegate step 1 (undelegate) failed: {}", result1["response"].as_str().unwrap_or("unknown")),
            "REDELEGATE_UNDELEGATE_FAILED",
            "Check your stake with staking-info and retry.",
        ));
        return Ok(());
    }

    // Step 2: Delegate to destination (nonce + 1)
    let nonce2 = nonce + 1;
    let signed2 = match onchainos_hl_sign(&delegate_action, nonce2, &wallet, chain_id, true, false) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "EIP712_SIGN_FAILED",
                "Step 1 succeeded. Step 2 (delegate) signing failed — retry with new command.",
            ));
            return Ok(());
        }
    };
    let result2 = match submit_exchange_request(exchange, signed2).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "TX_SUBMIT_FAILED",
                "Step 1 succeeded. Step 2 (delegate) failed — check staking-info and retry.",
            ));
            return Ok(());
        }
    };

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "action": "redelegate",
        "wallet": wallet,
        "from_validator": args.from_validator,
        "to_validator": args.to_validator,
        "amount": api::format_hype_amount(amount_raw),
        "amount_raw": amount_raw.to_string(),
        "step_1_undelegate_result": result1,
        "step_2_delegate_result": result2,
        "note": "HYPE redelegated. Use staking-info to see updated delegations.",
    }))?);
    Ok(())
}
