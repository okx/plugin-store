//! positions — user's share balance + underlying value via REST API + on-chain balance.

use crate::api::fetch_user_position;
use crate::config::{CHAIN_ID, VAULTS, format_18, format_units};
use crate::onchainos::resolve_wallet;
use crate::rpc::{balance_of, convert_to_assets, exchange_price};

pub async fn run(wallet_override: Option<&str>) -> anyhow::Result<()> {
    println!("=== CIAN Yield Layer — Positions ===");
    println!("Chain: Ethereum Mainnet (chain 1)");
    println!();

    let wallet = if let Some(w) = wallet_override {
        w.to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    if wallet.is_empty() {
        anyhow::bail!("No wallet found on Ethereum (chain 1). Run: onchainos wallet login");
    }
    println!("Wallet: {}", wallet);
    println!();

    for vault in VAULTS {
        println!("── {} ({}) ──", vault.name, vault.symbol);
        println!("   Vault: {}", vault.address);

        // On-chain share balance (authoritative)
        let shares = balance_of(vault.address, &wallet).await.unwrap_or(0);
        println!("   On-chain shares: {} {}", format_18(shares), vault.symbol);

        if shares > 0 {
            let assets = convert_to_assets(vault.address, shares).await.unwrap_or(0);
            let asset_label = if vault.asset_decimals == 18 { "WETH" } else { "pumpBTC" };
            println!(
                "   Underlying value: {} {}",
                format_units(assets, vault.asset_decimals),
                asset_label
            );

            if let Ok(price) = exchange_price(vault.address).await {
                let rate = (price as f64) / 1e18;
                println!("   Exchange rate: {:.8}", rate);
            }
        }

        // REST API for additional position data (pending redeems, USD value, points)
        match fetch_user_position(vault.address, &wallet).await {
            Ok(pos_data) => {
                let data = &pos_data["data"];
                if !data.is_null() {
                    if let Some(usd) = data["share_value_usd"].as_str() {
                        if !usd.is_empty() && usd != "0" {
                            println!("   USD value: ${}", usd);
                        }
                    }
                    if let Some(pending) = data["pending_redeem_shares"].as_str() {
                        if !pending.is_empty() && pending != "0" {
                            println!("   Pending redeem shares: {} (processing, ~5 day wait)", pending);
                        }
                    }
                }
            }
            Err(e) => {
                // REST API failure is non-fatal; on-chain data is sufficient
                eprintln!("   Note: REST API unavailable ({}), showing on-chain data only.", e);
            }
        }

        if shares == 0 {
            println!("   No position in this vault.");
        }
        println!();
    }

    println!("Note: Pending redeems show shares transferred to rebalancer (~5 day processing time).");

    Ok(())
}
