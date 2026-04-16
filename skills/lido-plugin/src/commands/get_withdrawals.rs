use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct GetWithdrawalsArgs {
    /// Address to query withdrawal requests for (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: GetWithdrawalsArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    let address = args
        .address
        .clone()
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if address.is_empty() {
        anyhow::bail!("Cannot get wallet address. Pass --address or ensure onchainos is logged in.");
    }

    // Step 1: getWithdrawalRequests(address) -> uint256[]
    let requests_calldata = rpc::calldata_get_withdrawal_requests(&address);
    let requests_result = onchainos::eth_call(
        chain_id,
        config::WITHDRAWAL_QUEUE_ADDRESS,
        &requests_calldata,
    ).await?;

    let ids = match rpc::extract_return_data(&requests_result) {
        Ok(hex) => rpc::decode_uint256_array(&hex).unwrap_or_default(),
        Err(e) => {
            anyhow::bail!("Failed to query withdrawal requests: {}", e);
        }
    };

    if ids.is_empty() {
        println!("{}", serde_json::json!({
            "ok": true,
            "address": address,
            "count": 0,
            "requests": []
        }));
        return Ok(());
    }

    // Step 2: getWithdrawalStatus(uint256[]) -> WithdrawalRequestStatus[]
    let status_calldata = rpc::calldata_get_withdrawal_status(&ids);
    let status_result = onchainos::eth_call(
        chain_id,
        config::WITHDRAWAL_QUEUE_ADDRESS,
        &status_calldata,
    ).await?;

    // Try to fetch estimated wait times from wq-api
    let wait_times = fetch_wait_times(&ids).await;

    let mut requests: Vec<serde_json::Value> = Vec::new();

    match rpc::extract_return_data(&status_result) {
        Ok(hex) => {
            let hex = hex.trim_start_matches("0x");
            // Skip ABI array header (offset + length = 128 hex chars)
            let data = if hex.len() > 128 { &hex[128..] } else { hex };
            let entry_size = 6 * 64; // 6 uint256/bool slots × 64 hex chars
            for (i, &id) in ids.iter().enumerate() {
                let start = i * entry_size;
                if start + entry_size > data.len() {
                    break;
                }
                let entry = &data[start..start + entry_size];

                // EVM-012 fix: propagate parse error instead of silently returning 0
                let amount_steth_wei = match u128::from_str_radix(&entry[0..64], 16) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[lido] Warning: failed to decode amountOfStETH for request #{}: {}", id, e);
                        continue;
                    }
                };
                let amount_steth = amount_steth_wei as f64 / 1e18;
                let is_finalized = u128::from_str_radix(&entry[4 * 64..5 * 64], 16)
                    .unwrap_or(0) != 0;
                let is_claimed = u128::from_str_radix(&entry[5 * 64..6 * 64], 16)
                    .unwrap_or(0) != 0;

                let status = if is_claimed {
                    "CLAIMED"
                } else if is_finalized {
                    "READY_TO_CLAIM"
                } else {
                    "PENDING"
                };

                let estimated_wait = wait_times
                    .as_ref()
                    .and_then(|w| w.get(i))
                    .and_then(|v| v.as_deref())
                    .unwrap_or("");

                let mut entry_json = serde_json::json!({
                    "id": id.to_string(),
                    "amountStEth": format!("{:.6}", amount_steth),
                    "amountStEthWei": amount_steth_wei.to_string(),
                    "status": status
                });
                if !estimated_wait.is_empty() {
                    entry_json["estimatedWait"] = serde_json::Value::String(estimated_wait.to_string());
                }
                requests.push(entry_json);
            }
        }
        Err(e) => {
            anyhow::bail!("Failed to decode withdrawal status: {}", e);
        }
    }

    println!("{}", serde_json::json!({
        "ok": true,
        "address": address,
        "count": requests.len(),
        "requests": requests,
        "hint": "Use `lido claim-withdrawal --ids <ID1,ID2,...>` to claim finalized requests."
    }));

    Ok(())
}

async fn fetch_wait_times(ids: &[u128]) -> Option<Vec<Option<String>>> {
    if ids.is_empty() {
        return None;
    }
    let ids_params: Vec<String> = ids.iter().map(|id| format!("ids={}", id)).collect();
    let url = format!(
        "{}/v2/request-time?{}",
        config::WQ_API_BASE_URL,
        ids_params.join("&")
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "lido-plugin/0.1.0")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let body: serde_json::Value = resp.json().await.ok()?;
    let arr = body.as_array().or_else(|| body["data"].as_array())?;

    Some(
        arr.iter()
            .map(|entry| {
                // Finalized requests have no wait — skip the label entirely
                if entry["status"].as_str() == Some("finalized") {
                    return None;
                }
                entry["requestInfo"]["finalizationIn"]
                    .as_str()
                    .map(|s| Some(s.to_string()))
                    .unwrap_or_else(|| {
                        entry["expectedWaitTimeSeconds"]
                            .as_u64()
                            .map(|s| format!("{}s", s))
                    })
            })
            .collect(),
    )
}
