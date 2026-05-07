use crate::{config, onchainos, rpc};

pub async fn run(
    chain_id: u64,
    token_id: u64,
    to: Option<String>,
    dry_run: bool,
    confirm: bool,
    rpc_url: Option<String>,
) -> anyhow::Result<()> {
    let cfg = config::get_chain_config(chain_id)?;
    let rpc = config::get_rpc_url(chain_id, rpc_url.as_deref())?;

    if dry_run {
        // Try to resolve wallet for accurate calldata; fall back to zero placeholder
        let recipient_addr = match to.as_deref() {
            Some(addr) => addr.to_string(),
            None => onchainos::resolve_wallet(chain_id)
                .await
                .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string()),
        };
        let pending_wei = rpc::pending_cake(cfg.masterchef_v3, token_id, &rpc)
            .await
            .unwrap_or(0);
        let calldata = build_harvest_calldata(token_id, &recipient_addr);
        let placeholder = recipient_addr == "0x0000000000000000000000000000000000000000";
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain_id": chain_id,
                "token_id": token_id,
                "recipient": recipient_addr,
                "pending_cake": rpc::format_cake_wei(pending_wei),
                "pending_cake_wei": pending_wei.to_string(),
                "to": cfg.masterchef_v3,
                "calldata": calldata,
                "description": "harvest(tokenId, to) — claims CAKE rewards without withdrawing the NFT",
                "note": if placeholder { Some("recipient is a placeholder — onchainos wallet not resolved") } else { None }
            }))?
        );
        return Ok(());
    }

    // Resolve signer wallet (always the active onchainos account — the NFT staker)
    let wallet = onchainos::resolve_wallet(chain_id).await.unwrap_or_default();
    if wallet.is_empty() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": false,
            "error": "Cannot resolve wallet address. Pass --to or ensure onchainos is logged in.",
            "action_required": "onchainos wallet login"
        }))?);
        return Ok(());
    }
    // Recipient defaults to signer wallet
    let recipient = to.unwrap_or_else(|| wallet.clone());

    // Pre-check: verify token exists (pending_cake returns 0 for nonexistent tokens — misleading)
    match rpc::owner_of(cfg.nonfungible_position_manager, token_id, &rpc).await {
        Ok(_) => {} // token exists, proceed
        Err(_) => {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "error": format!("Token ID {} does not exist on chain {}.", token_id, chain_id),
            }))?);
            return Ok(());
        }
    }

    // Check pending CAKE before harvest
    let pending_wei = rpc::pending_cake(cfg.masterchef_v3, token_id, &rpc).await?;
    if pending_wei == 0 {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "early_exit": true,
                "chain_id": chain_id,
                "token_id": token_id,
                "message": "No pending CAKE rewards to harvest.",
                "pending_cake": "0"
            }))?
        );
        return Ok(());
    }

    let pending_cake = rpc::format_cake_wei(pending_wei);

    if !confirm {
        // Preview mode: show what will happen and require --confirm to proceed
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "preview": true,
                "action": "harvest",
                "chain_id": chain_id,
                "token_id": token_id,
                "recipient": recipient,
                "pending_cake": pending_cake,
                "pending_cake_wei": pending_wei.to_string(),
                "masterchef_v3": cfg.masterchef_v3,
                "message": "Run again with --confirm to claim CAKE rewards."
            }))?
        );
        return Ok(());
    }

    eprintln!(
        "Harvesting {} CAKE for token ID {} on chain {}...",
        pending_cake, token_id, chain_id
    );

    // Build calldata for harvest(uint256 tokenId, address to)
    // selector = 0x18fccc76
    let calldata = build_harvest_calldata(token_id, &recipient);

    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.masterchef_v3,
        &calldata,
        Some(&wallet),  // signer = NFT staker wallet, not recipient
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
            "pending_cake_harvested": pending_cake,
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
