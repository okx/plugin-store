use crate::{config, onchainos, rpc};

pub async fn run(
    chain_id: u64,
    token_id: u64,
    to: Option<String>,
    dry_run: bool,
    rpc_url: Option<String>,
) -> anyhow::Result<()> {
    let cfg = config::get_chain_config(chain_id)?;
    let rpc = config::get_rpc_url(chain_id, rpc_url.as_deref())?;

    if dry_run {
        let calldata = build_harvest_calldata(token_id, "0x0000000000000000000000000000000000000000");
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain_id": chain_id,
                "token_id": token_id,
                "to": cfg.masterchef_v3,
                "calldata": calldata,
                "description": "harvest(tokenId, to) — claims CAKE rewards without withdrawing the NFT"
            }))?
        );
        return Ok(());
    }

    // Resolve recipient address (must not be zero) — only needed for non-dry-run
    let recipient = match to {
        Some(addr) => addr,
        None => onchainos::resolve_wallet(chain_id).await.unwrap_or_default(),
    };
    if recipient.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --to or ensure onchainos is logged in.");
    }

    // Check pending CAKE before harvest
    let pending_wei = rpc::pending_cake(cfg.masterchef_v3, token_id, &rpc).await?;
    if pending_wei == 0 {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "chain_id": chain_id,
                "token_id": token_id,
                "message": "No pending CAKE rewards to harvest.",
                "pending_cake": "0"
            }))?
        );
        return Ok(());
    }

    let pending_cake = pending_wei as f64 / 1e18;
    eprintln!(
        "Harvesting {:.6} CAKE for token ID {} on chain {}...",
        pending_cake, token_id, chain_id
    );

    // Build calldata for harvest(uint256 tokenId, address to)
    // selector = 0x18fccc76
    let calldata = build_harvest_calldata(token_id, &recipient);

    // Ask user to confirm — agent must present confirmation before calling without --dry-run
    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.masterchef_v3,
        &calldata,
        Some(&recipient),
        None,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "chain_id": chain_id,
            "token_id": token_id,
            "action": "harvest",
            "txHash": tx_hash,
            "pending_cake_harvested_wei": pending_wei.to_string(),
            "pending_cake_harvested": format!("{:.6}", pending_cake),
            "recipient": recipient,
            "masterchef_v3": cfg.masterchef_v3,
            "raw": result
        }))?
    );
    Ok(())
}

/// Build calldata for harvest(uint256 tokenId, address to).
/// selector = 0x18fccc76
fn build_harvest_calldata(token_id: u64, to: &str) -> String {
    let token_id_padded = format!("{:064x}", token_id);
    let to_padded = format!("{:0>64}", to.trim_start_matches("0x").to_lowercase());
    format!("0x18fccc76{}{}", token_id_padded, to_padded)
}
