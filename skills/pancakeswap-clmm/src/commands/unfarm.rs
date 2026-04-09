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
        let calldata = build_withdraw_calldata(token_id, "0x0000000000000000000000000000000000000000");
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain_id": chain_id,
                "token_id": token_id,
                "to": cfg.masterchef_v3,
                "calldata": calldata,
                "description": "withdraw(tokenId, to) — withdraws NFT from MasterChefV3 and harvests pending CAKE"
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

    // Pre-check: verify token is staked in MasterChefV3 by this user
    let info = rpc::user_position_infos(cfg.masterchef_v3, token_id, &rpc).await?;
    if info.user.to_lowercase() != recipient.to_lowercase()
        && info.user != "0x0000000000000000000000000000000000000000"
    {
        // If user field doesn't match, still attempt (user might pass --to a different addr)
        eprintln!(
            "Note: token {} staked by {}, withdrawing to {}",
            token_id, info.user, recipient
        );
    }

    // Show pending CAKE before unfarm
    let pending_wei = rpc::pending_cake(cfg.masterchef_v3, token_id, &rpc)
        .await
        .unwrap_or(0);
    let pending_cake = pending_wei as f64 / 1e18;
    eprintln!(
        "Withdrawing NFT {} from MasterChefV3. Pending CAKE to harvest: {:.6}",
        token_id, pending_cake
    );

    // Build calldata for withdraw(uint256 tokenId, address to)
    // selector = 0x00f714ce
    let calldata = build_withdraw_calldata(token_id, &recipient);

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
            "action": "unfarm",
            "txHash": tx_hash,
            "pending_cake_harvested": format!("{:.6}", pending_cake),
            "recipient": recipient,
            "masterchef_v3": cfg.masterchef_v3,
            "raw": result
        }))?
    );
    Ok(())
}

/// Build calldata for withdraw(uint256 tokenId, address to).
/// selector = 0x00f714ce
fn build_withdraw_calldata(token_id: u64, to: &str) -> String {
    let token_id_padded = format!("{:064x}", token_id);
    let to_padded = format!("{:0>64}", to.trim_start_matches("0x").to_lowercase());
    format!("0x00f714ce{}{}", token_id_padded, to_padded)
}
