use crate::config;
use crate::onchainos;
use serde_json::Value;

/// Query current sfrxETH APR and exchange rate.
pub async fn run() -> anyhow::Result<()> {
    // Fetch from Frax Finance API
    let url = format!("{}/v2/frxeth/summary/history?range=1d", config::FRAX_API_URL);
    let client = reqwest::Client::new();
    let resp: Value = client.get(&url).send().await?.json().await?;

    let items = resp.get("items").and_then(|v| v.as_array());
    let latest = if let Some(arr) = items {
        arr.first().cloned().unwrap_or(Value::Null)
    } else {
        // The API may return an array directly
        if resp.is_array() {
            resp.as_array().and_then(|a| a.first().cloned()).unwrap_or(Value::Null)
        } else {
            resp.clone()
        }
    };

    let sfrxeth_apr = latest.get("sfrxethApr").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let sfrxeth_frxeth_price = latest
        .get("sfrxethFrxethPrice")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let eth_price_usd = latest
        .get("ethPriceUsd")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let frxeth_eth_price = latest
        .get("frxethEthCurvePrice")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    // Also call convertToAssets on-chain for precision
    let one_ether_hex = format!("0x{:064x}", 1_000_000_000_000_000_000u128);
    let calldata = format!("0x{}{}", config::SEL_CONVERT_TO_ASSETS, &one_ether_hex[2..]);
    let on_chain_rate = onchainos::eth_call(config::CHAIN_ID, config::SFRXETH_VAULT, &calldata)
        .map(|r| {
            let raw = onchainos::decode_uint256(&r);
            raw as f64 / 1e18
        })
        .unwrap_or(sfrxeth_frxeth_price);

    // totalAssets
    let total_assets_call = format!("0x{}", config::SEL_TOTAL_ASSETS);
    let total_assets_wei = onchainos::eth_call(config::CHAIN_ID, config::SFRXETH_VAULT, &total_assets_call)
        .map(|r| onchainos::decode_uint256(&r))
        .unwrap_or(0);
    let total_assets_eth = total_assets_wei as f64 / 1e18;

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "data": {
                "sfrxeth_apr_pct": format!("{:.4}", sfrxeth_apr),
                "sfrxeth_per_frxeth": format!("{:.8}", on_chain_rate),
                "frxeth_per_eth_curve": format!("{:.6}", frxeth_eth_price),
                "total_assets_frxeth": format!("{:.4}", total_assets_eth),
                "eth_price_usd": format!("{:.2}", eth_price_usd),
                "chain": "ethereum",
                "contracts": {
                    "sfrxETH": config::SFRXETH_VAULT,
                    "frxETH": config::FRXETH_TOKEN
                }
            }
        })
    );
    Ok(())
}
