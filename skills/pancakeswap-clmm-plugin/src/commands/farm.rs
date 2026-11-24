use crate::{config, onchainos, rpc};

pub async fn run(
    chain_id: u64,
    token_id: u64,
    from: Option<String>,
    dry_run: bool,
    confirm: bool,
    rpc_url: Option<String>,
) -> anyhow::Result<()> {
    let cfg = config::get_chain_config(chain_id)?;
    let rpc = config::get_rpc_url(chain_id, rpc_url.as_deref())?;

    if dry_run {
        // Try to resolve wallet for accurate calldata; fall back to zero placeholder
        let from_addr = match from.as_deref() {
            Some(addr) => addr.to_string(),
            None => onchainos::resolve_wallet(chain_id)
                .await
                .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string()),
        };
        let calldata = build_safe_transfer_from_calldata(&from_addr, cfg.masterchef_v3, token_id);
        let placeholder = from_addr == "0x0000000000000000000000000000000000000000";
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain_id": chain_id,
                "token_id": token_id,
                "from": from_addr,
                "to": cfg.nonfungible_position_manager,
                "calldata": calldata,
                "description": "safeTransferFrom(from, masterchef_v3, tokenId) — stakes NFT into MasterChefV3 farming",
                "note": if placeholder { Some("from address is a placeholder — onchainos wallet not resolved") } else { None }
            }))?
        );
        return Ok(());
    }

    // Resolve wallet address
    let wallet = match from {
        Some(addr) => addr,
        None => onchainos::resolve_wallet(chain_id).await.unwrap_or_default(),
    };
    if wallet.is_empty() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": false,
            "error": "Cannot resolve wallet address. Pass --from or ensure onchainos is logged in.",
            "action_required": "onchainos wallet login"
        }))?);
        return Ok(());
    }

    // Pre-check: verify NFT exists and is owned by this wallet
    let owner = match rpc::owner_of(cfg.nonfungible_position_manager, token_id, &rpc).await {
        Ok(o) => o,
        Err(_) => {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "error": format!("Token ID {} does not exist on chain {}.", token_id, chain_id),
            }))?);
            return Ok(());
        }
    };
    if owner.to_lowercase() != wallet.to_lowercase() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": false,
            "error": format!("Token ID {} is not owned by wallet {}. Current owner: {}", token_id, wallet, owner),
            "action_required": format!("Use the wallet that owns token ID {}", token_id)
        }))?);
        return Ok(());
    }

    if !confirm {
        // Preview mode: show what will happen and require --confirm to proceed
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "preview": true,
                "action": "farm",
                "chain_id": chain_id,
                "token_id": token_id,
                "wallet": wallet,
                "masterchef_v3": cfg.masterchef_v3,
                "nonfungible_position_manager": cfg.nonfungible_position_manager,
                "message": "Run again with --confirm to stake the NFT into MasterChefV3."
            }))?
        );
        return Ok(());
    }

    // Build calldata for safeTransferFrom(from, masterchef_v3, tokenId)
    let calldata = build_safe_transfer_from_calldata(&wallet, cfg.masterchef_v3, token_id);

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
