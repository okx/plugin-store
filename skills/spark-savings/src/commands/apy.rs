use anyhow::Context;
use serde_json::{json, Value};

use crate::config::{get_chain_config, ETHEREUM_RPC, MAKER_POT};
use crate::rpc;

/// Read the Sky Savings Rate (SSR) from Ethereum sUSDS contract.
/// ssr() returns a per-second rate in ray (1e27) format.
async fn read_ssr_ethereum() -> anyhow::Result<u128> {
    // sUSDS on Ethereum: 0xa3931d71877C0E7a3148CB7Eb4463524FEc27fbD
    // ssr() selector: 0x03607ceb
    let result = rpc::eth_call(
        ETHEREUM_RPC,
        "0xa3931d71877C0E7a3148CB7Eb4463524FEc27fbD",
        "0x03607ceb",
    )
    .await
    .context("Failed to read SSR from Ethereum sUSDS")?;
    rpc::decode_u256(&result).context("Failed to decode SSR value")
}

/// Read the DAI Savings Rate (DSR) from MakerDAO Pot on Ethereum.
async fn read_dsr_ethereum() -> anyhow::Result<u128> {
    // Pot.dsr() selector: 0x487bf082
    let result = rpc::eth_call(ETHEREUM_RPC, MAKER_POT, "0x487bf082")
        .await
        .context("Failed to read DSR from Pot")?;
    rpc::decode_u256(&result).context("Failed to decode DSR value")
}

/// Read the SSR accumulator (chi / getConversionRate) from an L2 oracle.
/// Returns the conversion rate in 1e27 format (how many USDS per sUSDS).
async fn read_conversion_rate_l2(oracle: &str, rpc_url: &str) -> anyhow::Result<u128> {
    // getConversionRate() selector: 0xf36089ec
    let result = rpc::eth_call(rpc_url, oracle, "0xf36089ec")
        .await
        .context("Failed to read conversion rate from L2 oracle")?;
    rpc::decode_u256(&result).context("Failed to decode conversion rate")
}

/// Show the current Spark savings APY.
pub async fn run(chain_id: u64) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    // Always read SSR from Ethereum (canonical source)
    let ssr_ray = read_ssr_ethereum()
        .await
        .unwrap_or(1_000_000_001_167_363_430_498_603_315u128);
    let dsr_ray = read_dsr_ethereum()
        .await
        .unwrap_or(1_000_000_000_393_915_525_145_987_602u128);

    let ssr_apy = rpc::ray_to_apy(ssr_ray);
    let dsr_apy = rpc::ray_to_apy(dsr_ray);

    // On L2, also read the local conversion rate (chi accumulator)
    let (conversion_rate_str, susds_per_usds) = if let Some(oracle) = cfg.ssr_oracle {
        match read_conversion_rate_l2(oracle, cfg.rpc_url).await {
            Ok(rate) => {
                let rate_f64 = rate as f64 / 1e27;
                (format!("{:.6}", rate_f64), rate_f64)
            }
            Err(_) => ("1.000000".to_string(), 1.0),
        }
    } else {
        // On Ethereum, read chi from sUSDS directly
        // chi() selector: 0xc92aecc4
        let chi_result = rpc::eth_call(
            ETHEREUM_RPC,
            "0xa3931d71877C0E7a3148CB7Eb4463524FEc27fbD",
            "0xc92aecc4",
        )
        .await
        .unwrap_or_default();
        let chi = rpc::decode_u256(&chi_result).unwrap_or(1_000_000_000_000_000_000_000_000_000);
        let rate_f64 = chi as f64 / 1e27;
        (format!("{:.6}", rate_f64), rate_f64)
    };

    let usds_per_susds = if susds_per_usds > 0.0 {
        1.0 / susds_per_usds
    } else {
        1.0
    };

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "savings": {
            "ssrApy": format!("{:.4}%", ssr_apy * 100.0),
            "dsrApy": format!("{:.4}%", dsr_apy * 100.0),
            "ssrRaw": ssr_ray.to_string(),
            "dsrRaw": dsr_ray.to_string()
        },
        "conversionRate": {
            "usdsSPerSUSDS": conversion_rate_str,
            "sudsPerUSDS": format!("{:.6}", usds_per_susds)
        },
        "description": format!(
            "sUSDS earns {:.2}% APY (Sky Savings Rate). 1 sUSDS = {:.4} USDS.",
            ssr_apy * 100.0,
            usds_per_susds
        )
    }))
}
