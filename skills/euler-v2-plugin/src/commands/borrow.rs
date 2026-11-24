/// `euler-v2-plugin borrow` — borrow underlying asset from a controller vault.
///
/// Pre-conditions enforced by Euler v2 (will revert on-chain if missing):
///   1. `enable-controller --vault <this>` has been called (EVC tracks borrower)
///   2. `enable-collateral --vault <some-supply-vault>` has been called for backing
///   3. The user's account is healthy after the borrow (LTV checks)
///
/// The plugin doesn't pre-validate (1) or (2) yet (defers to EVC's revert), but
/// surfaces the error code clearly so the Agent can guide the user.

use anyhow::{Context, Result};
use clap::Args;

use crate::config::{chain_name, is_supported_chain};
use crate::rpc::{eth_get_balance_wei, estimate_native_gas_cost_wei, wei_to_eth};
use crate::calldata::build_borrow;

const GAS_LIMIT_BORROW: u64 = 400_000;

#[derive(Args)]
pub struct BorrowArgs {
    /// Vault to borrow FROM (must already be the user's enabled controller)
    #[arg(long)]
    pub vault: String,

    /// Amount in underlying-asset units (e.g. `0.5` for 0.5 ETH)
    #[arg(long)]
    pub amount: String,

    #[arg(long, default_value_t = 1)]
    pub chain: u64,

    #[arg(long)]
    pub dry_run: bool,

    /// Required to broadcast. Without this, the command prints a preview
    /// (calldata + intent) and exits without touching the chain.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: BorrowArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("borrow"), None)); Ok(()) }
    }
}

async fn run_inner(args: BorrowArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    let vault_addr = args.vault.to_lowercase();

    let vaults = crate::api::get_vaults_raw(args.chain).await?;
    let evk = vaults["evkVaults"].as_array()
        .ok_or_else(|| anyhow::anyhow!("Euler API returned no evkVaults"))?;
    let entry = evk.iter()
        .find(|v| v["address"].as_str().map(|s| s.to_lowercase()) == Some(vault_addr.clone()))
        .ok_or_else(|| anyhow::anyhow!("Vault {} not found in Euler API", args.vault))?;
    let decimals: u32 = entry["asset"]["decimals"]["__bi"].as_str()
        .and_then(|s| s.parse().ok()).unwrap_or(18);
    let asset_symbol = entry["asset"]["symbol"].as_str().unwrap_or("?").to_string();

    let amt_f: f64 = args.amount.parse()
        .with_context(|| format!("Invalid amount '{}'", args.amount))?;
    if amt_f <= 0.0 { anyhow::bail!("amount must be positive"); }
    let amount_raw = (amt_f * 10f64.powi(decimals as i32)).round() as u128;

    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;
    let calldata = build_borrow(amount_raw, &wallet);

    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true, "dry_run": true,
            "data": {
                "action": "borrow",
                "chain": chain_name(args.chain), "chain_id": args.chain,
                "wallet": wallet,
                "vault": vault_addr, "vault_name": entry["name"],
                "asset": asset_symbol,
                "amount": args.amount, "amount_raw": amount_raw.to_string(),
                "decimals": decimals,
                "note": "dry-run: no transaction submitted. \
                         If this borrow fails on-chain with a revert, verify: \
                         (1) enable-controller --vault <this> has been run, \
                         (2) at least one collateral vault is enabled, \
                         (3) the resulting LTV is within the vault's limits."
            }
        }))?);
        return Ok(());
    }

    let need_wei = estimate_native_gas_cost_wei(args.chain, GAS_LIMIT_BORROW).await?;
    let have_wei = eth_get_balance_wei(args.chain, &wallet).await?;
    if have_wei < need_wei {
        anyhow::bail!("Insufficient native gas: have {:.6} ETH, need ~{:.6} ETH.",
            wei_to_eth(have_wei), wei_to_eth(need_wei));
    }

    eprintln!("[euler-v2] borrowing {:.6} {} from vault {}...", amt_f, asset_symbol, vault_addr);
    let resp = crate::onchainos::wallet_contract_call(
        args.chain, &vault_addr, &calldata, Some(&wallet), None, false, false,
    ).await?;
    let tx = crate::onchainos::extract_tx_hash(&resp)?;
    eprintln!("[euler-v2] tx: {} (waiting...)", tx);
    crate::onchainos::wait_for_tx_receipt(&tx, args.chain, 120).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "action": "borrow",
            "chain": chain_name(args.chain), "chain_id": args.chain,
            "wallet": wallet, "vault": vault_addr,
            "asset": asset_symbol,
            "amount": args.amount, "amount_raw": amount_raw.to_string(),
            "tx_hash": tx, "on_chain_status": "0x1",
            "tip": "Run `health-factor --chain ".to_string() + &args.chain.to_string() + "` to verify safety margin."
        }
    }))?);
    Ok(())
}
