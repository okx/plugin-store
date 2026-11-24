use std::process::Command;
use serde_json::Value;

/// Solana native SOL mint address used by onchainos swap
pub const SOL_MINT: &str = "11111111111111111111111111111111";

/// Resolve the current Solana wallet address (base58) via onchainos.
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", "501"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}", e))?;
    let addr = json["data"]["solana"][0]["address"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if addr.is_empty() {
        anyhow::bail!("Could not resolve Solana wallet address. Make sure onchainos is logged in.");
    }
    Ok(addr)
}

/// Execute a swap via `onchainos swap execute`.
/// Works for both bonding curve tokens and graduated (DEX) tokens.
pub async fn swap_execute_solana(
    from_mint: &str,
    to_mint: &str,
    readable_amount: &str,
    slippage_bps: u64,
) -> anyhow::Result<Value> {
    // Convert bps to percent string (e.g. 100 bps → "1", 50 bps → "0.5")
    let slippage_pct = format!("{}", slippage_bps as f64 / 100.0);
    let wallet = resolve_wallet_solana()?;

    let output = tokio::process::Command::new("onchainos")
        .args([
            "swap",
            "execute",
            "--chain",
            "solana",
            "--from",
            from_mint,
            "--to",
            to_mint,
            "--readable-amount",
            readable_amount,
            "--slippage",
            &slippage_pct,
            "--wallet",
            &wallet,
        ])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if stdout.trim().is_empty() {
        anyhow::bail!("onchainos swap execute returned empty output. stderr: {}", stderr);
    }

    let result: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("onchainos swap execute non-JSON: {stdout}\n{e}"))?;

    if result["ok"].as_bool() != Some(true) {
        let err = result["error"].as_str().unwrap_or("unknown error");
        anyhow::bail!("onchainos swap execute failed: {}", err);
    }

    Ok(result)
}

/// Get token balance for a specific mint from the onchainos wallet.
/// Returns readable amount as a string, or None if not held.
pub fn get_token_balance(mint: &str) -> anyhow::Result<Option<String>> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;

    let details = match json["data"]["details"].as_array() {
        Some(d) => d,
        None => return Ok(None),
    };

    for detail in details {
        if let Some(assets) = detail["tokenAssets"].as_array() {
            for asset in assets {
                // onchainos wallet balance --chain 501 structure:
                //   asset["address"]      = wallet address (always present — do NOT use for mint matching)
                //   asset["tokenAddress"] = token mint address (correct field to match against)
                //   asset["mint"]         = fallback for alternative response shapes
                let addr = asset["tokenAddress"].as_str()
                    .or_else(|| asset["mint"].as_str())
                    .unwrap_or("");
                if addr.eq_ignore_ascii_case(mint) {
                    if let Some(bal) = asset["balance"].as_str()
                        .or_else(|| asset["readableBalance"].as_str())
                        .or_else(|| asset["uiAmount"].as_str())
                    {
                        return Ok(Some(bal.to_string()));
                    }
                    if let Some(n) = asset["balance"].as_f64()
                        .or_else(|| asset["uiAmount"].as_f64())
                    {
                        return Ok(Some(n.to_string()));
                    }
                }
            }
        }
    }
    Ok(None)
}

/// Extract the txHash from an onchainos swap response.
/// Returns an error if txHash is absent, so broadcast failures are not silently masked.
pub fn extract_tx_hash(result: &Value) -> anyhow::Result<String> {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["data"]["swapTxHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("onchainos response missing txHash: {}", result))
}
