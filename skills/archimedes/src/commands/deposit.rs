use crate::config::{find_vault, format_amount, parse_amount, CHAIN_ID};
use crate::onchainos;
use crate::rpc;
use clap::Args;
use serde_json::json;
use std::time::Duration;

#[derive(Args, Debug)]
pub struct DepositArgs {
    /// Vault address to deposit into
    #[arg(long)]
    pub vault: String,

    /// Amount of underlying asset to deposit (human-readable, e.g. "0.01")
    #[arg(long)]
    pub amount: String,

    /// Wallet address (receiver); defaults to logged-in wallet
    #[arg(long)]
    pub from: Option<String>,

    /// Custom RPC URL
    #[arg(long)]
    pub rpc: Option<String>,

    /// Simulate without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: DepositArgs) -> anyhow::Result<()> {
    let rpc_url_owned: Option<String> = args.rpc.clone();
    let rpc_urls: Vec<&str> = if let Some(ref url) = rpc_url_owned {
        vec![url.as_str()]
    } else {
        crate::config::RPC_URLS.to_vec()
    };

    // Resolve vault info
    let vault_info = find_vault(&args.vault)
        .ok_or_else(|| anyhow::anyhow!("Unknown vault address: {}. Run `archimedes vaults` to list known vaults.", args.vault))?;

    // Resolve wallet
    let wallet = if let Some(ref w) = args.from {
        w.clone()
    } else {
        let w = onchainos::resolve_wallet(CHAIN_ID)?;
        if w.is_empty() {
            anyhow::bail!(
                "Could not resolve wallet address. Pass --from or ensure onchainos is logged in."
            );
        }
        w
    };

    // Parse amount
    let assets = parse_amount(&args.amount, vault_info.underlying_decimals)?;
    if assets == 0 {
        anyhow::bail!("Deposit amount must be greater than zero.");
    }

    // Check underlying balance
    let balance = rpc::balance_of(&rpc_urls, vault_info.underlying_address, &wallet)
        .await
        .unwrap_or(0);
    if balance < assets {
        anyhow::bail!(
            "Insufficient {} balance. Have: {}, Need: {}",
            vault_info.underlying_symbol,
            format_amount(balance, vault_info.underlying_decimals),
            format_amount(assets, vault_info.underlying_decimals)
        );
    }

    // Preview shares to be received
    let preview_shares = rpc::preview_deposit(&rpc_urls, &args.vault, assets)
        .await
        .unwrap_or(0);

    eprintln!(
        "Depositing {} {} into {}",
        format_amount(assets, vault_info.underlying_decimals),
        vault_info.underlying_symbol,
        vault_info.name
    );
    eprintln!(
        "Expected shares: {}",
        format_amount(preview_shares, 18)
    );
    eprintln!("ask user to confirm before proceeding");

    // Step 1: ERC-20 approve(vault, assets)
    eprintln!(
        "\n[1/2] Approving {} to spend {} {}...",
        args.vault,
        format_amount(assets, vault_info.underlying_decimals),
        vault_info.underlying_symbol
    );
    let approve_result = onchainos::erc20_approve(
        CHAIN_ID,
        vault_info.underlying_address,
        &args.vault,
        assets,
        Some(&wallet),
        args.dry_run,
    )
    .await?;
    if !args.dry_run && approve_result["ok"].as_bool() != Some(true) {
        anyhow::bail!("approve failed: {}", approve_result["error"].as_str().unwrap_or("unknown error"));
    }
    let approve_tx = onchainos::extract_tx_hash(&approve_result);
    eprintln!("  Approve tx: {}", approve_tx);

    // Wait to avoid nonce collision
    if !args.dry_run {
        tokio::time::sleep(Duration::from_secs(crate::config::APPROVE_DELAY_SECS)).await;
    }

    // Step 2: deposit(assets, receiver)
    // selector: 0x6e553f65
    let wallet_stripped = wallet.strip_prefix("0x").unwrap_or(&wallet);
    let calldata = format!(
        "0x6e553f65{}{:0>64}",
        crate::rpc::pad_u128(assets),
        wallet_stripped
    );

    eprintln!("\n[2/2] Depositing into vault...");
    let deposit_result = onchainos::wallet_contract_call(
        CHAIN_ID,
        &args.vault,
        &calldata,
        Some(&wallet),
        None,
        args.dry_run,
    )
    .await?;
    if !args.dry_run && deposit_result["ok"].as_bool() != Some(true) {
        anyhow::bail!("deposit failed: {}", deposit_result["error"].as_str().unwrap_or("unknown error"));
    }
    let deposit_tx = onchainos::extract_tx_hash(&deposit_result);

    let output = json!({
        "status": if args.dry_run { "dry_run" } else { "submitted" },
        "vault": args.vault,
        "vault_name": vault_info.name,
        "underlying_symbol": vault_info.underlying_symbol,
        "assets_deposited_raw": assets.to_string(),
        "assets_deposited_formatted": format!("{} {}", format_amount(assets, vault_info.underlying_decimals), vault_info.underlying_symbol),
        "expected_shares": format_amount(preview_shares, 18),
        "wallet": wallet,
        "approve_tx": approve_tx,
        "deposit_tx": deposit_tx,
        "dry_run": args.dry_run,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
