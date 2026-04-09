use crate::{config, rpc};

// Maximum pools to fetch to keep response time reasonable (sequential RPC calls)
const MAX_POOLS: u64 = 50;

pub async fn run(chain_id: u64, rpc_url: Option<String>) -> anyhow::Result<()> {
    let cfg = config::get_chain_config(chain_id)?;
    let rpc = config::get_rpc_url(chain_id, rpc_url.as_deref())?;

    let length = rpc::pool_length(cfg.masterchef_v3, &rpc).await?;
    let fetch_count = length.min(MAX_POOLS);
    eprintln!(
        "Fetching {} of {} farm pools on chain {} (most recent {} by pid)...",
        fetch_count, length, chain_id, fetch_count
    );

    // Fetch the most recent pools (highest pids have most recent incentives)
    let start_pid = if length > MAX_POOLS { length - MAX_POOLS } else { 0 };
    let mut pools = Vec::new();
    for pid in start_pid..length {
        match rpc::pool_info(cfg.masterchef_v3, pid, &rpc).await {
            Ok(info) => pools.push(info),
            Err(e) => eprintln!("  Warning: failed to fetch pool pid={}: {}", pid, e),
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "chain_id": chain_id,
            "masterchef_v3": cfg.masterchef_v3,
            "total_pool_count": length,
            "pool_count": pools.len(),
            "note": format!("Showing last {} of {} pools (most recently added)", pools.len(), length),
            "pools": pools
        }))?
    );
    Ok(())
}
