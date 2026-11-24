/// `euler-v2-plugin enable-collateral` — designate a vault's shares as collateral.
///
/// Calls EVC.enableCollateral(account, vault). Required before the EVC will count
/// the user's shares of `vault` as backing for any borrow position.
///
/// Idempotent: re-enabling an already-enabled vault is a no-op on-chain.

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain};
use crate::rpc::{eth_get_balance_wei, estimate_native_gas_cost_wei, wei_to_eth};
use crate::calldata::build_enable_collateral;

const GAS_LIMIT_EVC_OP: u64 = 200_000;

#[derive(Args)]
pub struct EnableCollateralArgs {
    /// Vault address whose shares should be marked as collateral
    #[arg(long)]
    pub vault: String,

    #[arg(long, default_value_t = 1)]
    pub chain: u64,

    #[arg(long)]
    pub dry_run: bool,

    /// Required to broadcast. Without this, the command prints a preview
    /// (calldata + intent) and exits without touching the chain.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: EnableCollateralArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("enable-collateral"), None)); Ok(()) }
    }
}

async fn run_inner(args: EnableCollateralArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    let vault_addr = args.vault.to_lowercase();

    let chain_info = crate::api::get_chain(args.chain).await?;
    let evc_addr = chain_info.addresses.core_addrs.evc.clone()
        .ok_or_else(|| anyhow::anyhow!("EVC address missing from /api/euler-chains for chain {}", args.chain))?
        .to_lowercase();

    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;
    let calldata = build_enable_collateral(&wallet, &vault_addr);

    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true, "dry_run": true,
            "data": {
                "action": "enable_collateral",
                "chain": chain_name(args.chain), "chain_id": args.chain,
                "wallet": wallet, "vault": vault_addr, "evc": evc_addr,
                "calldata": calldata,
                "note": "dry-run: no transaction submitted. Re-run without --dry-run to broadcast."
            }
        }))?);
        return Ok(());
    }

    let need_wei = estimate_native_gas_cost_wei(args.chain, GAS_LIMIT_EVC_OP).await?;
    let have_wei = eth_get_balance_wei(args.chain, &wallet).await?;
    if have_wei < need_wei {
        anyhow::bail!(
            "Insufficient native gas: have {:.6} ETH, need ~{:.6} ETH on chain {}.",
            wei_to_eth(have_wei), wei_to_eth(need_wei), args.chain
        );
    }

    eprintln!("[euler-v2] enabling vault {} as collateral via EVC...", vault_addr);
    let resp = crate::onchainos::wallet_contract_call(
        args.chain, &evc_addr, &calldata,
        Some(&wallet), None, false, false,
    ).await?;
    let tx = crate::onchainos::extract_tx_hash(&resp)?;
    eprintln!("[euler-v2] tx: {} (waiting...)", tx);
    crate::onchainos::wait_for_tx_receipt(&tx, args.chain, 120).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "action": "enable_collateral",
            "chain": chain_name(args.chain), "chain_id": args.chain,
            "wallet": wallet, "vault": vault_addr, "evc": evc_addr,
            "tx_hash": tx, "on_chain_status": "0x1",
            "tip": "Vault is now collateral. Run `enable-controller --vault <borrower>` next, then `borrow`."
        }
    }))?);
    Ok(())
}
