use clap::Args;
use crate::api::fetch_stats;
use crate::config::{eeth_address, rpc_url, weeth_address, CHAIN_ID};
use crate::onchainos::resolve_wallet;
use crate::rpc::get_balance;

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub owner: Option<String>,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let eeth = eeth_address();
    let weeth = weeth_address();

    // Resolve wallet address
    let owner = match args.owner {
        Some(addr) => addr,
        None => resolve_wallet(CHAIN_ID)?,
    };

    // Parallel fetch: balances — fail-fast, 0 would be misleading if RPC is down
    let (eeth_result, weeth_result) = tokio::join!(
        get_balance(eeth, &owner, rpc),
        get_balance(weeth, &owner, rpc),
    );
    let eeth_balance = eeth_result
        .map_err(|e| anyhow::anyhow!("Failed to fetch eETH balance: {}", e))?;
    let weeth_balance = weeth_result
        .map_err(|e| anyhow::anyhow!("Failed to fetch weETH balance: {}", e))?;

    // Exchange rate: weETH → eETH — required for meaningful totals
    let rate = crate::rpc::weeth_get_rate(weeth, rpc).await
        .map_err(|e| anyhow::anyhow!("Failed to fetch weETH exchange rate: {}", e))?;
    if rate == 0.0 {
        anyhow::bail!(
            "weETH exchange rate returned 0 — RPC may be unavailable. \
             Check https://ethereum-rpc.publicnode.com connectivity."
        );
    }

    // Protocol stats + ETH price (non-fatal — external API may be unavailable)
    let (stats, eth_price_usd) = tokio::join!(
        fetch_stats(),
        crate::api::fetch_eth_price(),
    );
    let stats = stats.unwrap_or(crate::api::EtherFiStats { apy: None, tvl: None });

    // Derived values
    let eeth_f64      = eeth_balance as f64 / 1e18;
    let weeth_f64     = weeth_balance as f64 / 1e18;
    let weeth_as_eeth = weeth_f64 * rate;
    let total_eeth    = eeth_f64 + weeth_as_eeth;
    let total_usd     = eth_price_usd.map(|p| total_eeth * p);

    println!(
        "{}",
        serde_json::json!({
            "ok":               true,
            "wallet":           owner,
            "eeth_balance":     format!("{:.6}", eeth_f64),
            "eeth_balance_raw": eeth_balance.to_string(),
            "weeth_balance":    format!("{:.6}", weeth_f64),
            "weeth_balance_raw": weeth_balance.to_string(),
            "weeth_as_eeth":    format!("{:.6}", weeth_as_eeth),
            "total_eeth":       format!("{:.6}", total_eeth),
            "total_usd":        total_usd.map(|v| format!("{:.2}", v)),
            "rate":             format!("{:.8}", rate),
            "apy_pct":          stats.apy.map(|v| format!("{:.2}", v)),
            "tvl_usd":          stats.tvl.map(|v| format!("{:.0}", v)),
            "eth_price_usd":    eth_price_usd.map(|v| format!("{:.2}", v)),
        })
    );

    Ok(())
}
