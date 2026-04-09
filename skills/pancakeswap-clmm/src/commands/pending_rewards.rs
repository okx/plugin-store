use crate::{config, rpc};

pub async fn run(chain_id: u64, token_id: u64, rpc_url: Option<String>) -> anyhow::Result<()> {
    let cfg = config::get_chain_config(chain_id)?;
    let rpc = config::get_rpc_url(chain_id, rpc_url.as_deref())?;

    let reward_wei = rpc::pending_cake(cfg.masterchef_v3, token_id, &rpc).await?;
    let reward_cake = reward_wei as f64 / 1e18;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "chain_id": chain_id,
            "token_id": token_id,
            "pending_cake_wei": reward_wei.to_string(),
            "pending_cake": format!("{:.6}", reward_cake),
            "masterchef_v3": cfg.masterchef_v3
        }))?
    );
    Ok(())
}
