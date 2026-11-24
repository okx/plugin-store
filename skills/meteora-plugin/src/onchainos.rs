use std::process::Command;
use serde_json::Value;

/// `--biz-type` / `--strategy`: attribution to the onchainos backend.
/// Source-of-truth for the plugin name is Cargo.toml's `[package]` `name`.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Resolve the current Solana wallet address via onchainos
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    // Use `wallet addresses` — always returns the address regardless of balance.
    // `wallet balance --chain 501` is unreliable: tokenAssets is empty when SOL balance is 0.
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap_or(serde_json::json!({}));

    // data.solana[0].address
    if let Some(addr) = json["data"]["solana"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|s| s["address"].as_str())
    {
        return Ok(addr.to_string());
    }

    anyhow::bail!(
        "Cannot resolve Solana wallet address. Make sure onchainos is logged in.\nRaw output: {}",
        stdout
    )
}

/// Get the native SOL balance (lamports → SOL) for a Solana address.
pub fn get_sol_balance(wallet: &str) -> f64 {
    let output = std::process::Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()
        .ok();
    let stdout = output.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let json: Value = serde_json::from_str(&stdout).unwrap_or(serde_json::json!({}));
    // Find native SOL entry (tokenAddress == "" or == "11111111111111111111111111111111")
    if let Some(assets) = json["data"]["details"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|d| d["tokenAssets"].as_array())
    {
        for asset in assets {
            let addr = asset["tokenAddress"].as_str().unwrap_or("");
            if addr.is_empty() || addr == "11111111111111111111111111111111" {
                let _ = wallet; // wallet is implicit (logged-in account)
                return asset["balance"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
            }
        }
    }
    0.0
}

/// Get the SPL token balance (human-readable) for a given mint on Solana.
/// Returns 0.0 if the ATA doesn't exist or has zero balance.
pub fn get_spl_token_balance(token_mint: &str) -> f64 {
    let output = std::process::Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501", "--token-address", token_mint])
        .output()
        .ok();
    let stdout = output.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let json: Value = serde_json::from_str(&stdout).unwrap_or(serde_json::json!({}));
    json["data"]["details"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|d| d["tokenAssets"].as_array())
        .and_then(|a| a.first())
        .and_then(|t| t["balance"].as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0)
}

/// Execute onchainos swap quote for Solana (dry run path for swap)
pub fn dex_quote_solana(
    from_token: &str,
    to_token: &str,
    readable_amount: &str,
) -> anyhow::Result<Value> {
    let output = Command::new("onchainos")
        .args([
            "swap", "quote",
            "--chain", "solana",
            "--from", from_token,
            "--to", to_token,
            "--readable-amount", readable_amount,
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout).unwrap_or(serde_json::json!({
        "ok": true,
        "dry_run": true,
        "raw": stdout.to_string()
    })))
}

/// Execute onchainos swap execute for Solana
/// NOTE: Solana does NOT need --force
pub fn dex_swap_execute_solana(
    from_token: &str,
    to_token: &str,
    readable_amount: &str,
    wallet: &str,
    slippage: Option<&str>,
) -> anyhow::Result<Value> {
    let mut args = vec![
        "swap", "execute",
        "--chain", "solana",
        "--from", from_token,
        "--to", to_token,
        "--readable-amount", readable_amount,
        "--wallet", wallet,
    ];
    if let Some(s) = slippage {
        args.extend_from_slice(&["--slippage", s]);
    }
    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).map_err(|e| anyhow::anyhow!("Failed to parse onchainos output: {e}\nRaw: {stdout}"))
}

/// Send a pre-built (unsigned) Solana transaction via onchainos.
///
/// onchainos signs with the currently logged-in wallet and broadcasts to mainnet.
/// Chain 501 = Solana mainnet.
/// `program_id` is passed as `--to` (the primary program being called).
pub fn contract_call_solana(unsigned_tx_b58: &str, program_id: &str) -> anyhow::Result<Value> {
    let output = Command::new("onchainos")
        .args([
            "wallet", "contract-call",
            "--biz-type", BIZ_TYPE,
            "--strategy", STRATEGY,
            "--chain", "501",
            "--to", program_id,
            "--unsigned-tx", unsigned_tx_b58,
            "--force", // required — without this onchainos simulates first; new accounts don't exist yet → ProgramAccountNotFound
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    serde_json::from_str(&stdout).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse onchainos contract-call output: {e}\nstdout: {stdout}\nstderr: {stderr}"
        )
    })
}

/// Extract txHash from onchainos result
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["data"]["swapTxHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
