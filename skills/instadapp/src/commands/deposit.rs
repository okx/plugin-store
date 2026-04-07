// deposit command — deposit ETH into Instadapp Lite iETH v1 vault
// Uses supplyEth(address to_) with ETH value (payable)
// iETH v2 (stETH vault) requires ERC-20 approve + ERC-4626 deposit

use crate::{config, onchainos};
use anyhow::Result;
use serde_json::json;
use std::time::Duration;

pub async fn execute(
    chain_id: u64,
    vault_query: Option<&str>,
    amount: &str,
    dry_run: bool,
    wallet_override: Option<&str>,
) -> Result<()> {
    let (vault_addr, vault_info) = config::resolve_vault_address(vault_query);

    // Parse amount
    let amount_f: f64 = amount
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid amount: {}. Expected a number like 0.0001", amount))?;

    if amount_f <= 0.0 {
        anyhow::bail!("Amount must be positive, got: {}", amount);
    }

    // dry_run guard BEFORE resolve_wallet (onchainos may not be available)
    if dry_run {
        if vault_info.version == "v1" {
            let amount_wei = (amount_f * 1e18) as u64;
            let calldata = onchainos::encode_supply_eth("0x0000000000000000000000000000000000000000");
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "dry_run": true,
                    "data": {
                        "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                    },
                    "vault": vault_addr,
                    "vault_name": vault_info.name,
                    "vault_symbol": vault_info.symbol,
                    "deposit_token": "ETH (native)",
                    "amount_eth": amount,
                    "amount_wei": amount_wei.to_string(),
                    "calldata": calldata,
                    "selector": "0x87ee9312",
                    "note": "supplyEth(address) — sends ETH directly to vault. No ERC-20 approval needed."
                }))?
            );
        } else {
            // v2: stETH deposit
            let amount_raw = (amount_f * 1e18) as u128;
            let approve_calldata = onchainos::encode_approve(vault_addr, amount_raw);
            let deposit_calldata = onchainos::encode_deposit_v2(
                amount_raw,
                "0x0000000000000000000000000000000000000000",
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "dry_run": true,
                    "data": {
                        "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                    },
                    "vault": vault_addr,
                    "vault_name": vault_info.name,
                    "vault_symbol": vault_info.symbol,
                    "deposit_token": "stETH",
                    "steth_contract": config::STETH_ADDRESS,
                    "amount_steth": amount,
                    "amount_raw": amount_raw.to_string(),
                    "steps": [
                        {
                            "step": 1,
                            "action": "ERC-20 approve stETH",
                            "to": config::STETH_ADDRESS,
                            "calldata": approve_calldata,
                            "selector": "0x095ea7b3"
                        },
                        {
                            "step": 2,
                            "action": "ERC-4626 deposit stETH into iETHv2",
                            "to": vault_addr,
                            "calldata": deposit_calldata,
                            "selector": "0x6e553f65"
                        }
                    ]
                }))?
            );
        }
        return Ok(());
    }

    // Resolve wallet (after dry_run guard)
    let wallet = if let Some(w) = wallet_override {
        w.to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    if vault_info.version == "v1" {
        // iETH v1: supplyEth(address to_) with ETH value
        let amount_wei = (amount_f * 1e18) as u64;

        eprintln!(
            "Depositing {} ETH into {} ({}) via supplyEth()",
            amount,
            vault_info.name,
            vault_addr
        );
        eprintln!("Wallet: {}", wallet);
        eprintln!("Amount: {} ETH ({} wei)", amount, amount_wei);

        let calldata = onchainos::encode_supply_eth(&wallet);
        let result = onchainos::wallet_contract_call(
            chain_id,
            vault_addr,
            &calldata,
            Some(amount_wei),
            false,
        )?;

        let ok = result["ok"].as_bool().unwrap_or(false);
        if !ok {
            anyhow::bail!("Deposit failed: {}", result);
        }
        let tx_hash = onchainos::extract_tx_hash(&result);

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "data": {
                    "vault": vault_addr,
                    "vault_name": vault_info.name,
                    "vault_symbol": vault_info.symbol,
                    "deposit_token": "ETH",
                    "amount_eth": amount,
                    "amount_wei": amount_wei.to_string(),
                    "wallet": wallet,
                    "txHash": tx_hash,
                    "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
                }
            }))?
        );
    } else {
        // iETH v2: ERC-20 approve stETH → ERC-4626 deposit
        let amount_raw = (amount_f * 1e18) as u128;

        eprintln!(
            "Depositing {} stETH into {} ({}) via ERC-4626",
            amount,
            vault_info.name,
            vault_addr
        );
        eprintln!("Wallet: {}", wallet);
        eprintln!("Step 1/2: Approving {} stETH for vault...", amount);

        // Step 1: approve stETH
        let approve_calldata = onchainos::encode_approve(vault_addr, amount_raw);
        let approve_result = onchainos::wallet_contract_call(
            chain_id,
            config::STETH_ADDRESS,
            &approve_calldata,
            None,
            false,
        )?;

        let approve_ok = approve_result["ok"].as_bool().unwrap_or(false);
        if !approve_ok {
            anyhow::bail!("Approve failed: {}", approve_result);
        }
        let approve_tx = onchainos::extract_tx_hash(&approve_result);
        eprintln!("Approve tx: {}", approve_tx);

        // Wait for approve to confirm
        eprintln!("Waiting 3s for approve to confirm...");
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Step 2: ERC-4626 deposit
        eprintln!("Step 2/2: Depositing stETH into iETHv2 vault...");
        let deposit_calldata = onchainos::encode_deposit_v2(amount_raw, &wallet);
        let deposit_result = onchainos::wallet_contract_call(
            chain_id,
            vault_addr,
            &deposit_calldata,
            None,
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
                    "vault": vault_addr,
                    "vault_name": vault_info.name,
                    "vault_symbol": vault_info.symbol,
                    "deposit_token": "stETH",
                    "amount_steth": amount,
                    "amount_raw": amount_raw.to_string(),
                    "wallet": wallet,
                    "approve_txHash": approve_tx,
                    "deposit_txHash": deposit_tx,
                    "explorer": format!("https://etherscan.io/tx/{}", deposit_tx)
                }
            }))?
        );
    }

    Ok(())
}
