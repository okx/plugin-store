use std::process::Command;
use serde_json::Value;

/// Resolve the current logged-in wallet address for a given EVM chain.
/// Uses `onchainos wallet balance --chain <id>` and extracts `data.address`.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    if let Some(addr) = json["data"]["address"].as_str() {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    // Also try onchainos wallet addresses endpoint
    let output2 = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json2: Value = serde_json::from_str(&String::from_utf8_lossy(&output2.stdout))?;
    // Find address for chainIndex "1"
    if let Some(arr) = json2["data"]["evm"].as_array() {
        for item in arr {
            if item["chainIndex"].as_str() == Some("1") || item["chainIndex"].as_u64() == Some(1) {
                if let Some(addr) = item["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
        // Fallback: first EVM address
        if let Some(first) = arr.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not resolve wallet address. Please ensure onchainos is logged in.")
}

/// Submit an EVM contract call via onchainos wallet contract-call.
/// dry_run=true: returns a simulated response immediately (no onchainos call).
/// ⚠️  `onchainos wallet contract-call` does NOT support --dry-run.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>, // wei value for payable calls
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "calldata": input_data
        }));
    }

    let chain_str = chain_id.to_string();
    let mut args = vec![
        "wallet",
        "contract-call",
        "--chain",
        &chain_str,
        "--to",
        to,
        "--input-data",
        input_data,
        "--force",
    ];

    let amt_str;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }

    let from_str_owned;
    if let Some(f) = from {
        from_str_owned = f.to_string();
        args.extend_from_slice(&["--from", &from_str_owned]);
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos response: {}\nRaw: {}", e, stdout))
}

/// ERC-20 approve via onchainos.
/// approve(address,uint256) selector = 0x095ea7b3
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // approve(address,uint256) selector = 0x095ea7b3
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run).await
}

/// Extract txHash from onchainos response.
/// Checks data.txHash first, then root txHash.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// Direct eth_call via public JSON-RPC (for read-only queries).
/// Uses ethereum.publicnode.com for Ethereum mainnet.
pub fn eth_call(chain_id: u64, to: &str, input_data: &str) -> anyhow::Result<String> {
    let rpc_url = match chain_id {
        1 => "https://ethereum.publicnode.com",
        _ => anyhow::bail!("Unsupported chain_id for eth_call: {}", chain_id),
    };
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": input_data },
            "latest"
        ],
        "id": 1
    });
    let client = reqwest::blocking::Client::new();
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()?
        .json()?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Decode a uint256 hex string returned from eth_call
pub fn decode_uint256(hex_str: &str) -> u128 {
    let s = hex_str.trim_start_matches("0x");
    if s.is_empty() || s == "0" {
        return 0;
    }
    u128::from_str_radix(s, 16).unwrap_or(0)
}
