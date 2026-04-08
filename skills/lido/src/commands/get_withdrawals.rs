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
    )?;

    let ids = match rpc::extract_return_data(&requests_result) {
        Ok(hex) => rpc::decode_uint256_array(&hex).unwrap_or_default(),
        Err(_) => {
            println!("No withdrawal requests found for {}", address);
            return Ok(());
        }
    };

    if ids.is_empty() {
        println!("No withdrawal requests found for {}", address);
        return Ok(());
    }

    println!("=== Lido Withdrawal Requests ===");
    println!("Address: {}", address);
    println!("Found {} request(s): {:?}", ids.len(), ids);
    println!();

    // Step 2: getWithdrawalStatus(uint256[]) -> WithdrawalRequestStatus[]
    let status_calldata = rpc::calldata_get_withdrawal_status(&ids);
    let status_result = onchainos::eth_call(
        chain_id,
        config::WITHDRAWAL_QUEUE_ADDRESS,
        &status_calldata,
    )?;

    // Try to fetch estimated wait times from wq-api
    let wait_times = fetch_wait_times(&ids).await;

    // Print raw status data
    match rpc::extract_return_data(&status_result) {
        Ok(hex) => {
            println!("Status data (hex): {}", &hex[..hex.len().min(128)], );
            // Parse each status entry (each is 6 * 32 bytes = 192 bytes = 384 hex chars)
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
                let amount_steth_wei =
                    u128::from_str_radix(&entry[0..64], 16).unwrap_or(0);
                let amount_steth = amount_steth_wei as f64 / 1e18;
                let is_finalized = u128::from_str_radix(&entry[4 * 64..5 * 64], 16)
                    .unwrap_or(0)
                    != 0;
                let is_claimed = u128::from_str_radix(&entry[5 * 64..6 * 64], 16)
                    .unwrap_or(0)
                    != 0;

                let status = if is_claimed {
                    "CLAIMED"
                } else if is_finalized {
                    "READY TO CLAIM"
                } else {
                    "PENDING"
                };

                print!(
                    "  Request #{}: {:.6} stETH — {}",
                    id, amount_steth, status
                );
                if let Some(wait) = wait_times.as_ref().and_then(|w| w.get(i)) {
                    print!(" (est. wait: {})", wait);
                }
                println!();
            }
        }
        Err(_) => {
            println!("Status: {}", status_result);
        }
    }

    println!();
    println!("Use `lido claim-withdrawal --ids <ID1,ID2,...>` to claim finalized requests.");

    Ok(())
}

async fn fetch_wait_times(ids: &[u128]) -> Option<Vec<String>> {
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
                entry["requestInfo"]["finalizationIn"]
                    .as_str()
                    .map(|s| s.to_string())
                    .or_else(|| {
                        entry["expectedWaitTimeSeconds"]
                            .as_u64()
                            .map(|s| format!("{}s", s))
                    })
                    .unwrap_or_else(|| "unknown".to_string())
            })
            .collect(),
    )
}
