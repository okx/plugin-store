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

    println!("Fetching ether.fi positions for wallet: {}", owner);

    // Parallel fetch: balances
    let (eeth_balance, weeth_balance) = tokio::join!(
        get_balance(eeth, &owner, rpc),
        get_balance(weeth, &owner, rpc),
    );
    let eeth_balance = eeth_balance.unwrap_or(0);
    let weeth_balance = weeth_balance.unwrap_or(0);

    // Exchange rate: weETH → eETH (getRate())
    let exchange_rate = crate::rpc::weeth_get_rate(weeth, rpc).await.ok();
    let rate = exchange_rate.unwrap_or(0.0);

    // Protocol stats + ETH price (non-fatal)
    let (stats, eth_price_usd) = tokio::join!(
        fetch_stats(),
        crate::api::fetch_eth_price(),
    );
    let stats = stats.unwrap_or(crate::api::EtherFiStats { apy: None, tvl: None });

    // --- Derived values ---
    let eeth_f64      = eeth_balance  as f64 / 1e18;
    let weeth_f64     = weeth_balance as f64 / 1e18;
    let weeth_as_eeth = weeth_f64 * rate;
    let total_eeth    = eeth_f64 + weeth_as_eeth;

    // --- Human-readable output ---
    const SEP_W_USD:  &str = "─────────────────────────────────────────────────────────────";
    const SEP_NO_USD: &str = "────────────────────────────────────────────────";

    println!("\nether.fi Positions");
    println!("  Wallet: {}", owner);

    if let Some(price) = eth_price_usd {
        // Full table with USD column
        println!("{}", SEP_W_USD);
        println!("{:<10} {:>14} {:>14} {:>14}", "Token", "Balance", "As eETH", "USD Value");
        println!("{}", SEP_W_USD);
        println!(
            "{:<10} {:>14.6} {:>14.6} {:>14}",
            "eETH", eeth_f64, eeth_f64,
            format!("${:.2}", eeth_f64 * price)
        );
        println!(
            "{:<10} {:>14.6} {:>14.6} {:>14}",
            "weETH", weeth_f64, weeth_as_eeth,
            format!("${:.2}", weeth_as_eeth * price)
        );
        println!("{}", SEP_W_USD);
        println!(
            "{:<10} {:>14} {:>14.6} {:>14}",
            "Total", "", total_eeth,
            format!("${:.2}", total_eeth * price)
        );
    } else {
        // Narrower table without USD column
        println!("{}", SEP_NO_USD);
        println!("{:<10} {:>14} {:>14}", "Token", "Balance", "As eETH");
        println!("{}", SEP_NO_USD);
        println!("{:<10} {:>14.6} {:>14.6}", "eETH",  eeth_f64,  eeth_f64);
        println!("{:<10} {:>14.6} {:>14.6}", "weETH", weeth_f64, weeth_as_eeth);
        println!("{}", SEP_NO_USD);
        println!("{:<10} {:>14} {:>14.6}", "Total", "", total_eeth);
    }

    println!("\nProtocol Stats:");
    match exchange_rate {
        Some(r) => println!("  weETH/eETH rate:  {:.8}", r),
        None    => println!("  weETH/eETH rate:  N/A"),
    }
    match stats.apy {
        Some(v) => println!("  APY:              {:.2}%", v),
        None    => println!("  APY:              N/A"),
    }
    match stats.tvl {
        Some(v) => println!("  TVL:              ${:.0}", v),
        None    => println!("  TVL:              N/A"),
    }
    if let Some(p) = eth_price_usd {
        println!("  ETH price:        ${:.2}", p);
    }

    Ok(())
}
