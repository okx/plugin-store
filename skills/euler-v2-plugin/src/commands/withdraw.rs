/// `euler-v2-plugin withdraw` — burn vault shares to retrieve underlying asset.
///
/// `--all` uses ERC-4626 `redeem(shares, receiver, owner)` (burn user's full share count
/// to avoid rounding-down dust); explicit amount uses `withdraw(assets, receiver, owner)`.

use anyhow::{Context, Result};
use clap::Args;

use crate::config::{chain_name, is_supported_chain};
use crate::rpc::{build_address_call, eth_call, eth_get_balance_wei, estimate_native_gas_cost_wei,
    parse_uint256_to_u128, wei_to_eth, SELECTOR_BALANCE_OF};
use crate::calldata::{build_redeem, build_withdraw};

const GAS_LIMIT_WITHDRAW: u64 = 300_000;

#[derive(Args)]
pub struct WithdrawArgs {
    #[arg(long)]
    pub vault: String,

    /// Human-readable amount to withdraw (mutually exclusive with --all)
    #[arg(long)]
    pub amount: Option<String>,

    /// Withdraw the user's full share balance (uses redeem to avoid dust)
    #[arg(long)]
    pub all: bool,

    #[arg(long, default_value_t = 1)]
    pub chain: u64,

    #[arg(long)]
    pub dry_run: bool,

    /// Required to broadcast. Without this, the command prints a preview
    /// (calldata + intent) and exits without touching the chain.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: WithdrawArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("withdraw"), None)); Ok(()) }
    }
}

async fn run_inner(args: WithdrawArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    if args.all && args.amount.is_some() {
        anyhow::bail!("Pass either --amount or --all, not both.");
    }
    let vault_addr = args.vault.to_lowercase();

    let vaults = crate::api::get_vaults_raw(args.chain).await?;
    let evk = vaults["evkVaults"].as_array()
        .ok_or_else(|| anyhow::anyhow!("Euler API returned no evkVaults"))?;
    let entry = evk.iter()
        .find(|v| v["address"].as_str().map(|s| s.to_lowercase()) == Some(vault_addr.clone()))
        .ok_or_else(|| anyhow::anyhow!("Vault {} not found in Euler API", args.vault))?;
    let decimals: u32 = entry["asset"]["decimals"]["__bi"].as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(18);
    let asset_symbol = entry["asset"]["symbol"].as_str().unwrap_or("?").to_string();

    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;

    // Read user's current shares (for --all path AND to bail early if zero)
    let bal_call = build_address_call(SELECTOR_BALANCE_OF, &wallet);
    let shares_hex = eth_call(args.chain, &vault_addr, &bal_call).await?;
    let shares_raw = parse_uint256_to_u128(&shares_hex);
    if shares_raw == 0 {
        anyhow::bail!("No shares in vault {} for wallet {} — nothing to withdraw.", vault_addr, wallet);
    }

    let (calldata, amount_label, amount_raw): (String, String, u128) = if args.all {
        (build_redeem(shares_raw, &wallet, &wallet), "all".to_string(), shares_raw)
    } else {
        let amt_str = args.amount.as_ref()
            .ok_or_else(|| anyhow::anyhow!("--amount or --all is required"))?;
        let amt_f: f64 = amt_str.parse()
            .with_context(|| format!("Invalid amount '{}'", amt_str))?;
        if amt_f <= 0.0 { anyhow::bail!("amount must be positive"); }
        let amt_raw = (amt_f * 10f64.powi(decimals as i32)).round() as u128;
        (build_withdraw(amt_raw, &wallet, &wallet), amt_str.clone(), amt_raw)
    };

    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "action": if args.all { "withdraw_all (redeem)" } else { "withdraw" },
                "chain": chain_name(args.chain), "chain_id": args.chain,
                "wallet": wallet,
                "vault": vault_addr, "vault_name": entry["name"],
                "asset": asset_symbol,
                "current_shares_raw": shares_raw.to_string(),
                "amount": amount_label, "amount_raw": amount_raw.to_string(),
                "decimals": decimals,
                "note": "dry-run: no transaction submitted",
            }
        }))?);
        return Ok(());
    }

    let need_wei = estimate_native_gas_cost_wei(args.chain, GAS_LIMIT_WITHDRAW).await?;
    let have_wei = eth_get_balance_wei(args.chain, &wallet).await?;
    if have_wei < need_wei {
        anyhow::bail!(
            "Insufficient native gas: have {:.6} ETH, need ~{:.6} ETH on chain {}.",
            wei_to_eth(have_wei), wei_to_eth(need_wei), args.chain
        );
    }

    eprintln!("[euler-v2] withdrawing {} from vault {}...", amount_label, vault_addr);
    let resp = crate::onchainos::wallet_contract_call(
        args.chain, &vault_addr, &calldata,
        Some(&wallet), None, false, false,
    ).await?;
    let tx_hash = crate::onchainos::extract_tx_hash(&resp)?;
    eprintln!("[euler-v2] withdraw tx: {} (waiting...)", tx_hash);
    crate::onchainos::wait_for_tx_receipt(&tx_hash, args.chain, 120).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "action": if args.all { "withdraw_all (redeem)" } else { "withdraw" },
            "chain": chain_name(args.chain), "chain_id": args.chain,
            "wallet": wallet, "vault": vault_addr,
            "asset": asset_symbol,
            "amount": amount_label, "amount_raw": amount_raw.to_string(),
            "withdraw_tx": tx_hash,
            "on_chain_status": "0x1",
        }
    }))?);
    Ok(())
}
