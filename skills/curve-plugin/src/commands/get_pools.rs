// commands/get_pools.rs — List Curve pools on a given chain
use crate::{api, config};
use anyhow::Result;

pub async fn run(chain_id: u64, registry: Option<String>, limit: usize) -> Result<()> {
    let chain_name = config::chain_name(chain_id);

    let pools = match registry {
        Some(ref r) => api::get_pools(chain_name, r).await?,
        None => api::get_all_pools(chain_name).await?,
    };

    if pools.is_empty() {
        println!("{}", serde_json::json!({ "ok": false, "error": "No pools found" }));
        return Ok(());
    }

    let mut sorted = pools;
    sorted.sort_by(|a, b| {
        b.usd_total
            .unwrap_or(0.0)
            .partial_cmp(&a.usd_total.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let display: Vec<_> = sorted
        .iter()
        .take(limit)
        .map(|p| {
            let coins: Vec<_> = p
                .coins
                .iter()
                .map(|c| serde_json::json!({ "symbol": c.symbol, "address": c.address }))
                .collect();
            let base_apy = p.latest_daily_apy_pcent.map(|v| format!("{:.2}%", v));
            let crv_apy = p
                .gauge_crv_apy
                .as_ref()
                .and_then(|v| v.first())
                .and_then(|v| *v)
                .map(|v| format!("{:.2}%", v));
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "address": p.address,
                "coins": coins,
                "tvl_usd": p.usd_total,
                "base_apy": base_apy,
                "crv_apy": crv_apy,
                "fee_raw": p.fee
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "chain": chain_name,
            "count": display.len(),
            "pools": display
        })
    );
    Ok(())
}
