use crate::{config, onchainos, rpc};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
struct YieldEntry {
    #[serde(rename = "lastUpdated")]
    last_updated: Option<String>,
    value: Option<f64>,
}

#[derive(Deserialize, Debug)]
struct YieldResponse {
    #[serde(rename = "stakingYield")]
    staking_yield: Option<YieldEntry>,
    #[serde(rename = "avg30dSusdeYield")]
    avg30d_susde_yield: Option<YieldEntry>,
    #[serde(rename = "avg90dSusdeYield")]
    avg90d_susde_yield: Option<YieldEntry>,
    #[serde(rename = "protocolYield")]
    protocol_yield: Option<YieldEntry>,
}

pub async fn run() -> anyhow::Result<()> {
    // Fetch yield data from Ethena API
    let client = reqwest::Client::builder()
        .build()
        .unwrap_or_default();
    let resp = client
        .get(config::YIELD_API_URL)
        .send()
        .await?;
    let yield_data: Value = resp.json().await?;

    // Parse yield fields
    let staking_yield = yield_data["stakingYield"]["value"].as_f64().unwrap_or(0.0);
    let avg30d = yield_data["avg30dSusdeYield"]["value"].as_f64().unwrap_or(0.0);
    let avg90d = yield_data["avg90dSusdeYield"]["value"].as_f64().unwrap_or(0.0);
    let protocol_yield = yield_data["protocolYield"]["value"].as_f64().unwrap_or(0.0);
    let last_updated = yield_data["stakingYield"]["lastUpdated"]
        .as_str()
        .unwrap_or("unknown");

    // Fetch exchange rate: convertToAssets(1e18) = USDe per 1 sUSDe
    let one_susde: u128 = 1_000_000_000_000_000_000;
    let calldata = rpc::calldata_convert_to_assets(one_susde);
    let exchange_result = onchainos::eth_call(config::CHAIN_ID, config::SUSDE_ADDRESS, &calldata)?;
    let exchange_hex = rpc::extract_return_data(&exchange_result).unwrap_or_default();
    let exchange_rate = rpc::decode_uint256(&exchange_hex).unwrap_or(one_susde);
    let exchange_rate_float = rpc::wei_to_float(exchange_rate);

    // Fetch total assets for TVL
    let total_calldata = rpc::calldata_total_assets();
    let total_result = onchainos::eth_call(config::CHAIN_ID, config::SUSDE_ADDRESS, &total_calldata)?;
    let total_hex = rpc::extract_return_data(&total_result).unwrap_or_default();
    let total_assets = rpc::decode_uint256(&total_hex).unwrap_or(0);
    let total_assets_float = rpc::wei_to_float(total_assets);

    // Fetch cooldown duration
    let cd_calldata = rpc::calldata_cooldown_duration();
    let cd_result = onchainos::eth_call(config::CHAIN_ID, config::SUSDE_ADDRESS, &cd_calldata)?;
    let cd_hex = rpc::extract_return_data(&cd_result).unwrap_or_default();
    let cooldown_secs = rpc::decode_uint256(&cd_hex).unwrap_or(86400);
    let cooldown_days = cooldown_secs as f64 / 86400.0;

    println!("=== USDe Staking Rates (Ethena sUSDe) ===");
    println!();
    println!("Yield:");
    println!("  Current staking APY:  {:.2}%", staking_yield);
    println!("  Protocol yield:       {:.2}%", protocol_yield);
    println!("  30-day avg APY:       {:.2}%", avg30d);
    println!("  90-day avg APY:       {:.2}%", avg90d);
    println!("  Last updated:         {}", last_updated);
    println!();
    println!("Exchange Rate:");
    println!("  1 sUSDe = {:.6} USDe", exchange_rate_float);
    println!("  (sUSDe appreciates against USDe over time)");
    println!();
    println!("Protocol Info:");
    println!("  Total Value Locked:   {:.2} USDe", total_assets_float);
    println!("  Cooldown period:      {:.0} day(s)", cooldown_days);
    println!("  Chain:                Ethereum mainnet");
    println!("  sUSDe contract:       {}", config::SUSDE_ADDRESS);
    println!("  USDe contract:        {}", config::USDE_ADDRESS);

    Ok(())
}
