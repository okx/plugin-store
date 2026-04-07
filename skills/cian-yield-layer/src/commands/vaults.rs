//! vaults — list all CIAN Yield Layer vaults with APY and TVL from REST API.

use crate::api::fetch_pools;

pub async fn run() -> anyhow::Result<()> {
    println!("=== CIAN Yield Layer — Vaults (Ethereum Mainnet) ===");
    println!();

    let pools = fetch_pools().await?;

    if pools.is_empty() {
        // API unavailable — show hardcoded vault info so users still know what vaults exist
        println!("[info] CIAN API unavailable. Showing known vault addresses:");
        println!();
        println!("Vault: ylstETH (Leveraged Staking - stETH/ETH)");
        println!("  Address: {}", crate::config::VAULT_YLSTETH);
        println!("  Accepted tokens: ETH, WETH, stETH, wstETH, eETH, weETH");
        println!();
        println!("Vault: ylpumpBTC (Leveraged BTC Staking)");
        println!("  Address: {}", crate::config::VAULT_YLPUMPBTC);
        println!("  Accepted tokens: pumpBTC, WBTC");
        println!();
        println!("Note: APY/TVL data unavailable (CIAN API offline). Check https://cian.app for live data.");
        println!("Note: Withdrawal is ASYNC (~{} days). Use 'request-redeem' to initiate.", crate::config::WITHDRAWAL_WAIT_DAYS);
        return Ok(());
    }

    for pool in &pools {
        let apy_7: f64 = pool.apy_7.parse().unwrap_or(0.0);
        let apy_instant: f64 = pool.apy_instant_7.parse().unwrap_or(0.0);
        let apy_points: f64 = pool.apy_esti_by_points_7.parse().unwrap_or(0.0);
        let apy_eco: f64 = pool.apy_eco_earn_7.parse().unwrap_or(0.0);
        let tvl_usd: f64 = pool.tvl_usd.parse().unwrap_or(0.0);
        let net_tvl: f64 = pool.net_tvl_usd.parse().unwrap_or(0.0);

        println!("Vault: {}", pool.pool_name);
        println!("  Address:        {}", pool.pool_address);
        println!("  APY (7d):       {:.4}%", apy_7);
        println!("    Base APY:     {:.4}%", apy_instant);
        println!("    Points APY:   {:.4}%", apy_points);
        println!("    Eco Earn APY: {:.4}%", apy_eco);
        println!("  TVL (USD):      ${:.2}", tvl_usd);
        println!("  Net TVL (USD):  ${:.2}", net_tvl);
        println!();

        // Show accepted tokens
        if pool.pool_address.to_lowercase() == crate::config::VAULT_YLSTETH.to_lowercase() {
            println!("  Accepted tokens: ETH, WETH, stETH, wstETH, eETH, weETH");
        } else if pool.pool_address.to_lowercase() == crate::config::VAULT_YLPUMPBTC.to_lowercase() {
            println!("  Accepted tokens: pumpBTC, WBTC");
        }
        println!("  ---");
    }

    println!();
    println!("Note: Withdrawal is ASYNC (~{} days). Use 'request-redeem' to initiate.", crate::config::WITHDRAWAL_WAIT_DAYS);
    println!("      Instant withdraw() / redeem() are DISABLED in CIAN vaults.");

    Ok(())
}
