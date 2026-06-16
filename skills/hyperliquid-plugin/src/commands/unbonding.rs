use clap::Args;
use crate::api;
use crate::config::info_url;
use crate::onchainos::resolve_wallet;
use crate::config::ARBITRUM_CHAIN_ID;
use super::error_response;

#[derive(Args)]
pub struct UnbondingArgs {
    /// Wallet address to query (defaults to the connected onchainos wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: UnbondingArgs) -> anyhow::Result<()> {
    let wallet = match args.address {
        Some(addr) => addr,
        None => match resolve_wallet(ARBITRUM_CHAIN_ID) {
            Ok(w) => w,
            Err(e) => {
                println!("{}", error_response(
                    &format!("{:#}", e),
                    "WALLET_NOT_FOUND",
                    "Run onchainos wallet addresses to verify login.",
                ));
                return Ok(());
            }
        },
    };

    let url = info_url();
    // Unbonding / pending-withdrawal state is exposed by HL via `delegatorSummary`, NOT the
    // `delegations` endpoint (which only lists active delegations, so unbonding entries would
    // never appear). Field names per HL docs: `undelegated` (HYPE undelegated, in the lock
    // before it returns to the staking balance), `totalPendingWithdrawal` + `nPendingWithdrawals`
    // (the staking -> spot withdrawal queue).
    let summary = match api::get_delegator_summary(url, &wallet).await {
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

    // Amounts are decimal HYPE strings (e.g. "0.17").
    let undelegated_raw = summary["undelegated"].as_str()
        .and_then(|s| api::parse_hype_amount(s).ok())
        .unwrap_or(0);
    let pending_withdrawal_raw = summary["totalPendingWithdrawal"].as_str()
        .and_then(|s| api::parse_hype_amount(s).ok())
        .unwrap_or(0);
    let pending_withdrawals_count = summary["nPendingWithdrawals"].as_u64().unwrap_or(0);

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "wallet": wallet,
        "undelegated": api::format_hype_amount(undelegated_raw),
        "undelegated_raw": undelegated_raw.to_string(),
        "pending_withdrawal": api::format_hype_amount(pending_withdrawal_raw),
        "pending_withdrawal_raw": pending_withdrawal_raw.to_string(),
        "pending_withdrawals_count": pending_withdrawals_count,
        "source": "delegatorSummary",
    }))?);
    Ok(())
}
