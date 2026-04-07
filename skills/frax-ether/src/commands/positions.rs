use crate::config;
use crate::onchainos;
use clap::Args;
use serde_json::Value;

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query (defaults to current logged-in wallet)
    #[arg(long)]
    pub address: Option<String>,

    /// Chain ID (only Ethereum mainnet supported)
    #[arg(long, default_value = "1")]
    pub chain: u64,
}

/// Query frxETH and sfrxETH balances for a wallet.
pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let wallet = if let Some(addr) = args.address {
        addr
    } else {
        onchainos::resolve_wallet(args.chain)?
    };

    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --address or ensure onchainos is logged in.");
    }

    let wallet_clean = wallet.trim_start_matches("0x");
    let wallet_padded = format!("{:0>64}", wallet_clean);

    // Query frxETH balance
    let frxeth_calldata = format!("0x{}{}", config::SEL_BALANCE_OF, wallet_padded);
    let frxeth_raw = onchainos::eth_call(config::CHAIN_ID, config::FRXETH_TOKEN, &frxeth_calldata)
        .map(|r| onchainos::decode_uint256(&r))
        .unwrap_or(0);
    let frxeth_balance = frxeth_raw as f64 / 1e18;

    // Query sfrxETH balance
    let sfrxeth_calldata = format!("0x{}{}", config::SEL_BALANCE_OF, wallet_padded);
    let sfrxeth_raw = onchainos::eth_call(config::CHAIN_ID, config::SFRXETH_VAULT, &sfrxeth_calldata)
        .map(|r| onchainos::decode_uint256(&r))
        .unwrap_or(0);
    let sfrxeth_balance = sfrxeth_raw as f64 / 1e18;

    // Convert sfrxETH balance to frxETH value using convertToAssets
    let sfrxeth_frxeth_value = if sfrxeth_raw > 0 {
        let shares_hex = format!("{:064x}", sfrxeth_raw);
        let convert_calldata = format!("0x{}{}", config::SEL_CONVERT_TO_ASSETS, shares_hex);
        let frxeth_value_raw = onchainos::eth_call(config::CHAIN_ID, config::SFRXETH_VAULT, &convert_calldata)
            .map(|r| onchainos::decode_uint256(&r))
            .unwrap_or(0);
        frxeth_value_raw as f64 / 1e18
    } else {
        0.0
    };

    // Get ETH price for USD estimation
    let url = format!("{}/v2/frxeth/summary/history?range=1d", config::FRAX_API_URL);
    let client = reqwest::Client::new();
    let eth_price_usd = match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<Value>().await {
                let items = json.get("items").and_then(|v| v.as_array());
                let latest = items.and_then(|a| a.first()).unwrap_or(&Value::Null);
                latest.get("ethPriceUsd").and_then(|v| v.as_f64()).unwrap_or(2000.0)
            } else {
                2000.0
            }
        }
        Err(_) => 2000.0,
    };

    let total_frxeth = frxeth_balance + sfrxeth_frxeth_value;
    let total_usd = total_frxeth * eth_price_usd;

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet,
                "frxETH": {
                    "balance": format!("{:.8}", frxeth_balance),
                    "usd_value": format!("{:.2}", frxeth_balance * eth_price_usd)
                },
                "sfrxETH": {
                    "balance": format!("{:.8}", sfrxeth_balance),
                    "frxeth_value": format!("{:.8}", sfrxeth_frxeth_value),
                    "usd_value": format!("{:.2}", sfrxeth_frxeth_value * eth_price_usd)
                },
                "total_frxeth_equivalent": format!("{:.8}", total_frxeth),
                "total_usd_value": format!("{:.2}", total_usd),
                "chain": "ethereum"
            }
        })
    );
    Ok(())
}
