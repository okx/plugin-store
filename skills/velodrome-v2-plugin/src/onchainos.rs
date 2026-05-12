use std::process::Command;
use serde_json::Value;

/// `--biz-type` / `--strategy`: attribution to the onchainos backend.
/// Source-of-truth for the plugin name is Cargo.toml's `[package]` `name`.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Resolve the wallet address for chain_id (Optimism=10) from the onchainos CLI.
/// Uses `onchainos wallet addresses` and parses data.evm[].address matching chainIndex.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    let chain_id_str = chain_id.to_string();
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_id_str) {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
        // fallback: use first EVM address
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not resolve wallet address for chain {}", chain_id)
}

/// Execute a write operation via `onchainos wallet contract-call`.
/// All DEX write ops require --force to actually broadcast.
/// In dry_run mode, returns a mock response without calling onchainos.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    force: bool,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {"txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"},
            "calldata": input_data
        }));
    }
    if !force {
        // Preview mode: --confirm not passed, do not broadcast
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "message": "Add --confirm to broadcast this transaction",
            "calldata": input_data
        }));
    }
    let chain_str = chain_id.to_string();
    let mut args = vec![
        "wallet",
        "contract-call",
        "--biz-type",
        BIZ_TYPE,
        "--strategy",
        STRATEGY,
        "--chain",
        &chain_str,
        "--to",
        to,
        "--input-data",
        input_data,
    ];
    if force {
        args.push("--force");
    }
    let output = Command::new("onchainos").args(&args).output()?;
    Ok(serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?)
}

/// Extract txHash from a wallet_contract_call response.
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}
