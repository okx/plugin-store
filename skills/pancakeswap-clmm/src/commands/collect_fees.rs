use crate::{config, onchainos, rpc};

pub async fn run(
    chain_id: u64,
    token_id: u64,
    recipient: Option<String>,
    dry_run: bool,
    rpc_url: Option<String>,
) -> anyhow::Result<()> {
    let cfg = config::get_chain_config(chain_id)?;
    let rpc = config::get_rpc_url(chain_id, rpc_url.as_deref())?;

    if dry_run {
        let calldata = build_collect_calldata(token_id, "0x0000000000000000000000000000000000000000");
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain_id": chain_id,
                "token_id": token_id,
                "to": cfg.nonfungible_position_manager,
                "calldata": calldata,
                "description": "collect((tokenId, recipient, uint128Max, uint128Max)) — collects all accrued swap fees from an unstaked position"
            }))?
        );
        return Ok(());
    }

    // Resolve recipient address (must not be zero) — only needed for non-dry-run
    let fee_recipient = match recipient {
        Some(addr) => addr,
        None => onchainos::resolve_wallet(chain_id).await.unwrap_or_default(),
    };
    if fee_recipient.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --recipient or ensure onchainos is logged in.");
    }

    // Pre-check: verify NFT is held in wallet (not staked in MasterChefV3)
    let owner = rpc::owner_of(cfg.nonfungible_position_manager, token_id, &rpc).await?;
    if owner.to_lowercase() == cfg.masterchef_v3.to_lowercase() {
        anyhow::bail!(
            "Token ID {} is staked in MasterChefV3. Please run 'unfarm' first to withdraw it before collecting fees.",
            token_id
        );
    }

    // Check accrued fees
    let pos = rpc::get_position(cfg.nonfungible_position_manager, token_id, &rpc).await?;
    if pos.tokens_owed0 == 0 && pos.tokens_owed1 == 0 {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "chain_id": chain_id,
                "token_id": token_id,
                "message": "No accrued fees to collect.",
                "tokens_owed0": "0",
                "tokens_owed1": "0"
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

    // Ask user to confirm — agent must present confirmation before calling without --dry-run
    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.nonfungible_position_manager,
        &calldata,
        Some(&fee_recipient),
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
