use std::process::Command;
use serde_json::Value;

/// Resolve the current logged-in Solana wallet address (base58).
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    // Try data.address first, then data.details[0].tokenAssets[0].address
    if let Some(addr) = json["data"]["address"].as_str() {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    anyhow::bail!(
        "Could not resolve Solana wallet address from onchainos output: {}",
        serde_json::to_string(&json).unwrap_or_default()
    )
}

/// Return native SOL balance in lamports for the given wallet.
pub async fn get_sol_balance(wallet: &str, rpc_url: &str) -> anyhow::Result<u64> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [wallet]
    });
    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    resp["result"]["value"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Failed to parse SOL balance: {}", resp))
}

/// Return SPL token balance in UI units (f64) for the given wallet and mint.
/// Returns 0.0 if the wallet holds no token accounts for this mint.
pub async fn get_spl_balance(wallet: &str, mint: &str, rpc_url: &str) -> anyhow::Result<f64> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet,
            { "mint": mint },
            { "encoding": "jsonParsed" }
        ]
    });
    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    let accounts = resp["result"]["value"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Unexpected RPC response: {}", resp))?;
    if accounts.is_empty() {
        return Ok(0.0);
    }
    let ui_amount = accounts[0]["account"]["data"]["parsed"]["info"]["tokenAmount"]["uiAmount"]
        .as_f64()
        .unwrap_or(0.0);
    Ok(ui_amount)
}

/// Extract transaction hash from onchainos JSON response.
/// onchainos swap execute returns { "ok": true, "data": { "swapTxHash": "..." } }
/// Some responses use "txHash" at data level.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["swapTxHash"]
        .as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| result["data"]["txHash"].as_str().filter(|s| !s.is_empty()))
        .or_else(|| result["txHash"].as_str().filter(|s| !s.is_empty()))
        .unwrap_or("pending")
        .to_string()
}

/// Run `onchainos security token-scan` for a given mint address.
/// Returns "safe", "warn", or "block".
/// Invocation: `onchainos security token-scan --tokens "501:<mint>"`
pub fn security_token_scan(mint: &str) -> anyhow::Result<String> {
    let token_arg = format!("501:{}", mint);
    let output = Command::new("onchainos")
        .args([
            "security",
            "token-scan",
            "--tokens",
            &token_arg,
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
    if json.is_null() {
        anyhow::bail!(
            "onchainos security token-scan returned non-JSON output for mint {}: {}",
            mint,
            stdout.trim()
        );
    }
    // Try to get risk level from response
    let risk = json["data"]["riskLevel"]
        .as_str()
        .or_else(|| json["data"]["risk"].as_str())
        .or_else(|| json["riskLevel"].as_str())
        .unwrap_or("safe")
        .to_lowercase();
    Ok(risk)
}
