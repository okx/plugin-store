// stake — Deposit ETH to receive ETHx liquid staking token
// Write operation: requires user confirmation before broadcasting.
//
// Contract: StaderStakePoolsManager.deposit(address _receiver)
// Selector: 0xf340fa01 (verified via cast sig)
// Payable: yes — ETH amount passed via --amt (wei)
// Min deposit: 0.0001 ETH (100000000000000 wei) — protocol enforced

use anyhow::Result;
use clap::Args;
use serde_json::json;
use crate::config;
use crate::rpc;
use crate::onchainos;

#[derive(Args)]
pub struct StakeArgs {
    /// ETH amount in wei to stake (min: 100000000000000 = 0.0001 ETH)
    #[arg(long)]
    pub amount: u64,

    /// Receiver address for ETHx (defaults to logged-in wallet)
    #[arg(long)]
    pub receiver: Option<String>,
}

pub async fn execute(args: &StakeArgs, rpc_url: &str, chain_id: u64, dry_run: bool) -> Result<()> {
    // Validate minimum deposit
    if args.amount < config::MIN_DEPOSIT_WEI {
        anyhow::bail!(
            "Amount {} wei is below minimum deposit ({} wei = 0.0001 ETH)",
            args.amount,
            config::MIN_DEPOSIT_WEI
        );
    }

    // dry_run guard — resolve wallet only after this check
    if dry_run {
        // Build calldata using zero address as placeholder
        let zero_addr = "0000000000000000000000000000000000000000000000000000000000000000";
        let calldata = format!("0xf340fa01{}", zero_addr);
        let output = json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "calldata": calldata,
            "calldata_selector": "0xf340fa01",
            "description": "deposit(address) — stake ETH to receive ETHx",
            "eth_amount_wei": args.amount.to_string(),
            "eth_amount": rpc::format_eth(args.amount as u128)
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Resolve receiver address
    let receiver = match &args.receiver {
        Some(r) => r.clone(),
        None => onchainos::resolve_wallet(chain_id)?,
    };

    // Preview expected ETHx
    let ethx_preview = rpc::preview_deposit(rpc_url, config::STADER_MANAGER, args.amount as u128).await.unwrap_or(0);

    // Build calldata: deposit(address _receiver)
    // selector 0xf340fa01 + 32-byte padded receiver address
    let receiver_clean = receiver.trim_start_matches("0x");
    let receiver_padded = format!("{:0>64}", receiver_clean);
    let calldata = format!("0xf340fa01{}", receiver_padded);

    // Execute on-chain
    let result = onchainos::wallet_contract_call(
        chain_id,
        config::STADER_MANAGER,
        &calldata,
        Some(args.amount),
        false,
    )?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    let output = json!({
        "ok": true,
        "data": {
            "txHash": tx_hash,
            "action": "stake",
            "eth_staked_wei": args.amount.to_string(),
            "eth_staked": rpc::format_eth(args.amount as u128),
            "ethx_expected_wei": ethx_preview.to_string(),
            "ethx_expected": rpc::format_eth(ethx_preview),
            "receiver": receiver,
            "contract": config::STADER_MANAGER,
            "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
