use anyhow::Context;
use serde_json::{json, Value};

/// Claim accrued Aave V3 rewards via onchainos defi collect.
///
/// Flow:
/// 1. Fetch defi positions to get analysisPlatformId for Aave V3
/// 2. If no Aave V3 positions exist, return early with "no positions" message
/// 3. Call defi collect --platform-id <id> --chain <chain> --reward-type REWARD_PLATFORM
pub async fn run(
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let wallet_addr = if let Some(addr) = from {
        addr.to_string()
    } else {
        crate::onchainos::wallet_address(chain_id).context(
            "No --from address specified and could not resolve active wallet.",
        )?
    };

    // Step 1: get positions to find analysisPlatformId
    let positions = crate::onchainos::defi_positions(chain_id, &wallet_addr)
        .context("Failed to fetch defi positions")?;

    let platform_id = find_aave_platform_id(&positions);
    let platform_id = match platform_id {
        Some(id) => id,
        None => {
            return Ok(json!({
                "ok": true,
                "message": "No active Aave V3 positions found on this chain. Supply assets first to earn rewards.",
                "chainId": chain_id
            }));
        }
    };

    if dry_run {
        let cmd = format!(
            "onchainos defi collect --platform-id {} --address {} --chain {} --reward-type REWARD_PLATFORM",
            platform_id,
            wallet_addr,
            crate::onchainos::chain_id_to_name_pub(chain_id)
        );
        eprintln!("[dry-run] would execute: {}", cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "platformId": platform_id,
            "simulatedCommand": cmd
        }));
    }

    let result = match crate::onchainos::defi_collect(platform_id, chain_id, &wallet_addr, "REWARD_PLATFORM") {
        Ok(res) => res,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("No reward tokens found") || msg.contains("no reward") {
                return Ok(json!({
                    "ok": true,
                    "message": "No claimable rewards found for this Aave V3 position.",
                    "platformId": platform_id,
                    "chainId": chain_id
                }));
            }
            return Err(e.context("onchainos defi collect failed"));
        }
    };

    let tx_hash = result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending");

    Ok(json!({
        "ok": true,
        "txHash": tx_hash,
        "platformId": platform_id,
        "chainId": chain_id,
        "raw": result
    }))
}

/// Extract the analysisPlatformId for Aave V3 from defi positions response.
fn find_aave_platform_id(positions: &Value) -> Option<u64> {
    let wallet_list = positions
        .get("data")
        .and_then(|d| d.get("walletIdPlatformList"))
        .and_then(|l| l.as_array())?;

    for wallet_entry in wallet_list {
        let platforms = wallet_entry
            .get("platformList")
            .and_then(|l| l.as_array())?;
        for platform in platforms {
            let name = platform
                .get("platformName")
                .and_then(|n| n.as_str())
                .unwrap_or("");
            if name.to_lowercase().contains("aave") {
                if let Some(id) = platform.get("analysisPlatformId").and_then(|v| v.as_u64()) {
                    return Some(id);
                }
            }
        }
    }
    None
}
