/// Wrappers around the `onchainos` CLI for wallet resolution + EVM contract calls.
/// The plugin holds NO private keys.
///
/// `--force` (ONC-001): ALWAYS passed for contract-call (Spark uses regular EVM
///   calls, not pre-signed EIP-712, so onchainos backend's policy gate would
///   otherwise return cryptic "execution reverted" without --force).
/// `--biz-type` / `--strategy`: attribution to the onchainos backend (since
///   onchainos 3.0.0) so analytics can group calls by source plugin.

use std::process::Command;
use serde_json::Value;

/// Single source of truth: `env!` resolves Cargo.toml's `name` field at compile time.
/// CI invariant — Cargo.toml.name === plugin.yaml.name.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Resolve user wallet address on a specific chain.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to spawn onchainos: {} (is onchainos on PATH?)", e))?;
    if !output.status.success() {
        anyhow::bail!(
            "onchainos wallet addresses failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim(),
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
        // Fallback: most onchainos accounts have the same EVM address everywhere
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not resolve wallet address for chain {} from onchainos.", chain_id)
}

/// Execute an EVM contract call via `onchainos wallet contract-call --force`.
///
/// `--force` is **defensively included** — see ONC-001. It is NOT "required for
/// non-interactive calls" (onchainos is non-interactive by default). Its real
/// role is to skip the backend's risk-control prompt that triggers on:
/// unlimited-approve, untrusted contracts, or internal threshold violations.
/// Low-risk daily calls work without `--force` (verified on Polygon
/// `USDC.e.approve(0,0)` in 2026-04). But when the rare risk-control path
/// fires, onchainos returns generic "execution reverted" with no specific
/// code, indistinguishable from an on-chain revert. Always passing `--force`
/// is a no-op in the common case and prevents that misleading failure mode.
///
/// The plugin's `--dry-run` / `--confirm` flags already gate user authorization;
/// the backend confirmation prompt is redundant for our flow.
///
/// `gas_limit` (EVM-015): explicit gas limit override. onchainos's internal
/// eth_estimateGas occasionally under-estimates ERC-4626 / PSM-style calls
/// (observed: redeem(0.5 sUSDS) estimated ~70k → 1.4× buffer = 100k → actual
/// needed 120k → OOG revert). Pass `Some(200_000)` (or higher) for write ops
/// that touch storage. None preserves onchainos's auto-estimate behavior.
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
        "--force".to_string(),  // ← ONC-001
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
    let output = Command::new("onchainos").args(&args).output()
        .map_err(|e| anyhow::anyhow!("Failed to spawn onchainos: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stdout.trim().is_empty() { stderr.to_string() } else { stdout.to_string() };
        anyhow::bail!("onchainos contract-call failed: {}", detail.trim());
    }
    let result: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|_| serde_json::json!({ "raw": stdout.to_string() }));
    Ok(result)
}

/// Pull the tx hash out of a `wallet contract-call` result.
pub fn extract_tx_hash(result: &Value) -> Option<String> {
    for path in [
        ("data", "txHash"),
        ("data", "hash"),
    ] {
        if let Some(s) = result[path.0][path.1].as_str() {
            return Some(s.to_string());
        }
    }
    if let Some(s) = result["txHash"].as_str() { return Some(s.to_string()); }
    if let Some(s) = result["hash"].as_str()    { return Some(s.to_string()); }
    None
}
