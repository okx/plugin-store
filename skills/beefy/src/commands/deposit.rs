// Deposit tokens into a Beefy vault
//
// Beefy uses BeefyVaultV7 (NOT ERC-4626):
//   deposit(uint256 _amount) selector: 0xb6b55f25 (cast sig "deposit(uint256)" verified)
//   depositAll() selector: 0xde5f6268
//
// Flow:
//   1. Resolve vault address and token address from vault ID
//   2. Parse amount with correct decimals
//   3. Check/submit ERC-20 approve(vault, amount)
//   4. Submit deposit(uint256 _amount) — selector: 0xb6b55f25

use anyhow::Result;
use serde_json::{json, Value};

use crate::api;
use crate::config::chain_id_to_beefy_name;
use crate::onchainos;
use crate::rpc;

pub async fn execute(
    chain_id: u64,
    vault_query: &str,
    amount_str: &str,
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

    let token_addr = vault
        .token_address
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Vault {} has no tokenAddress", vault.id))?;

    // Determine token decimals
    let decimals = if let Some(d) = vault.token_decimals {
        d
    } else {
        rpc::get_decimals(chain_id, token_addr).await.unwrap_or(18)
    };

    // Parse amount
    let amount_f: f64 = amount_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid amount: {}", amount_str))?;
    let denom = 10u128.pow(decimals);
    let amount_raw = (amount_f * denom as f64) as u128;

    if amount_raw == 0 {
        anyhow::bail!("Amount too small: {}", amount_str);
    }

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

    // Step 1: Check allowance and approve if needed
    let allowance = if dry_run {
        0u128
    } else {
        rpc::get_allowance(chain_id, token_addr, &wallet_addr, earn_addr)
            .await
            .unwrap_or(0)
    };

    let mut approve_tx = Value::Null;
    if allowance < amount_raw || dry_run {
        // Approve u128::MAX (unlimited) to avoid repeated approvals
        let approve_result = onchainos::erc20_approve(
            chain_id,
            token_addr,
            earn_addr,
            u128::MAX,
            Some(&wallet_addr),
            dry_run,
        )
        .await?;
        approve_tx = approve_result;

        // Wait briefly for approve to be mined (skip in dry_run)
        if !dry_run {
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
        }
    }

    // Step 2: deposit(uint256 _amount)
    // Beefy BeefyVaultV7 deposit selector: 0xb6b55f25 (cast sig "deposit(uint256)" verified)
    // NOTE: This is NOT ERC-4626 — Beefy uses a single-param deposit
    let amount_hex = format!("{:064x}", amount_raw);
    let deposit_calldata = format!("0xb6b55f25{}", amount_hex);

    let deposit_result = onchainos::wallet_contract_call_force(
        chain_id,
        earn_addr,
        &deposit_calldata,
        Some(&wallet_addr),
        None,
        dry_run,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&deposit_result);

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
        "token": vault.token,
        "token_address": token_addr,
        "amount": amount_str,
        "amount_raw": amount_raw.to_string(),
        "wallet": wallet_addr,
        "approve_tx": approve_tx,
        "deposit_tx": {
            "txHash": tx_hash,
            "calldata": deposit_calldata,
            "selector": "0xb6b55f25",
            "explorer": if dry_run { Value::Null } else { Value::String(explorer_url) },
        }
    }))
}
