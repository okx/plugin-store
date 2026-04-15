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

/// Execute a Solana DEX swap via `onchainos dex swap execute`.
/// This is the primary swap path — onchainos handles routing, signing, and broadcasting.
/// NOTE: Solana dex swap does NOT require --force.
pub async fn dex_swap_solana(
    from_token: &str, // SPL mint address or SOL native mint
    to_token: &str,
    readable_amount: &str, // UI units e.g. "0.5"
    slippage_pct: &str,    // percent e.g. "1" for 1%
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "" },
            "note": "dry_run=true — no transaction submitted"
        }));
    }
    let output = Command::new("onchainos")
        .args([
            "dex",
            "swap",
            "execute",
            "--chain",
            "501",
            "--from-token",
            from_token,
            "--to-token",
            to_token,
            "--readable-amount",
            readable_amount,
            "--slippage",
            slippage_pct,
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || stdout.trim().is_empty() {
        anyhow::bail!(
            "onchainos dex swap execute failed: stdout={} stderr={}",
            stdout,
            stderr
        );
    }
    Ok(serde_json::from_str(&stdout)?)
}

/// Submit a serialized Solana transaction via `onchainos wallet contract-call --unsigned-tx`.
/// Used as fallback when dex swap execute is unavailable.
/// WARNING: Solana blockhash expires ~60 seconds — call immediately after receiving serialized_tx.
pub async fn wallet_contract_call_solana(
    to: &str,
    serialized_tx: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "" },
            "serialized_tx": serialized_tx
        }));
    }
    let output = Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            "501",
            "--to",
            to,
            "--unsigned-tx",
            serialized_tx,
            "--force", // required — without this onchainos won't broadcast
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
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
pub fn security_token_scan(mint: &str) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args([
            "security",
            "token-scan",
            "--address",
            mint,
            "--chain",
            "501",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
    // Try to get risk level from response
    let risk = json["data"]["riskLevel"]
        .as_str()
        .or_else(|| json["data"]["risk"].as_str())
        .or_else(|| json["riskLevel"].as_str())
        .unwrap_or("safe")
        .to_lowercase();
    Ok(risk)
}
