use crate::config::{find_vault, format_amount, CHAIN_ID, DEFAULT_SLIPPAGE_BPS};
use crate::onchainos;
use crate::rpc;
use clap::Args;
use serde_json::json;

#[derive(Args, Debug)]
pub struct RedeemArgs {
    /// Vault address to redeem from
    #[arg(long)]
    pub vault: String,

    /// Number of shares to redeem (human-readable). Omit to redeem all shares.
    #[arg(long)]
    pub shares: Option<String>,

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

pub async fn run(args: RedeemArgs) -> anyhow::Result<()> {
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

    // Determine shares to redeem
    let total_shares = rpc::balance_of(&rpc_urls, &args.vault, &wallet)
        .await
        .unwrap_or(0);
    if total_shares == 0 {
        anyhow::bail!("No shares held in vault {}.", args.vault);
    }

    let shares_to_redeem = if let Some(ref s) = args.shares {
        crate::config::parse_amount(s, 18)?
    } else {
        // Redeem all shares
        total_shares
    };

    if shares_to_redeem == 0 {
        anyhow::bail!("Shares amount must be greater than zero.");
    }
    if shares_to_redeem > total_shares {
        anyhow::bail!(
            "Cannot redeem {} shares; only {} held.",
            format_amount(shares_to_redeem, 18),
            format_amount(total_shares, 18)
        );
    }

    // Get expected assets
    let expected_assets = rpc::convert_to_assets(&rpc_urls, &args.vault, shares_to_redeem)
        .await
        .unwrap_or(0);

    // Compute minimumReceive
    let slippage_bps = args.slippage_bps.unwrap_or(DEFAULT_SLIPPAGE_BPS);
    let min_assets = if slippage_bps == 0 || expected_assets == 0 {
        0u128
    } else {
        expected_assets - (expected_assets * slippage_bps as u128 / 10000)
    };

    eprintln!(
        "Redeeming {} shares from {}",
        format_amount(shares_to_redeem, 18),
        vault_info.name
    );
    eprintln!(
        "Expected: {} {} (min: {})",
        format_amount(expected_assets, vault_info.underlying_decimals),
        vault_info.underlying_symbol,
        format_amount(min_assets, vault_info.underlying_decimals)
    );
    eprintln!("ask user to confirm before proceeding");

    // Encode redeem(shares, receiver, owner, minimumReceive)
    // selector: 0x9f40a7b3
    let wallet_stripped = wallet.strip_prefix("0x").unwrap_or(&wallet);
    let calldata = format!(
        "0x9f40a7b3{}{:0>64}{:0>64}{}",
        crate::rpc::pad_u128(shares_to_redeem),
        wallet_stripped,
        wallet_stripped,
        crate::rpc::pad_u128(min_assets),
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
        anyhow::bail!("redeem failed: {}", result["error"].as_str().unwrap_or("unknown error"));
    }
    let tx_hash = onchainos::extract_tx_hash(&result);

    let output = json!({
        "status": if args.dry_run { "dry_run" } else { "submitted" },
        "vault": args.vault,
        "vault_name": vault_info.name,
        "underlying_symbol": vault_info.underlying_symbol,
        "shares_redeemed_raw": shares_to_redeem.to_string(),
        "shares_redeemed_formatted": format_amount(shares_to_redeem, 18),
        "expected_assets_raw": expected_assets.to_string(),
        "expected_assets_formatted": format!("{} {}", format_amount(expected_assets, vault_info.underlying_decimals), vault_info.underlying_symbol),
        "min_assets_raw": min_assets.to_string(),
        "min_assets_formatted": format!("{} {}", format_amount(min_assets, vault_info.underlying_decimals), vault_info.underlying_symbol),
        "slippage_bps": slippage_bps,
        "wallet": wallet,
        "tx_hash": tx_hash,
        "dry_run": args.dry_run,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
