use serde_json::{json, Value};

use crate::config::{get_chain_config, ETHEREUM_RPC, MAKER_POT};
use crate::rpc;

/// Show Spark savings market info: TVL, rates, conversion rate.
pub async fn run(chain_id: u64) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    // Read SSR from Ethereum (canonical)
    let ssr_ray = {
        let r = rpc::eth_call(
            ETHEREUM_RPC,
            "0xa3931d71877C0E7a3148CB7Eb4463524FEc27fbD",
            "0x03607ceb",
        )
        .await
        .unwrap_or_default();
        rpc::decode_u256(&r).unwrap_or(1_000_000_001_167_363_430_498_603_315u128)
    };

    let dsr_ray = {
        let r = rpc::eth_call(ETHEREUM_RPC, MAKER_POT, "0x487bf082")
            .await
            .unwrap_or_default();
        rpc::decode_u256(&r).unwrap_or(1_000_000_000_393_915_525_145_987_602u128)
    };

    let ssr_apy = rpc::ray_to_apy(ssr_ray);
    let dsr_apy = rpc::ray_to_apy(dsr_ray);

    // TVL: PSM3 totalAssets on L2, sUSDS totalAssets on Ethereum
    let (tvl_raw, tvl_source) = if let Some(psm3) = cfg.psm3 {
        // PSM3.totalAssets() = 0x01e1d114
        match rpc::eth_call(cfg.rpc_url, psm3, "0x01e1d114").await {
            Ok(hex) => {
                let assets = rpc::decode_u256(&hex).unwrap_or(0);
                (assets, psm3)
            }
            Err(_) => (0u128, psm3),
        }
    } else {
        // sUSDS.totalAssets() on Ethereum
        match rpc::eth_call(cfg.rpc_url, cfg.susds, "0x01e1d114").await {
            Ok(hex) => {
                let assets = rpc::decode_u256(&hex).unwrap_or(0);
                (assets, cfg.susds)
            }
            Err(_) => (0u128, cfg.susds),
        }
    };

    let tvl_human = rpc::from_minimal(tvl_raw, 18);

    // sUSDS total supply (circulating)
    let susds_supply_hex = rpc::eth_call(cfg.rpc_url, cfg.susds, "0x18160ddd")
        .await
        .unwrap_or_default();
    let susds_supply = rpc::decode_u256(&susds_supply_hex).unwrap_or(0);
    let susds_supply_human = rpc::from_minimal(susds_supply, 18);

    // Conversion rate (chi): sUSDS → USDS
    let conversion_rate = if let Some(oracle) = cfg.ssr_oracle {
        match rpc::eth_call(cfg.rpc_url, oracle, "0xf36089ec").await {
            Ok(hex) => {
                let chi = rpc::decode_u256(&hex).unwrap_or(1_000_000_000_000_000_000_000_000_000);
                chi as f64 / 1e27
            }
            Err(_) => 1.0,
        }
    } else {
        // Ethereum: chi from sUSDS
        match rpc::eth_call(cfg.rpc_url, cfg.susds, "0xc92aecc4").await {
            Ok(hex) => {
                let chi = rpc::decode_u256(&hex).unwrap_or(1_000_000_000_000_000_000_000_000_000);
                chi as f64 / 1e27
            }
            Err(_) => 1.0,
        }
    };

    let susds_per_usds = if conversion_rate > 0.0 {
        1.0 / conversion_rate
    } else {
        1.0
    };

    let mut result = json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "rates": {
            "ssrApy": format!("{:.4}%", ssr_apy * 100.0),
            "dsrApy": format!("{:.4}%", dsr_apy * 100.0)
        },
        "sUSDS": {
            "address": cfg.susds,
            "totalSupply": format!("{:.2}", susds_supply_human),
            "conversionRate": format!("{:.6} USDS per sUSDS", conversion_rate),
            "susdsPerUSDS": format!("{:.6} sUSDS per USDS", susds_per_usds)
        },
        "tvl": {
            "amount": format!("{:.2}", tvl_human),
            "unit": "USD",
            "source": tvl_source
        },
        "tokens": {
            "usds": cfg.usds,
            "susds": cfg.susds
        }
    });

    // Add sDAI info on Ethereum
    if let Some(sdai) = cfg.sdai {
        let sdai_supply_hex = rpc::eth_call(cfg.rpc_url, sdai, "0x18160ddd")
            .await
            .unwrap_or_default();
        let sdai_supply = rpc::decode_u256(&sdai_supply_hex).unwrap_or(0);
        let sdai_supply_human = rpc::from_minimal(sdai_supply, 18);

        let sdai_tvl_hex = rpc::eth_call(cfg.rpc_url, sdai, "0x01e1d114")
            .await
            .unwrap_or_default();
        let sdai_tvl = rpc::decode_u256(&sdai_tvl_hex).unwrap_or(0);
        let sdai_tvl_human = rpc::from_minimal(sdai_tvl, 18);

        result["sDAI"] = json!({
            "address": sdai,
            "totalSupply": format!("{:.2}", sdai_supply_human),
            "tvl": format!("{:.2}", sdai_tvl_human),
            "dsrApy": format!("{:.4}%", dsr_apy * 100.0)
        });
        result["tokens"]["dai"] = json!(cfg.dai.unwrap_or(""));
        result["tokens"]["sdai"] = json!(sdai);
    }

    // Add PSM3 info on L2
    if let Some(psm3) = cfg.psm3 {
        result["psm3"] = json!({
            "address": psm3,
            "totalLiquidity": format!("{:.2}", tvl_human)
        });
    }

    Ok(result)
}
