// unstake — Request ETHx withdrawal (receive ETH after finalization)
// 2-step write operation: approve ETHx allowance + requestWithdraw
// Requires user confirmation before broadcasting.
//
// Step 1: ETHx.approve(UserWithdrawManager, ethx_amount)
//   selector: 0x095ea7b3
// Step 2: UserWithdrawManager.requestWithdraw(uint256 ethXAmount, address _owner)
//   selector: 0xccc143b8 (verified via cast sig)
//
// Returns requestId — save for claim command.

use anyhow::Result;
use clap::Args;
use serde_json::json;
use crate::config;
use crate::rpc;
use crate::onchainos;

#[derive(Args)]
pub struct UnstakeArgs {
    /// ETHx amount in wei to unstake
    #[arg(long)]
    pub amount: u128,

    /// Owner address for the withdrawal request (defaults to logged-in wallet)
    #[arg(long)]
    pub owner: Option<String>,
}

pub async fn execute(args: &UnstakeArgs, rpc_url: &str, chain_id: u64, dry_run: bool) -> Result<()> {
    if dry_run {
        // Build calldata for requestWithdraw (dry-run uses zero address placeholder)
        let amount_hex = format!("{:064x}", args.amount);
        let zero_owner = "0000000000000000000000000000000000000000000000000000000000000000";
        let calldata = format!("0xccc143b8{}{}", amount_hex, zero_owner);

        let output = json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "step1_calldata": {
                "selector": "0x095ea7b3",
                "description": "approve(address,uint256) — ETHx approval for UserWithdrawManager"
            },
            "step2_calldata": calldata,
            "step2_selector": "0xccc143b8",
            "description": "requestWithdraw(uint256,address) — request ETH withdrawal",
            "ethx_amount_wei": args.amount.to_string(),
            "ethx_amount": rpc::format_eth(args.amount)
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Resolve owner address
    let owner = match &args.owner {
        Some(o) => o.clone(),
        None => onchainos::resolve_wallet(chain_id)?,
    };

    // Check current allowance; only approve if insufficient
    let current_allowance = rpc::ethx_allowance(rpc_url, config::ETHX_TOKEN, &owner, config::USER_WITHDRAW_MANAGER).await.unwrap_or(0);

    let mut approve_tx = None;
    if current_allowance < args.amount {
        let approve_calldata = onchainos::erc20_approve_calldata(config::USER_WITHDRAW_MANAGER, args.amount);
        let approve_result = onchainos::wallet_contract_call(
            chain_id,
            config::ETHX_TOKEN,
            &approve_calldata,
            None,
            false,
        )?;
        let approve_hash = onchainos::extract_tx_hash(&approve_result);
        approve_tx = Some(approve_hash);

        // Small delay to allow approve to propagate
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    // Build requestWithdraw calldata
    // requestWithdraw(uint256 _ethXAmount, address _owner)
    // selector: 0xccc143b8
    let amount_hex = format!("{:064x}", args.amount);
    let owner_clean = owner.trim_start_matches("0x");
    let owner_padded = format!("{:0>64}", owner_clean);
    let calldata = format!("0xccc143b8{}{}", amount_hex, owner_padded);

    let result = onchainos::wallet_contract_call(
        chain_id,
        config::USER_WITHDRAW_MANAGER,
        &calldata,
        None,
        false,
    )?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    let mut output_data = json!({
        "action": "unstake",
        "ethx_amount_wei": args.amount.to_string(),
        "ethx_amount": rpc::format_eth(args.amount),
        "owner": owner,
        "request_tx_hash": tx_hash,
        "note": "Withdrawal finalization typically takes 3-10 days. Use 'claim' command with your requestId once finalized.",
        "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
    });

    if let Some(hash) = approve_tx {
        output_data["approve_tx_hash"] = json!(hash);
    }

    let output = json!({
        "ok": true,
        "data": output_data
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
