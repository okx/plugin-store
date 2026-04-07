// withdraw command — redeem shares from a Yearn ERC-4626 vault
// Uses ERC-4626 redeem(uint256 shares, address receiver, address owner)

use crate::{api, config, onchainos, rpc};
use anyhow::Result;
use serde_json::json;

pub async fn execute(
    chain_id: u64,
    vault_query: &str,
    shares_amount: Option<&str>, // None = redeem all
    dry_run: bool,
    wallet_override: Option<&str>,
) -> Result<()> {
    let rpc_url = if chain_id == 1 {
        config::ETHEREUM_RPC
    } else {
        "https://ethereum.publicnode.com"
    };

    // dry_run guard BEFORE resolve_wallet
    if dry_run {
        let vaults = api::fetch_vaults(chain_id).await?;
        let vault = api::find_vault_by_address_or_symbol(&vaults, vault_query)
            .ok_or_else(|| anyhow::anyhow!("Vault not found for query: {}", vault_query))?;

        let decimals = vault.decimals.unwrap_or(18);
        let shares_raw: u128 = match shares_amount {
            Some(s) => {
                let sf: f64 = s.parse().map_err(|_| anyhow::anyhow!("Invalid shares amount: {}", s))?;
                (sf * 10f64.powi(decimals as i32)) as u128
            }
            None => u128::MAX, // redeem all
        };

        let placeholder = "0x0000000000000000000000000000000000000000";
        let redeem_calldata = onchainos::encode_redeem(shares_raw, placeholder, placeholder);

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
                "shares": shares_amount.unwrap_or("all"),
                "calldata": redeem_calldata,
                "selector": "0xba087652"
            }))?
        );
        return Ok(());
    }

    // Resolve wallet (after dry_run guard)
    let wallet = if let Some(w) = wallet_override {
        w.to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    let vaults = api::fetch_vaults(chain_id).await?;
    let vault = api::find_vault_by_address_or_symbol(&vaults, vault_query)
        .ok_or_else(|| anyhow::anyhow!("Vault not found for query: {}. Use 'vaults' command to list available vaults.", vault_query))?;

    let decimals = vault.decimals.unwrap_or(18);

    // Determine shares to redeem
    let shares_raw: u128 = match shares_amount {
        Some(s) => {
            let sf: f64 = s.parse().map_err(|_| anyhow::anyhow!("Invalid shares amount: {}", s))?;
            (sf * 10f64.powi(decimals as i32)) as u128
        }
        None => {
            // Redeem all: query current shares balance
            let balance = rpc::get_balance_of(&vault.address, &wallet, rpc_url).await?;
            if balance == 0 {
                anyhow::bail!(
                    "No shares held in vault {} for wallet {}",
                    vault.name.as_deref().unwrap_or(&vault.address),
                    wallet
                );
            }
            balance
        }
    };

    let shares_display = format!("{:.6}", shares_raw as f64 / 10f64.powi(decimals as i32));

    eprintln!(
        "Withdrawing {} shares from {} ({})",
        shares_display,
        vault.name.as_deref().unwrap_or("vault"),
        vault.address
    );
    eprintln!("Wallet: {}", wallet);

    let redeem_calldata = onchainos::encode_redeem(shares_raw, &wallet, &wallet);
    let result = onchainos::wallet_contract_call(
        chain_id,
        &vault.address,
        &redeem_calldata,
        false,
    )?;

    let ok = result["ok"].as_bool().unwrap_or(false);
    if !ok {
        anyhow::bail!("Redeem failed: {}", result);
    }
    let tx_hash = onchainos::extract_tx_hash(&result);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "data": {
                "vault": vault.address,
                "vault_name": vault.name.as_deref().unwrap_or(""),
                "token": vault.token.symbol,
                "shares_redeemed": shares_display,
                "wallet": wallet,
                "txHash": tx_hash,
                "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
            }
        }))?
    );
    Ok(())
}
