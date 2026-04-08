/// positions command — list LP positions (BPT holdings) for a wallet

use anyhow::Result;
use serde::Serialize;

use crate::config;
use crate::onchainos;
use crate::rpc;

#[derive(Debug, Serialize)]
struct Position {
    pool_id: String,
    pool_address: String,
    bpt_balance: String,
    bpt_balance_raw: String,
    chain_id: u64,
}

pub async fn run(chain_id: u64, wallet: Option<&str>) -> Result<()> {
    let rpc_url = config::rpc_url(chain_id);

    let wallet_addr = match wallet {
        Some(w) => w.to_string(),
        None => onchainos::resolve_wallet(chain_id).unwrap_or_else(|_| {
            // Try to get from onchainos wallet addresses command
            std::process::Command::new("onchainos")
                .args(["wallet", "addresses"])
                .output()
                .ok()
                .and_then(|o| {
                    let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                    let json: serde_json::Value = serde_json::from_str(&stdout).ok()?;
                    // Look for chainIndex matching chain_id
                    let chain_str = chain_id.to_string();
                    json["data"]["evm"]
                        .as_array()?
                        .iter()
                        .find(|e| e["chainIndex"].as_str() == Some(&chain_str))
                        .and_then(|e| e["address"].as_str().map(String::from))
                })
                .unwrap_or_default()
        }),
    };
    if wallet_addr.is_empty() {
        anyhow::bail!("Could not resolve wallet address. Pass --wallet <address> or ensure onchainos is logged in.");
    }

    let known = config::known_pools(chain_id);

    let mut positions: Vec<Position> = Vec::new();

    for (pool_id, pool_address) in &known {
        let balance = rpc::get_balance_of(pool_address, &wallet_addr, rpc_url).await?;
        if balance > 0 {
            let bpt_human = format!("{:.6}", balance as f64 / 1e18);
            positions.push(Position {
                pool_id: pool_id.to_string(),
                pool_address: pool_address.to_string(),
                bpt_balance: bpt_human,
                bpt_balance_raw: balance.to_string(),
                chain_id,
            });
        }
    }

    if positions.is_empty() {
        println!("{}", serde_json::json!({"positions": [], "wallet": wallet_addr, "chain_id": chain_id}));
    } else {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "positions": positions,
            "wallet": wallet_addr,
            "chain_id": chain_id
        }))?);
    }
    Ok(())
}
