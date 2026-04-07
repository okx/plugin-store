// deposit command — deposit ERC-20 assets into a Yearn ERC-4626 vault
// Flow: ERC-20 approve → 3s delay → ERC-4626 deposit(uint256,address)

use crate::{api, onchainos};
use anyhow::Result;
use serde_json::json;
use std::time::Duration;

pub async fn execute(
    chain_id: u64,
    vault_query: &str,
    amount: &str,
    dry_run: bool,
    wallet_override: Option<&str>,
) -> Result<()> {
    // dry_run guard BEFORE resolve_wallet (onchainos may not be available)
    if dry_run {
        let vaults = api::fetch_vaults(chain_id).await?;
        let vault = api::find_vault_by_address_or_symbol(&vaults, vault_query)
            .ok_or_else(|| anyhow::anyhow!("Vault not found for query: {}", vault_query))?;

        let token_decimals = vault.token.decimals;
        let amount_f: f64 = amount.parse().map_err(|_| anyhow::anyhow!("Invalid amount: {}", amount))?;
        let amount_raw = (amount_f * 10f64.powi(token_decimals as i32)) as u128;

        let approve_calldata = onchainos::encode_approve(&vault.address, amount_raw);
        let deposit_calldata = onchainos::encode_deposit(amount_raw, "0x0000000000000000000000000000000000000000");

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "vault": vault.address,
                "vault_name": vault.name.as_deref().unwrap_or(""),
                "token": vault.token.symbol,
                "token_address": vault.token.address,
                "amount": amount,
                "amount_raw": amount_raw.to_string(),
                "steps": [
                    {
                        "step": 1,
                        "action": "ERC-20 approve",
                        "to": vault.token.address,
                        "calldata": approve_calldata,
                        "selector": "0x095ea7b3"
                    },
                    {
                        "step": 2,
                        "action": "ERC-4626 deposit",
                        "to": vault.address,
                        "calldata": deposit_calldata,
                        "selector": "0x6e553f65"
                    }
                ]
            }))?
        );
        return Ok(());
    }

    // Resolve wallet address (after dry_run guard)
    let wallet = if let Some(w) = wallet_override {
        w.to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    let vaults = api::fetch_vaults(chain_id).await?;
    let vault = api::find_vault_by_address_or_symbol(&vaults, vault_query)
        .ok_or_else(|| anyhow::anyhow!("Vault not found for query: {}. Use 'vaults' command to list available vaults.", vault_query))?;

    let token_decimals = vault.token.decimals;
    let amount_f: f64 = amount.parse().map_err(|_| anyhow::anyhow!("Invalid amount: {}", amount))?;
    let amount_raw = (amount_f * 10f64.powi(token_decimals as i32)) as u128;

    eprintln!(
        "Depositing {} {} into {} ({})",
        amount,
        vault.token.symbol,
        vault.name.as_deref().unwrap_or("vault"),
        vault.address
    );
    eprintln!("Wallet: {}", wallet);
    eprintln!("Step 1/2: Approving {} {} for vault...", amount, vault.token.symbol);

    // Step 1: ERC-20 approve
    let approve_calldata = onchainos::encode_approve(&vault.address, amount_raw);
    let approve_result = onchainos::wallet_contract_call(
        chain_id,
        &vault.token.address,
        &approve_calldata,
        false,
    )?;

    let approve_ok = approve_result["ok"].as_bool().unwrap_or(false);
    if !approve_ok {
        anyhow::bail!("Approve failed: {}", approve_result);
    }
    let approve_tx = onchainos::extract_tx_hash(&approve_result);
    eprintln!("Approve tx: {}", approve_tx);

    // Wait 3 seconds for approve to confirm
    eprintln!("Waiting 3s for approve to confirm...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Step 2: ERC-4626 deposit
    eprintln!("Step 2/2: Depositing into vault...");
    let deposit_calldata = onchainos::encode_deposit(amount_raw, &wallet);
    let deposit_result = onchainos::wallet_contract_call(
        chain_id,
        &vault.address,
        &deposit_calldata,
        false,
    )?;

    let deposit_ok = deposit_result["ok"].as_bool().unwrap_or(false);
    if !deposit_ok {
        anyhow::bail!("Deposit failed: {}", deposit_result);
    }
    let deposit_tx = onchainos::extract_tx_hash(&deposit_result);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "data": {
                "vault": vault.address,
                "vault_name": vault.name.as_deref().unwrap_or(""),
                "token": vault.token.symbol,
                "amount": amount,
                "amount_raw": amount_raw.to_string(),
                "wallet": wallet,
                "approve_tx": approve_tx,
                "deposit_tx": deposit_tx,
                "explorer": format!("https://etherscan.io/tx/{}", deposit_tx)
            }
        }))?
    );
    Ok(())
}
