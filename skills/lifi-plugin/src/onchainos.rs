/// Thin wrappers around the `onchainos` CLI for wallet resolution + contract calls.
///
/// The plugin holds NO private keys. All signing/broadcasting goes through `onchainos`.
/// This file exposes 3 functions:
///   resolve_wallet(chain_id) → user's wallet address on that chain
///   wallet_contract_call(chain, to, data, value, dry_run) → executes a contract call
///   extract_tx_hash(result) → pulls the tx hash from the `wallet contract-call` JSON
///
/// `--biz-type` / `--strategy`: attribution to the onchainos backend (since
///   onchainos 3.0.0) so analytics can group calls by source plugin.

use std::process::Command;
use serde_json::Value;

/// Single source of truth: `env!` resolves Cargo.toml's `name` field at compile time.
/// CI invariant — Cargo.toml.name === plugin.yaml.name.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Resolve the user's wallet address on a specific chain.
///
/// Strategy:
///   1. Run `onchainos wallet addresses` (returns all chains' addresses)
///   2. Find the entry whose `chainIndex` matches `chain_id`
///   3. Fall back to the first EVM address if a per-chain match isn't found
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to spawn onchainos: {} (is onchainos installed and on PATH?)", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "onchainos wallet addresses failed: stdout={} stderr={}",
            stdout.trim(),
            stderr.trim()
        );
    }
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| anyhow::anyhow!("parse onchainos addresses JSON failed: {}", e))?;
    let chain_id_str = chain_id.to_string();
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_id_str) {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!(
        "Could not resolve wallet address for chain {} from onchainos output. Run `onchainos wallet addresses` to inspect.",
        chain_id
    )
}

/// Execute an EVM contract call via `onchainos wallet contract-call --force`.
///
/// `--force` is **defensively included** — see ONC-001. It is NOT "required for
/// non-interactive calls" (onchainos is non-interactive by default). Its real
/// role is to skip the backend's risk-control prompt that triggers on:
/// unlimited-approve, untrusted contracts, or internal threshold violations.
/// Low-risk daily calls work without `--force` (verified on Polygon
/// `USDC.e.approve(0,0)`). But when the rare risk-control path fires,
/// onchainos returns generic "execution reverted" with no specific code,
/// indistinguishable from an on-chain revert. Always passing `--force`
/// is a no-op in the common case and prevents that misleading failure mode.
///
/// `gas_limit` (EVM-015): explicit gas limit override. onchainos's internal
/// eth_estimateGas occasionally under-estimates complex bridge calls (some
/// LI.FI tools allocate 200k+ gas for cross-chain message routing). Pass
/// `Some(N)` per-tool to avoid OOG reverts; None preserves auto-estimate.
///
/// chain_id: target EVM chain id
/// to:       contract address (LI.FI router for bridge, or token for approve)
/// calldata: hex-encoded call (0x-prefixed)
/// value_wei: ETH/native value to attach
/// gas_limit: explicit gas limit (recommended for EVM writes — see EVM-015)
/// dry_run: if true, returns preview JSON without invoking onchainos
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    calldata: &str,
    value_wei: Option<u128>,
    gas_limit: Option<u64>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "chain": chain_id,
            "to": to,
            "data": calldata,
            "value_wei": value_wei.map(|v| v.to_string()),
            "gas_limit": gas_limit.map(|g| g.to_string()),
            "note": "Dry run — calldata not submitted"
        }));
    }
    let mut args = vec![
        "wallet".to_string(),
        "contract-call".to_string(),
        "--force".to_string(),  // ← ONC-001 defensive
        "--biz-type".to_string(),
        BIZ_TYPE.to_string(),
        "--strategy".to_string(),
        STRATEGY.to_string(),
        "--chain".to_string(),
        chain_id.to_string(),
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        calldata.to_string(),
    ];
    if let Some(v) = value_wei {
        args.push("--amt".to_string());
        args.push(v.to_string());
    }
    if let Some(g) = gas_limit {
        args.push("--gas-limit".to_string());
        args.push(g.to_string());
    }
    let output = Command::new("onchainos")
        .args(&args)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to spawn onchainos: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stdout.trim().is_empty() {
            stderr.to_string()
        } else {
            stdout.to_string()
        };
        anyhow::bail!("onchainos contract-call failed: {}", detail.trim());
    }
    let result: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|_| serde_json::json!({ "raw": stdout.to_string() }));
    Ok(result)
}

/// Pull the tx hash out of an `onchainos wallet contract-call` result.
/// onchainos puts the hash under `data.txHash` for EVM. Falls back to `txHash` at root.
pub fn extract_tx_hash(result: &Value) -> Option<String> {
    if let Some(s) = result["data"]["txHash"].as_str() {
        return Some(s.to_string());
    }
    if let Some(s) = result["txHash"].as_str() {
        return Some(s.to_string());
    }
    if let Some(s) = result["data"]["hash"].as_str() {
        return Some(s.to_string());
    }
    if let Some(s) = result["hash"].as_str() {
        return Some(s.to_string());
    }
    None
}
