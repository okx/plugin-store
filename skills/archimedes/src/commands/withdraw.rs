use crate::config::{find_vault, format_amount, parse_amount, CHAIN_ID, DEFAULT_SLIPPAGE_BPS};
use crate::onchainos;
use crate::rpc;
use clap::Args;
use serde_json::json;

#[derive(Args, Debug)]
pub struct WithdrawArgs {
    /// Vault address to withdraw from
    #[arg(long)]
    pub vault: String,

    /// Amount of underlying asset to withdraw (human-readable, e.g. "0.01")
    #[arg(long)]
    pub amount: String,

    /// Wallet address (receiver and owner); defaults to logged-in wallet
    #[arg(long)]
    pub from: Option<String>,

    /// Slippage in basis points (default: 50 = 0.5%). Use 0 to disable minimum.
    #[arg(long)]
    pub slippage_bps: Option<u64>,

    /// Custom RPC URL
    #[arg(long)]
    pub rpc: Option<String>,

    /// Simulate without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: WithdrawArgs) -> anyhow::Result<()> {
    let rpc_url_owned: Option<String> = args.rpc.clone();
    let rpc_urls: Vec<&str> = if let Some(ref url) = rpc_url_owned {
        vec![url.as_str()]
    } else {
        crate::config::RPC_URLS.to_vec()
    };

    let vault_info = find_vault(&args.vault).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown vault address: {}. Run `archimedes vaults` to list known vaults.",
            args.vault
        )
    })?;

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

    let assets = parse_amount(&args.amount, vault_info.underlying_decimals)?;
    if assets == 0 {
        anyhow::bail!("Withdraw amount must be greater than zero.");
    }

    // Check shares balance
    let shares = rpc::balance_of(&rpc_urls, &args.vault, &wallet)
        .await
        .unwrap_or(0);
    if shares == 0 {
        anyhow::bail!("No shares held in this vault.");
    }

    // Preview withdraw to check feasibility
    let shares_needed = rpc::preview_withdraw(&rpc_urls, &args.vault, assets)
        .await
        .unwrap_or(0);
    if shares_needed > shares {
        anyhow::bail!(
            "Insufficient shares. Need {} shares to withdraw {}, but only hold {}.",
            format_amount(shares_needed, 18),
            format_amount(assets, vault_info.underlying_decimals),
            format_amount(shares, 18)
        );
    }

    // Compute minimumReceive
    let slippage_bps = args.slippage_bps.unwrap_or(DEFAULT_SLIPPAGE_BPS);
    let minimum_receive = if slippage_bps == 0 {
        0u128
    } else {
        assets - (assets * slippage_bps as u128 / 10000)
    };

    eprintln!(
        "Withdrawing {} {} from {}",
        format_amount(assets, vault_info.underlying_decimals),
        vault_info.underlying_symbol,
        vault_info.name
    );
    eprintln!(
        "minimumReceive: {} {} (slippage: {} bps)",
        format_amount(minimum_receive, vault_info.underlying_decimals),
        vault_info.underlying_symbol,
        slippage_bps
    );
    eprintln!("ask user to confirm before proceeding");

    // Encode withdraw(assets, receiver, owner, minimumReceive)
    // selector: 0xa318c1a4
    let wallet_stripped = wallet.strip_prefix("0x").unwrap_or(&wallet);
    let calldata = format!(
        "0xa318c1a4{}{:0>64}{:0>64}{}",
        crate::rpc::pad_u128(assets),
        wallet_stripped,
        wallet_stripped,
        crate::rpc::pad_u128(minimum_receive),
    );

    let result = onchainos::wallet_contract_call_force(
        CHAIN_ID,
        &args.vault,
        &calldata,
        Some(&wallet),
        None,
        true,
        Some(600_000),
        args.dry_run,
    )
    .await?;
    if !args.dry_run && result["ok"].as_bool() != Some(true) {
        anyhow::bail!("withdraw failed: {}", result["error"].as_str().unwrap_or("unknown error"));
    }
    let tx_hash = onchainos::extract_tx_hash(&result);

    let output = json!({
        "status": if args.dry_run { "dry_run" } else { "submitted" },
        "vault": args.vault,
        "vault_name": vault_info.name,
        "underlying_symbol": vault_info.underlying_symbol,
        "assets_requested_raw": assets.to_string(),
        "assets_requested_formatted": format!("{} {}", format_amount(assets, vault_info.underlying_decimals), vault_info.underlying_symbol),
        "minimum_receive_raw": minimum_receive.to_string(),
        "minimum_receive_formatted": format!("{} {}", format_amount(minimum_receive, vault_info.underlying_decimals), vault_info.underlying_symbol),
        "slippage_bps": slippage_bps,
        "wallet": wallet,
        "tx_hash": tx_hash,
        "dry_run": args.dry_run,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
