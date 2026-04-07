// withdraw command — withdraw from Instadapp Lite vaults
// iETH v1: withdraw(uint256 amount_, address to_) — burns iETH shares
// iETH v2: redeem(uint256 shares_, address receiver_, address owner_) — ERC-4626

use crate::{config, onchainos, rpc};
use anyhow::Result;
use serde_json::json;

pub async fn execute(
    chain_id: u64,
    vault_query: Option<&str>,
    shares_amount: Option<&str>, // None = withdraw all
    dry_run: bool,
    wallet_override: Option<&str>,
) -> Result<()> {
    let (vault_addr, vault_info) = config::resolve_vault_address(vault_query);
    let rpc_url = config::ETHEREUM_RPC;

    // dry_run guard BEFORE resolve_wallet
    if dry_run {
        let shares_raw: u128 = match shares_amount {
            Some(s) => {
                let sf: f64 = s
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid shares amount: {}", s))?;
                (sf * 1e18) as u128
            }
            None => u128::MAX, // redeem all
        };

        let placeholder = "0x0000000000000000000000000000000000000000";
        let calldata = if vault_info.version == "v1" {
            onchainos::encode_withdraw_v1(shares_raw, placeholder)
        } else {
            onchainos::encode_redeem_v2(shares_raw, placeholder, placeholder)
        };
        let selector = if vault_info.version == "v1" {
            "0x00f714ce"
        } else {
            "0xba087652"
        };

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
                "shares": shares_amount.unwrap_or("all"),
                "shares_raw": shares_raw.to_string(),
                "calldata": calldata,
                "selector": selector
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

    // Determine shares to withdraw
    let shares_raw: u128 = match shares_amount {
        Some(s) => {
            let sf: f64 = s
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid shares amount: {}", s))?;
            (sf * 1e18) as u128
        }
        None => {
            // Withdraw all: query current balance
            let balance = rpc::get_balance_of(vault_addr, &wallet, rpc_url).await?;
            if balance == 0 {
                anyhow::bail!(
                    "No {} shares held for wallet {}. Use 'instadapp positions' to check.",
                    vault_info.symbol,
                    wallet
                );
            }
            balance
        }
    };

    let shares_display = format!("{:.6}", shares_raw as f64 / 1e18);
    eprintln!(
        "Withdrawing {} {} shares from {} ({})",
        shares_display,
        vault_info.symbol,
        vault_info.name,
        vault_addr
    );
    eprintln!("Wallet: {}", wallet);

    let (calldata, selector) = if vault_info.version == "v1" {
        (
            onchainos::encode_withdraw_v1(shares_raw, &wallet),
            "0x00f714ce",
        )
    } else {
        (
            onchainos::encode_redeem_v2(shares_raw, &wallet, &wallet),
            "0xba087652",
        )
    };

    let result = onchainos::wallet_contract_call(chain_id, vault_addr, &calldata, None, false)?;

    let ok = result["ok"].as_bool().unwrap_or(false);
    if !ok {
        anyhow::bail!("Withdraw failed: {}", result);
    }
    let tx_hash = onchainos::extract_tx_hash(&result);

    let underlying = if vault_info.version == "v1" { "ETH" } else { "stETH" };

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "data": {
                "vault": vault_addr,
                "vault_name": vault_info.name,
                "vault_symbol": vault_info.symbol,
                "shares_redeemed": shares_display,
                "underlying_token": underlying,
                "wallet": wallet,
                "selector": selector,
                "txHash": tx_hash,
                "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
            }
        }))?
    );
    Ok(())
}
