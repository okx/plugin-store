// positions — Query user's ETHx balance and pending withdrawal requests
// Read-only: no wallet confirmation needed.

use anyhow::Result;
use clap::Args;
use serde_json::json;
use crate::config;
use crate::rpc;
use crate::onchainos;

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query (defaults to logged-in wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn execute(args: &PositionsArgs, rpc_url: &str, chain_id: u64, dry_run: bool) -> Result<()> {
    if dry_run {
        let output = json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "ethx_balance": "0.00000",
                "ethx_balance_wei": "0",
                "eth_value": "0.00000",
                "pending_withdrawals": []
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let wallet = match &args.address {
        Some(a) => a.clone(),
        None => onchainos::resolve_wallet(chain_id)?,
    };

    let ethx_token = config::ETHX_TOKEN;
    let withdraw_mgr = config::USER_WITHDRAW_MANAGER;
    let manager = config::STADER_MANAGER;

    // ETHx balance
    let ethx_balance = rpc::ethx_balance_of(rpc_url, ethx_token, &wallet).await?;

    // Convert ETHx to ETH value
    let eth_value = if ethx_balance > 0 {
        rpc::convert_to_assets(rpc_url, manager, ethx_balance).await.unwrap_or(0)
    } else {
        0
    };

    // Pending withdrawal requests
    let request_ids = rpc::get_request_ids_by_user(rpc_url, withdraw_mgr, &wallet).await?;

    let mut withdrawals = Vec::new();
    for req_id in &request_ids {
        match rpc::get_withdraw_request(rpc_url, withdraw_mgr, *req_id).await {
            Ok(info) => withdrawals.push(info),
            Err(e) => {
                eprintln!("Warning: failed to fetch request {}: {}", req_id, e);
            }
        }
    }

    let output = json!({
        "ok": true,
        "data": {
            "wallet": wallet,
            "ethx_balance": rpc::format_eth(ethx_balance),
            "ethx_balance_wei": ethx_balance.to_string(),
            "eth_value": rpc::format_eth(eth_value),
            "pending_withdrawal_count": request_ids.len(),
            "pending_withdrawals": withdrawals,
            "chain": "Ethereum Mainnet (1)"
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
