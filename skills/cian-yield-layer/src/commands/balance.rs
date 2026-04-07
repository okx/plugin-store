//! balance — check user's share balance and estimated underlying value in each vault.

use crate::config::{CHAIN_ID, VAULTS, format_18, format_units};
use crate::onchainos::resolve_wallet;
use crate::rpc::{balance_of, convert_to_assets, exchange_price};

pub async fn run(wallet_override: Option<&str>) -> anyhow::Result<()> {
    println!("=== CIAN Yield Layer — Balance Overview ===");
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
        println!("Vault: {} ({})", vault.name, vault.symbol);
        println!("  Address: {}", vault.address);

        let shares = balance_of(vault.address, &wallet).await.unwrap_or(0);
        println!("  Share balance: {} {}", format_18(shares), vault.symbol);

        if shares > 0 {
            // Convert shares to underlying assets
            let assets = convert_to_assets(vault.address, shares).await.unwrap_or(0);
            println!(
                "  Underlying value: {} ({})",
                format_units(assets, vault.asset_decimals),
                if vault.asset_decimals == 18 { "WETH equivalent" } else { "pumpBTC equivalent" }
            );

            // Show exchange rate
            if let Ok(price) = exchange_price(vault.address).await {
                let rate = (price as f64) / 1e18;
                println!("  Exchange rate: {:.8} (asset per share)", rate);
            }
        } else {
            println!("  No shares held.");
        }
        println!();
    }

    println!("To deposit:        cian-yield-layer deposit --vault ylsteth --token stETH --amount 1.0");
    println!("To request redeem: cian-yield-layer request-redeem --vault ylsteth --shares 1.0 --token stETH");
    println!("Note: Withdrawal is ASYNC (~5 days). Instant redeem is DISABLED.");

    Ok(())
}
