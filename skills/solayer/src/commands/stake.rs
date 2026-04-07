use crate::config::SSOL_MINT;
use crate::onchainos;
use serde_json::Value;
use std::process::Command;

/// Stake SOL to receive sSOL.
/// Uses `onchainos swap execute` to route SOL → sSOL via Jupiter DEX.
///
/// NOTE: The Solayer REST API returns partially-signed transactions requiring
/// 2 signers (Solayer key + user key). onchainos --unsigned-tx does not support
/// multi-signer partially-signed transactions, so we use `onchainos swap execute`
/// which routes via Jupiter and properly handles Solana transaction signing.
///
/// amount: SOL amount in UI units (e.g. 0.001)
/// dry_run: simulate without broadcasting
pub async fn execute(amount: f64, dry_run: bool) -> anyhow::Result<Value> {
    // Native SOL mint address on Solana
    const SOL_NATIVE_MINT: &str = "11111111111111111111111111111111";

    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "",
                "amount_sol": amount,
                "ssol_mint": SSOL_MINT,
                "description": format!("Would swap {} SOL → sSOL via onchainos swap execute (Jupiter routing)", amount)
            }
        }));
    }

    // Resolve Solana wallet address (after dry_run guard)
    let wallet = onchainos::resolve_wallet_solana()?;

    // Use onchainos swap execute: SOL → sSOL via Jupiter
    // Note: onchainos swap execute handles signing internally
    let amount_str = format!("{}", amount);
    let output = Command::new("onchainos")
        .args([
            "swap", "execute",
            "--chain", "501",
            "--from", SOL_NATIVE_MINT,
            "--to", SSOL_MINT,
            "--readable-amount", &amount_str,
            "--slippage", "0.5",
            "--wallet", &wallet,
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos response: {}\nOutput: {}", e, stdout))?;

    if result["ok"].as_bool() != Some(true) {
        anyhow::bail!("Stake failed: {}", result);
    }

    let tx_hash = onchainos::extract_tx_hash(&result);
    let to_amount = result["data"]["toAmount"]
        .as_str()
        .unwrap_or("0");
    let ssol_received = to_amount.parse::<f64>().unwrap_or(0.0) / 1e9;

    Ok(serde_json::json!({
        "ok": true,
        "data": {
            "txHash": tx_hash,
            "amount_sol": amount,
            "ssol_received": ssol_received,
            "ssol_mint": SSOL_MINT,
            "description": format!("Staked {} SOL → {:.9} sSOL", amount, ssol_received)
        }
    }))
}
