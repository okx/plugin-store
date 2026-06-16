use clap::Args;
use crate::api;
use crate::config::{info_url, exchange_url, now_ms, ARBITRUM_CHAIN_ID};
use crate::onchainos::{resolve_wallet_with_chain, onchainos_hl_sign};
use crate::signing::{build_claim_reward_action, submit_exchange_request};
use super::error_response;

#[derive(Args)]
pub struct ClaimRewardsArgs {
    /// Submit the transaction on-chain (dry-run preview by default)
    #[arg(long, default_value_t = false)]
    pub confirm: bool,

    /// Wallet address override (defaults to onchainos connected wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: ClaimRewardsArgs) -> anyhow::Result<()> {
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

    // S-2: Check rewards — do not silently proceed if query fails
    let rewards_raw_str: String;
    let rewards_raw: u64;
    match api::get_delegator_rewards(url, &wallet).await {
        Ok(r) => {
            let s = r["pendingRewards"].as_str()
                .or_else(|| r["totalRewards"].as_str())
                .unwrap_or("0");
            rewards_raw = s.parse().unwrap_or_default();
            rewards_raw_str = s.to_string();
        }
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "API_ERROR",
                "Check your connection and retry.",
            ));
            return Ok(());
        }
    }

    if rewards_raw == 0 {
        println!("{}", error_response(
            "No pending rewards to claim",
            "NO_REWARDS_TO_CLAIM",
            "Stake HYPE to a validator to start earning rewards. Use staking-rewards to check.",
        ));
        return Ok(());
    }

    let action = build_claim_reward_action(nonce);

    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "preview": true,
            "action": "claim-rewards",
            "wallet": wallet,
            "estimated_rewards": api::format_hype_amount(rewards_raw),
            "estimated_rewards_raw": rewards_raw_str,
            "action_payload": action,
            "note": "Dry-run preview — add --confirm to sign and submit.",
        }))?);
        return Ok(());
    }

    // Sign + submit
    let signed = match onchainos_hl_sign(&action, nonce, &wallet, chain_id, true, false) {
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
            &format!("Claim rewards failed: {}", result["response"].as_str().unwrap_or("unknown error")),
            "CLAIM_FAILED",
            "Retry the command. If the issue persists, check onchainos status.",
        ));
        return Ok(());
    }

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "action": "claim-rewards",
        "wallet": wallet,
        "claimed_amount": api::format_hype_amount(rewards_raw),
        "claimed_amount_raw": rewards_raw_str,
        "result": result,
        "note": "Rewards claimed and added to your HYPE spot balance.",
    }))?);
    Ok(())
}
