// Withdraw from a Beefy vault (redeem mooTokens for underlying)
//
// Beefy uses BeefyVaultV7 (NOT ERC-4626):
//   withdraw(uint256 _shares) selector: 0x2e1a7d4d (cast sig "withdraw(uint256)" verified)
//   withdrawAll() selector: 0x853828b6
//
// Flow:
//   1. Resolve vault from ID or address
//   2. Determine shares to redeem (user-specified or full balance)
//   3. Submit withdraw(uint256 _shares) — selector: 0x2e1a7d4d

use anyhow::Result;
use serde_json::{json, Value};

use crate::api;
use crate::config::chain_id_to_beefy_name;
use crate::onchainos;
use crate::rpc;

pub async fn execute(
    chain_id: u64,
    vault_query: &str,
    shares_str: Option<&str>,
    dry_run: bool,
    wallet: Option<&str>,
) -> Result<Value> {
    let chain_name = chain_id_to_beefy_name(chain_id)
        .ok_or_else(|| anyhow::anyhow!("Unsupported chain ID: {}", chain_id))?;

    let vaults = api::fetch_vaults(chain_id).await?;
    let vault = api::find_vault(&vaults, vault_query)
        .ok_or_else(|| anyhow::anyhow!("Vault not found: {}", vault_query))?;

    let earn_addr = vault
        .earn_contract_address
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Vault {} has no earnContractAddress", vault.id))?;

    // Resolve wallet
    let wallet_addr = if dry_run {
        wallet.map(|w| w.to_string()).unwrap_or_else(|| {
            "0x0000000000000000000000000000000000000000".to_string()
        })
    } else {
        match wallet {
            Some(w) => w.to_string(),
            None => onchainos::resolve_wallet(chain_id)?,
        }
    };

    // Determine shares to redeem
    // shares are passed as raw mooToken units (same decimal scale as underlying asset)
    // For USDC vaults: 10000 = 0.01 USDC worth of mooTokens
    let shares_raw: u128 = if let Some(s) = shares_str {
        // Parse as raw mooToken integer (e.g. "9927" for a USDC vault balance)
        s.parse::<u128>()
            .map_err(|_| anyhow::anyhow!("Invalid shares: expected raw integer (e.g. 9927), got: {}", s))?
    } else {
        // Redeem full balance
        if dry_run {
            1000u128 // placeholder for dry-run
        } else {
            rpc::get_moo_balance(chain_id, earn_addr, &wallet_addr).await?
        }
    };

    if shares_raw == 0 {
        anyhow::bail!("No mooToken balance to redeem in vault {}", vault.id);
    }

    // withdraw(uint256 _shares)
    // Beefy BeefyVaultV7 withdraw selector: 0x2e1a7d4d (cast sig "withdraw(uint256)" verified)
    // NOTE: This is NOT ERC-4626 — Beefy uses single-param withdraw
    let shares_hex = format!("{:064x}", shares_raw);
    let redeem_calldata = format!("0x2e1a7d4d{}", shares_hex);

    let result = onchainos::wallet_contract_call_force(
        chain_id,
        earn_addr,
        &redeem_calldata,
        Some(&wallet_addr),
        None,
        dry_run,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    let explorer_url = match chain_id {
        8453 => format!("https://basescan.org/tx/{}", tx_hash),
        56 => format!("https://bscscan.com/tx/{}", tx_hash),
        1 => format!("https://etherscan.io/tx/{}", tx_hash),
        _ => format!("https://blockscan.com/tx/{}", tx_hash),
    };

    Ok(json!({
        "ok": true,
        "dry_run": dry_run,
        "chain": chain_name,
        "vault_id": vault.id,
        "vault_name": vault.name,
        "earn_contract": earn_addr,
        "shares_redeemed_raw": shares_raw.to_string(),
        "wallet": wallet_addr,
        "redeem_tx": {
            "txHash": tx_hash,
            "calldata": redeem_calldata,
            "selector": "0x2e1a7d4d",
            "explorer": if dry_run { Value::Null } else { Value::String(explorer_url) },
        }
    }))
}
