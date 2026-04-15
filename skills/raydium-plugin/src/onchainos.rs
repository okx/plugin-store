use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::process::Command;
use serde_json::Value;

/// Resolve the current logged-in Solana wallet address (base58).
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", "501"])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("onchainos wallet addresses failed (exit {}): {}", output.status, stderr.trim());
    }
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}", e))?;
    let addr = json["data"]["solana"][0]["address"].as_str().unwrap_or("").to_string();
    if addr.is_empty() {
        anyhow::bail!("Could not resolve Solana wallet address -- ensure onchainos is logged in");
    }
    Ok(addr)
}

/// Submit a Solana serialized transaction via onchainos.
/// serialized_tx: base64-encoded VersionedTransaction from Raydium API.
/// onchainos --unsigned-tx expects base58, so we convert here.
/// NOTE: Solana blockhash expires in ~60s -- call immediately after receiving tx.
/// NOTE: --force is required for Solana --unsigned-tx submissions to broadcast.
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
    // onchainos --unsigned-tx expects base58; Raydium API returns base64
    let tx_bytes = BASE64.decode(serialized_tx)
        .map_err(|e| anyhow::anyhow!("Failed to decode base64 tx: {}", e))?;
    let tx_base58 = bs58::encode(&tx_bytes).into_string();

    let output = tokio::process::Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            "501",
            "--to",
            to,
            "--unsigned-tx",
            &tx_base58,
            "--force", // required for Solana --unsigned-tx to broadcast
        ])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
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
    let resp: Value = client
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

/// Return SPL token balance in raw units (u64) for the given wallet and mint.
/// Returns 0 if the wallet holds no token account for this mint.
pub async fn get_spl_token_balance(owner: &str, mint: &str, rpc_url: &str) -> anyhow::Result<u64> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            owner,
            { "mint": mint },
            { "encoding": "jsonParsed" }
        ]
    });
    let resp: Value = client
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
        return Ok(0);
    }
    let amount_str = accounts[0]["account"]["data"]["parsed"]["info"]["tokenAmount"]["amount"]
        .as_str()
        .unwrap_or("0");
    amount_str
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("Failed to parse token amount: {}", amount_str))
}

/// Resolve the user's Associated Token Account (ATA) for a given SPL mint via Solana RPC.
/// Required by Raydium's /transaction/swap-base-in API as `inputAccount` when input is SPL.
pub async fn get_token_account(owner: &str, mint: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            owner,
            { "mint": mint },
            { "encoding": "base64" }
        ]
    });
    let resp: Value = client
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
        anyhow::bail!(
            "No token account found for mint {} in wallet {}. \
             Ensure the wallet holds this token before swapping.",
            mint, owner
        );
    }
    accounts[0]["pubkey"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Missing pubkey in token account response"))
}
/// Extract txHash from onchainos response.
/// Returns an error if txHash is absent, so broadcast failures are not silently masked.
pub fn extract_tx_hash(result: &Value) -> anyhow::Result<String> {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("onchainos response missing txHash: {}", result))
}
