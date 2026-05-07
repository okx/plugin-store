use crate::{config, onchainos, rpc};

pub async fn run(
    chain_id: u64,
    token_id: u64,
    recipient: Option<String>,
    dry_run: bool,
    confirm: bool,
    rpc_url: Option<String>,
) -> anyhow::Result<()> {
    let cfg = config::get_chain_config(chain_id)?;
    let rpc = config::get_rpc_url(chain_id, rpc_url.as_deref())?;

    if dry_run {
        // Try to resolve wallet for accurate calldata; fall back to zero placeholder
        let recipient_addr = match recipient.as_deref() {
            Some(addr) => addr.to_string(),
            None => onchainos::resolve_wallet(chain_id)
                .await
                .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string()),
        };
        // Fetch accrued fee amounts for the preview
        let pos = rpc::get_position(cfg.nonfungible_position_manager, token_id, &rpc).await.ok();
        let calldata = build_collect_calldata(token_id, &recipient_addr);
        let placeholder = recipient_addr == "0x0000000000000000000000000000000000000000";
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain_id": chain_id,
                "token_id": token_id,
                "recipient": recipient_addr,
                "tokens_owed0": pos.as_ref().map(|p| p.tokens_owed0.to_string()),
                "tokens_owed1": pos.as_ref().map(|p| p.tokens_owed1.to_string()),
                "token0": pos.as_ref().map(|p| p.token0.as_str()),
                "token1": pos.as_ref().map(|p| p.token1.as_str()),
                "to": cfg.nonfungible_position_manager,
                "calldata": calldata,
                "description": "collect((tokenId, recipient, uint128Max, uint128Max)) — collects all accrued swap fees from an unstaked position",
                "note": if placeholder { Some("recipient is a placeholder — onchainos wallet not resolved") } else { None }
            }))?
        );
        return Ok(());
    }

    // Resolve signer wallet (always the active onchainos account — the NFT owner)
    let wallet = onchainos::resolve_wallet(chain_id).await.unwrap_or_default();
    if wallet.is_empty() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": false,
            "error": "Cannot resolve wallet address. Pass --recipient or ensure onchainos is logged in.",
            "action_required": "onchainos wallet login"
        }))?);
        return Ok(());
    }
    // Fee recipient defaults to signer wallet
    let fee_recipient = recipient.unwrap_or_else(|| wallet.clone());

    // Pre-check: verify NFT exists and is held in wallet (not staked in MasterChefV3)
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
    if owner.to_lowercase() == cfg.masterchef_v3.to_lowercase() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": false,
            "error": format!("Token ID {} is staked in MasterChefV3. Run 'unfarm --token-id {}' first to withdraw it.", token_id, token_id),
            "action_required": format!("pancakeswap-clmm-plugin unfarm --token-id {}", token_id)
        }))?);
        return Ok(());
    }

    // Check accrued fees
    let pos = rpc::get_position(cfg.nonfungible_position_manager, token_id, &rpc).await?;
    if pos.tokens_owed0 == 0 && pos.tokens_owed1 == 0 {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "early_exit": true,
                "chain_id": chain_id,
                "token_id": token_id,
                "message": "No accrued fees to collect.",
                "tokens_owed0": "0",
                "tokens_owed1": "0"
            }))?
        );
        return Ok(());
    }

    if !confirm {
        // Preview mode: show fee amounts and require --confirm to proceed
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "preview": true,
                "action": "collect-fees",
                "chain_id": chain_id,
                "token_id": token_id,
                "recipient": fee_recipient,
                "tokens_owed0": pos.tokens_owed0.to_string(),
                "tokens_owed1": pos.tokens_owed1.to_string(),
                "token0": pos.token0,
                "token1": pos.token1,
                "nonfungible_position_manager": cfg.nonfungible_position_manager,
                "message": "Run again with --confirm to collect the fees."
            }))?
        );
        return Ok(());
    }

    eprintln!(
        "Collecting fees for token ID {}: tokensOwed0={}, tokensOwed1={}",
        token_id, pos.tokens_owed0, pos.tokens_owed1
    );

    // Build calldata for collect((uint256 tokenId, address recipient, uint128 amount0Max, uint128 amount1Max))
    // selector = 0xfc6f7865
    let calldata = build_collect_calldata(token_id, &fee_recipient);

    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.nonfungible_position_manager,
        &calldata,
        Some(&wallet),  // signer = NFT owner wallet, not fee recipient
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
            "action": "collect-fees",
            "txHash": tx_hash,
            "tokens_owed0": pos.tokens_owed0.to_string(),
            "tokens_owed1": pos.tokens_owed1.to_string(),
            "token0": pos.token0,
            "token1": pos.token1,
            "recipient": fee_recipient,
            "nonfungible_position_manager": cfg.nonfungible_position_manager,
            "raw": result
        }))?
    );
    Ok(())
}

/// Build calldata for collect((uint256,address,uint128,uint128)).
/// selector = 0xfc6f7865
/// amount0Max = amount1Max = uint128::MAX
fn build_collect_calldata(token_id: u64, recipient: &str) -> String {
    let token_id_padded = format!("{:064x}", token_id);
    let recipient_padded = format!(
        "{:0>64}",
        recipient.trim_start_matches("0x").to_lowercase()
    );
    // uint128::MAX = 0xffffffffffffffffffffffffffffffff (16 bytes), padded to 32 bytes
    let amount_max = "00000000000000000000000000000000ffffffffffffffffffffffffffffffff";
    format!(
        "0xfc6f7865{}{}{}{}",
        token_id_padded, recipient_padded, amount_max, amount_max
    )
}
