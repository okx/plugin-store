use clap::Args;
use crate::api::fetch_stats;
use crate::config::{eeth_address, format_units, rpc_url, weeth_address, CHAIN_ID};
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

    // Fetch eETH balance (18 decimals)
    let eeth_balance = get_balance(eeth, &owner, rpc).await.unwrap_or(0);

    // Fetch weETH balance (18 decimals)
    let weeth_balance = get_balance(weeth, &owner, rpc).await.unwrap_or(0);

    // Convert weETH to eETH equivalent for display.
    // weETH.convertToAssets() reverts on this contract; use getRate() instead.
    let weeth_as_eeth = if weeth_balance > 0 {
        let rate = crate::rpc::weeth_get_rate(weeth, rpc).await.unwrap_or(0.0);
        (weeth_balance as f64 * rate) as u128
    } else {
        0
    };

    // Fetch protocol stats (APY, TVL) from DeFiLlama — non-fatal if unavailable
    let stats = fetch_stats().await.unwrap_or(crate::api::EtherFiStats {
        apy: None,
        tvl: None,
    });

    // Exchange rate from on-chain weETH.getRate() — more reliable than any API
    let exchange_rate = crate::rpc::weeth_get_rate(weeth, rpc).await.ok();

    let apy_str = match stats.apy {
        Some(v) => format!("{:.2}%", v),
        None => "N/A".to_string(),
    };

    let exchange_rate_str = match exchange_rate {
        Some(v) => format!("{:.6}", v),
        None => "N/A".to_string(),
    };

    let tvl_str = match stats.tvl {
        Some(v) => format!("${:.0}", v),
        None => "N/A".to_string(),
    };

    println!(
        concat!(
            "{{",
            "\"ok\":true,",
            "\"owner\":\"{owner}\",",
            "\"eETH\":{{\"balanceWei\":\"{eeth_wei}\",\"balance\":\"{eeth_fmt}\"}},",
            "\"weETH\":{{\"balanceWei\":\"{weeth_wei}\",\"balance\":\"{weeth_fmt}\",\"asEETH\":\"{weeth_as_eeth_fmt}\"}},",
            "\"protocol\":{{\"apy\":\"{apy}\",\"tvl\":\"{tvl}\",\"weETHtoEETH\":\"{rate}\"}}",
            "}}"
        ),
        owner = owner,
        eeth_wei = eeth_balance,
        eeth_fmt = format_units(eeth_balance, 18),
        weeth_wei = weeth_balance,
        weeth_fmt = format_units(weeth_balance, 18),
        weeth_as_eeth_fmt = format_units(weeth_as_eeth, 18),
        apy = apy_str,
        tvl = tvl_str,
        rate = exchange_rate_str,
    );

    Ok(())
}
