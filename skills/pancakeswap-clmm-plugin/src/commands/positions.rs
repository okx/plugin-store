use crate::{config, onchainos, rpc};

/// Return a human-friendly symbol for well-known token addresses.
/// Falls back to the first 8 chars of the address for unknown tokens.
fn token_symbol(addr: &str) -> String {
    match addr.to_lowercase().as_str() {
        // BSC
        "0x55d398326f99059ff775485246999027b3197955" => "USDT".to_string(),
        "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d" => "USDC".to_string(),
        "0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c" => "WBNB".to_string(),
        "0xe9e7cea3dedca5984780bafc599bd69add087d56" => "BUSD".to_string(),
        "0x7130d2a12b9bcbfae4f2634d864a1ee1ce3ead9c" => "BTCB".to_string(),
        "0x2170ed0880ac9a755fd29b2688956bd959f933f8" => "ETH".to_string(),
        "0x0e09fabb73bd3ade0a17ecc321fd13a19e81ce82" => "CAKE".to_string(),
        // Ethereum mainnet
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => "WETH".to_string(),
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => "USDC".to_string(),
        "0xdac17f958d2ee523a2206206994597c13d831ec7" => "USDT".to_string(),
        // Base
        "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" => "USDC".to_string(),
        "0x4200000000000000000000000000000000000006" => "WETH".to_string(),
        _ => addr.chars().take(8).collect(),
    }
}

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
    let mut unstaked: Vec<serde_json::Value> = Vec::new();
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
                    Ok(pos) => {
                        let pair = format!("{}/{}", token_symbol(&pos.token0), token_symbol(&pos.token1));
                        let mut v = serde_json::to_value(&pos).unwrap_or_default();
                        if let serde_json::Value::Object(ref mut map) = v {
                            map.insert("pair".to_string(), serde_json::Value::String(pair));
                        }
                        unstaked.push(v);
                    }
                    Err(e) => eprintln!("Warning: failed to fetch position {}: {}", token_id, e),
                }
            }
            Err(e) => eprintln!("Warning: tokenOfOwnerByIndex({}) failed: {}", i, e),
        }
    }

    // Determine candidate staked token IDs:
    // - If --include-staked provided: use those IDs directly (explicit override)
    // - Otherwise: auto-discover via ERC-721 Transfer log scan
    let (staked_candidates, discovery_mode, discovery_note) = if let Some(ids_str) = token_ids_staked {
        let ids: Vec<u64> = ids_str
            .split(',')
            .filter_map(|s| s.trim().parse::<u64>().ok())
            .collect();
        let note = format!("Using {} manually specified token ID(s).", ids.len());
        (ids, "manual", note)
    } else {
        let (candidates, note) = rpc::scan_staked_token_ids(
            cfg.nonfungible_position_manager,
            cfg.masterchef_v3,
            &wallet,
            cfg.nft_deployment_block,
            &rpc,
        )
        .await;
        (candidates, "auto", note)
    };

    // For each candidate, verify it is currently staked for this wallet via userPositionInfos.
    // This is the authoritative on-chain check and handles any log-scan edge cases.
    let mut staked = Vec::new();
    let mut verified_count = 0usize;
    for token_id in &staked_candidates {
        match rpc::user_position_infos(cfg.masterchef_v3, *token_id, &rpc).await {
            Ok(info) => {
                // Confirm this position is staked for our wallet (not someone else's)
                if info.user.to_lowercase() != wallet.to_lowercase() {
                    continue;
                }
                verified_count += 1;
                let pending =
                    rpc::pending_cake(cfg.masterchef_v3, *token_id, &rpc).await.unwrap_or(0);
                let pos = rpc::get_position(cfg.nonfungible_position_manager, *token_id, &rpc)
                    .await
                    .ok();
                let pair = pos.as_ref().map(|p| {
                    format!("{}/{}", token_symbol(&p.token0), token_symbol(&p.token1))
                });
                staked.push(serde_json::json!({
                    "token_id": token_id,
                    "staked": true,
                    "pair": pair,
                    "user": info.user,
                    "pid": info.pid,
                    "liquidity": info.liquidity.to_string(),
                    "boost_liquidity": info.boost_liquidity.to_string(),
                    "tick_lower": info.tick_lower,
                    "tick_upper": info.tick_upper,
                    "pending_cake_wei": pending.to_string(),
                    "pending_cake": rpc::format_cake_wei(pending),
                    "position": pos
                }));
            }
            Err(e) => {
                eprintln!(
                    "Warning: userPositionInfos({}) failed (may not be staked): {}",
                    token_id, e
                );
            }
        }
    }

    let final_note = if discovery_mode == "auto" && !staked_candidates.is_empty() {
        format!(
            "{} Confirmed {} staked position(s) on-chain.",
            discovery_note, verified_count
        )
    } else {
        discovery_note
    };

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
            "staked_positions": staked,
            "staked_discovery": discovery_mode,
            "staked_discovery_note": final_note
        }))?
    );
    Ok(())
}
