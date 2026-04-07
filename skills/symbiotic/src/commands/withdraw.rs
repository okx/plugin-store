use clap::Args;
use serde_json::Value;
use crate::{api, config, onchainos, rpc};

#[derive(Args)]
pub struct WithdrawArgs {
    /// Token symbol to withdraw (e.g. wstETH, rETH, cbETH)
    #[arg(long, default_value = "wstETH")]
    pub token: String,

    /// Amount to withdraw (human-readable, e.g. 0.01)
    #[arg(long)]
    pub amount: String,

    /// Vault address (defaults to the largest vault for the token)
    #[arg(long)]
    pub vault: Option<String>,

    /// Recipient address for the withdrawal claim (defaults to logged-in wallet)
    #[arg(long)]
    pub from: Option<String>,

    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value = "1")]
    pub chain: u64,

    /// Simulate only — do not broadcast transaction
    #[arg(long, default_value = "false")]
    pub dry_run: bool,
}

pub async fn run(args: WithdrawArgs) -> anyhow::Result<Value> {
    // Resolve wallet — after dry_run guard
    let wallet = if args.dry_run {
        args.from.clone().unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string())
    } else {
        match args.from.clone() {
            Some(addr) => addr,
            None => {
                let w = onchainos::resolve_wallet(args.chain)?;
                if w.is_empty() {
                    anyhow::bail!("Cannot resolve wallet address. Pass --from or log in via onchainos.");
                }
                w
            }
        }
    };

    // Fetch vault info
    let vaults = api::fetch_vaults(100).await?;

    let vault_info = if let Some(ref addr) = args.vault {
        api::find_vault_by_address(&vaults, addr)
            .ok_or_else(|| anyhow::anyhow!("Vault {} not found", addr))?
    } else {
        api::find_vault_by_token(&vaults, &args.token)
            .ok_or_else(|| anyhow::anyhow!("No vault found for token {}. Use --vault <address> to specify.", args.token))?
    };

    let vault_name = vault_info.meta.as_ref()
        .and_then(|m| m.name.as_deref())
        .unwrap_or(&vault_info.token.symbol);

    let decimals = vault_info.token.decimals;
    let amount_raw = api::parse_amount(&args.amount, decimals)?;

    let rpc = config::ETH_RPC;

    // Check current epoch for informational output
    let current_epoch = rpc::current_epoch(&vault_info.address, rpc).await.unwrap_or(0);
    let epoch_dur = rpc::epoch_duration(&vault_info.address, rpc).await.unwrap_or(0);
    let epoch_days = epoch_dur / 86400;

    // Validate balance (skip for dry_run)
    if !args.dry_run {
        let balance = rpc::active_balance_of(&vault_info.address, &wallet, rpc).await
            .unwrap_or(0);
        if balance < amount_raw {
            let divisor = 10u128.pow(decimals as u32);
            let balance_fmt = format!("{}.{:0>width$}", balance / divisor, balance % divisor, width = decimals as usize);
            anyhow::bail!(
                "Insufficient balance: have {} {}, requested {}",
                balance_fmt, vault_info.token.symbol, args.amount
            );
        }
    }

    // Build calldata: withdraw(address claimer, uint256 amount)
    // selector: 0xf3fef3a3 [verified: cast sig "withdraw(address,uint256)"]
    let wallet_clean = wallet.trim_start_matches("0x");
    let wallet_padded = format!("{:0>64}", wallet_clean);
    let amount_hex = format!("{:064x}", amount_raw);
    let withdraw_calldata = format!("0xf3fef3a3{}{}", wallet_padded, amount_hex);

    if args.dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "action": "withdraw_request",
            "vault_address": vault_info.address,
            "vault_name": vault_name,
            "token_symbol": vault_info.token.symbol,
            "amount": args.amount,
            "amount_raw": amount_raw.to_string(),
            "current_epoch": current_epoch.to_string(),
            "epoch_duration_days": epoch_days.to_string(),
            "note": format!("Withdrawal will be claimable after epoch {} ends (~{} days)", current_epoch + 1, epoch_days),
            "calldata": withdraw_calldata,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" }
        }));
    }

    // Submit withdrawal — user must confirm before this transaction is sent
    let result = onchainos::wallet_contract_call(
        args.chain,
        &vault_info.address,
        &withdraw_calldata,
        Some(&wallet),
        None,
        false,
    ).await?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    Ok(serde_json::json!({
        "ok": true,
        "action": "withdraw_request",
        "vault_address": vault_info.address,
        "vault_name": vault_name,
        "token_symbol": vault_info.token.symbol,
        "amount": args.amount,
        "amount_raw": amount_raw.to_string(),
        "current_epoch": current_epoch.to_string(),
        "epoch_duration_days": epoch_days.to_string(),
        "note": format!("Withdrawal queued for epoch {}. You can claim your tokens after the epoch ends (~{} days).", current_epoch + 1, epoch_days),
        "txHash": tx_hash,
        "data": { "txHash": tx_hash }
    }))
}
