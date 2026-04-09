use crate::{config, onchainos, rpc};

pub async fn run(
    chain_id: u64,
    token_id: u64,
    from: Option<String>,
    dry_run: bool,
    rpc_url: Option<String>,
) -> anyhow::Result<()> {
    let cfg = config::get_chain_config(chain_id)?;
    let rpc = config::get_rpc_url(chain_id, rpc_url.as_deref())?;

    if dry_run {
        // For dry-run use zero address as placeholder (avoids ABI encode errors)
        let from_addr = "0x0000000000000000000000000000000000000000";
        let calldata = build_safe_transfer_from_calldata(from_addr, cfg.masterchef_v3, token_id);
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain_id": chain_id,
                "token_id": token_id,
                "to": cfg.nonfungible_position_manager,
                "calldata": calldata,
                "description": "safeTransferFrom(from, masterchef_v3, tokenId) — stakes NFT into MasterChefV3 farming"
            }))?
        );
        return Ok(());
    }

    // Resolve wallet address (must not be zero) — only needed for non-dry-run
    let wallet = match from {
        Some(addr) => addr,
        None => onchainos::resolve_wallet(chain_id).await.unwrap_or_default(),
    };
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Pre-check: verify NFT ownership
    let owner = rpc::owner_of(cfg.nonfungible_position_manager, token_id, &rpc).await?;
    if owner.to_lowercase() != wallet.to_lowercase() {
        anyhow::bail!(
            "Token ID {} is not owned by wallet {}. Current owner: {}",
            token_id,
            wallet,
            owner
        );
    }

    // Build calldata for safeTransferFrom(from, masterchef_v3, tokenId)
    let calldata = build_safe_transfer_from_calldata(&wallet, cfg.masterchef_v3, token_id);

    // Ask user to confirm — agent must present this to user before calling without --dry-run
    eprintln!(
        "Staking NFT token ID {} into MasterChefV3 ({}) on chain {}...",
        token_id, cfg.masterchef_v3, chain_id
    );

    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.nonfungible_position_manager,
        &calldata,
        Some(&wallet),
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
            "action": "farm",
            "txHash": tx_hash,
            "masterchef_v3": cfg.masterchef_v3,
            "nonfungible_position_manager": cfg.nonfungible_position_manager,
            "raw": result
        }))?
    );
    Ok(())
}

/// Build calldata for safeTransferFrom(address from, address to, uint256 tokenId).
/// selector = 0x42842e0e
fn build_safe_transfer_from_calldata(from: &str, to: &str, token_id: u64) -> String {
    let from_padded = format!("{:0>64}", from.trim_start_matches("0x").to_lowercase());
    let to_padded = format!("{:0>64}", to.trim_start_matches("0x").to_lowercase());
    let token_id_padded = format!("{:064x}", token_id);
    format!("0x42842e0e{}{}{}", from_padded, to_padded, token_id_padded)
}
