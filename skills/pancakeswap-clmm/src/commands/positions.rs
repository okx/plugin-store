use crate::{config, onchainos, rpc};

pub async fn run(
    chain_id: u64,
    owner: Option<String>,
    token_ids_staked: Option<String>,
    rpc_url: Option<String>,
) -> anyhow::Result<()> {
    let cfg = config::get_chain_config(chain_id)?;
    let rpc = config::get_rpc_url(chain_id, rpc_url.as_deref())?;

    // Resolve wallet address
    let wallet = match owner {
        Some(addr) => addr,
        None => onchainos::resolve_wallet(chain_id).await.unwrap_or_default(),
    };
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --owner or ensure onchainos is logged in.");
    }

    // Fetch unstaked positions (held in wallet)
    let balance = rpc::nft_balance_of(cfg.nonfungible_position_manager, &wallet, &rpc).await?;
    let mut unstaked = Vec::new();
    for i in 0..balance {
        match rpc::token_of_owner_by_index(
            cfg.nonfungible_position_manager,
            &wallet,
            i,
            &rpc,
        )
        .await
        {
            Ok(token_id) => {
                match rpc::get_position(cfg.nonfungible_position_manager, token_id, &rpc).await {
                    Ok(pos) => unstaked.push(pos),
                    Err(e) => eprintln!("Warning: failed to fetch position {}: {}", token_id, e),
                }
            }
            Err(e) => eprintln!("Warning: tokenOfOwnerByIndex({}) failed: {}", i, e),
        }
    }

    // Fetch staked positions (if token IDs provided via --include-staked)
    let mut staked = Vec::new();
    if let Some(ids_str) = token_ids_staked {
        for part in ids_str.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            match part.parse::<u64>() {
                Ok(token_id) => {
                    match rpc::user_position_infos(cfg.masterchef_v3, token_id, &rpc).await {
                        Ok(info) => {
                            // Also fetch position details from NonfungiblePositionManager
                            let pos =
                                rpc::get_position(cfg.nonfungible_position_manager, token_id, &rpc)
                                    .await
                                    .ok();
                            let pending =
                                rpc::pending_cake(cfg.masterchef_v3, token_id, &rpc).await.unwrap_or(0);
                            staked.push(serde_json::json!({
                                "token_id": token_id,
                                "staked": true,
                                "user": info.user,
                                "pid": info.pid,
                                "liquidity": info.liquidity.to_string(),
                                "tick_lower": info.tick_lower,
                                "tick_upper": info.tick_upper,
                                "pending_cake_wei": pending.to_string(),
                                "pending_cake": format!("{:.6}", pending as f64 / 1e18),
                                "position": pos
                            }));
                        }
                        Err(e) => eprintln!("Warning: userPositionInfos({}) failed: {}", token_id, e),
                    }
                }
                Err(_) => eprintln!("Warning: invalid token_id '{}'", part),
            }
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "chain_id": chain_id,
            "wallet": wallet,
            "nonfungible_position_manager": cfg.nonfungible_position_manager,
            "masterchef_v3": cfg.masterchef_v3,
            "unstaked_count": unstaked.len(),
            "unstaked_positions": unstaked,
            "staked_count": staked.len(),
            "staked_positions": staked
        }))?
    );
    Ok(())
}
