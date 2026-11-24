/// `euler-v2-plugin disable-controller` — release the borrower-vault designation.
///
/// Called on the **vault contract** (not the EVC), with no arguments. The vault's
/// `disableController()` function checks `debtOf(msg.sender) == 0` and only then
/// notifies EVC to clear the role. Required after `repay --all` to free up the
/// account for full collateral withdrawal.

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain};
use crate::rpc::{eth_get_balance_wei, estimate_native_gas_cost_wei, wei_to_eth};
use crate::calldata::build_disable_controller;

const GAS_LIMIT: u64 = 200_000;

#[derive(Args)]
pub struct DisableControllerArgs {
    /// The currently-enabled controller vault to disable
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

pub async fn run(args: DisableControllerArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("disable-controller"), None)); Ok(()) }
    }
}

async fn run_inner(args: DisableControllerArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    let vault_addr = args.vault.to_lowercase();
    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;
    let calldata = build_disable_controller();

    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true, "dry_run": true,
            "data": {
                "action": "disable_controller",
                "chain": chain_name(args.chain), "chain_id": args.chain,
                "wallet": wallet, "vault": vault_addr,
                "calldata": calldata,
                "note": "dry-run: no transaction submitted. \
                         The vault enforces debtOf(account) == 0; if you still owe debt, this will revert. \
                         Repay first with `repay --vault <this> --all`."
            }
        }))?);
        return Ok(());
    }

    let need_wei = estimate_native_gas_cost_wei(args.chain, GAS_LIMIT).await?;
    let have_wei = eth_get_balance_wei(args.chain, &wallet).await?;
    if have_wei < need_wei {
        anyhow::bail!("Insufficient native gas: have {:.6} ETH, need ~{:.6} ETH.",
            wei_to_eth(have_wei), wei_to_eth(need_wei));
    }

    eprintln!("[euler-v2] disabling controller on vault {}...", vault_addr);
    let resp = crate::onchainos::wallet_contract_call(
        args.chain, &vault_addr, &calldata, Some(&wallet), None, false, false,
    ).await?;
    let tx = crate::onchainos::extract_tx_hash(&resp)?;
    eprintln!("[euler-v2] tx: {} (waiting...)", tx);
    crate::onchainos::wait_for_tx_receipt(&tx, args.chain, 120).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "action": "disable_controller",
            "chain": chain_name(args.chain), "chain_id": args.chain,
            "wallet": wallet, "vault": vault_addr,
            "tx_hash": tx, "on_chain_status": "0x1",
            "tip": "Controller cleared. You can now withdraw all collateral if desired."
        }
    }))?);
    Ok(())
}
