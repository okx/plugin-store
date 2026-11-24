use crate::{config, rpc};

pub async fn run(chain_id: u64, token_id: u64, rpc_url: Option<String>) -> anyhow::Result<()> {
    let cfg = config::get_chain_config(chain_id)?;
    let rpc = config::get_rpc_url(chain_id, rpc_url.as_deref())?;

    // Pre-check: verify token exists on-chain (ownerOf reverts for non-existent tokens)
    rpc::owner_of(cfg.nonfungible_position_manager, token_id, &rpc)
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "Token ID {} does not exist on chain {}.",
                token_id,
                chain_id
            )
        })?;

    // Check if the token is currently staked in MasterChefV3
    let staked_info = rpc::user_position_infos(cfg.masterchef_v3, token_id, &rpc).await.ok();
    let zero_addr = "0x0000000000000000000000000000000000000000";
    let is_staked = staked_info
        .as_ref()
        .map(|info| info.user.to_lowercase() != zero_addr)
        .unwrap_or(false);

    let reward_wei = rpc::pending_cake(cfg.masterchef_v3, token_id, &rpc).await?;
    let reward_cake = rpc::format_cake_wei(reward_wei);

    let mut result = serde_json::json!({
        "ok": true,
        "chain_id": chain_id,
        "token_id": token_id,
        "staked": is_staked,
        "pending_cake_wei": reward_wei.to_string(),
        "pending_cake": reward_cake,
        "masterchef_v3": cfg.masterchef_v3
    });

    if !is_staked {
        result["note"] = serde_json::json!(
            "This token is not staked in MasterChefV3. CAKE rewards only accrue on staked positions. \
             Use 'farm --token-id {}' to start earning CAKE."
        );
        // Replace placeholder with actual token_id
        result["note"] = serde_json::json!(format!(
            "This token is not staked in MasterChefV3. CAKE rewards only accrue on staked positions. \
             Use 'farm --token-id {}' to start earning CAKE.",
            token_id
        ));
    }

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
