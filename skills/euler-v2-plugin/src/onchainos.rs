/// onchainos CLI wrappers for Euler v2 plugin.
///
/// The `force` parameter on `wallet_contract_call` follows the morpho convention:
/// - `force=true` for prerequisite steps (token approvals) — Agent already authorized
///   the user-facing operation, so the inner approve doesn't need a second confirmation.
/// - `force=false` for the main user action (supply / borrow / etc.) — onchainos's
///   backend may add a confirmation prompt for risk-flagged contracts; passing the
///   prompt through to the user is acceptable since this is the user-facing tx.
///
/// `--force` (ONC-001): only matters when backend triggers a confirmation
///   prompt; for low-risk calls it's a no-op. Passed defensively on approves;
///   for main txs we let onchainos surface backend prompts naturally.
/// `--biz-type` / `--strategy`: attribution to the onchainos backend (since
///   onchainos 3.0.0) so analytics can group calls by source plugin.

use anyhow::{Context, Result};
use serde_json::Value;

/// Single source of truth: `env!` resolves Cargo.toml's `name` field at compile time.
/// CI invariant — Cargo.toml.name === plugin.yaml.name (Phase 2 build pipeline matches
/// the binary against `plugins/<plugin.yaml.name>@<version>`), so this stays in sync
/// with the canonical plugin name without any manual drift between files.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Sign an EIP-712 structured message via `onchainos wallet sign-message`.
/// Required for some Euler operations (e.g. permit-style approvals planned for v0.2).
#[allow(dead_code)]
pub async fn sign_eip712(structured_data_json: &str, chain_id: u64) -> Result<String> {
    let wallet_addr = get_wallet_address(chain_id).await
        .context("Failed to resolve wallet address for sign-message")?;

    let output = tokio::process::Command::new("onchainos")
        .args([
            "wallet", "sign-message",
            "--type", "eip712",
            "--message", structured_data_json,
            "--chain", &chain_id.to_string(),
            "--from", &wallet_addr,
            "--force",
        ])
        .output()
        .await
        .context("Failed to spawn onchainos wallet sign-message")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("onchainos sign-message failed ({}): {}", output.status, stderr.trim());
    }

    let v: Value = serde_json::from_str(stdout.trim())
        .with_context(|| format!("parsing sign-message output: {}", stdout.trim()))?;
    v["data"]["signature"]
        .as_str()
        .or_else(|| v["signature"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no signature in onchainos output: {}", stdout.trim()))
}

/// Call `onchainos wallet contract-call` on the given chain.
///
/// `force=true` skips backend confirmation prompts (use for token approvals).
/// `force=false` lets onchainos surface backend prompts (use for main user actions).
///
/// Note: `onchainos wallet contract-call` does NOT have a `--dry-run` flag — plugins
/// that need dry-run semantics must short-circuit BEFORE calling this function and
/// print their own preview JSON. We don't add dry_run to the args list to avoid
/// silently passing an invalid flag to onchainos.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>,
    _dry_run: bool, // historical signature param, no longer routed to onchainos
    force: bool,
) -> Result<Value> {
    let chain_str = chain_id.to_string();
    let mut args: Vec<String> = vec![
        "wallet".into(),
        "contract-call".into(),
        "--biz-type".into(), BIZ_TYPE.into(),
        "--strategy".into(), STRATEGY.into(),
        "--chain".into(), chain_str,
        "--to".into(), to.into(),
        "--input-data".into(), input_data.into(),
    ];
    if let Some(f) = from {
        args.push("--from".into());
        args.push(f.into());
    }
    if let Some(a) = amt {
        args.push("--amt".into());
        args.push(a.to_string());
    }
    if force {
        args.push("--force".into());
    }

    let output = tokio::process::Command::new("onchainos")
        .args(&args)
        .output()
        .await
        .context("Failed to spawn onchainos wallet contract-call")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .with_context(|| format!("parse onchainos output: {}", stdout))
}

/// Extract txHash from wallet contract-call response.
pub fn extract_tx_hash(result: &Value) -> Result<String> {
    if result["ok"].as_bool() != Some(true) {
        let msg = result["error"].as_str()
            .or_else(|| result["message"].as_str())
            .unwrap_or("unknown error");
        anyhow::bail!("contract-call failed: {}", msg);
    }
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no txHash in contract-call response: {}", result))
}

/// Get the wallet address for the given chain via `onchainos wallet addresses`.
/// Parses `data.evm[0].address`.
pub async fn get_wallet_address(chain_id: u64) -> Result<String> {
    let output = tokio::process::Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", &chain_id.to_string()])
        .output()
        .await
        .context("Failed to spawn onchainos wallet addresses")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(&stdout)
        .with_context(|| format!("wallet addresses parse error: raw={}", stdout))?;
    v["data"]["evm"][0]["address"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!(
            "Could not determine wallet address from onchainos output. \
             Run `onchainos wallet status` to verify the active wallet, \
             or `onchainos wallet add` to create one if none exists."
        ))
}

/// Wait for a tx hash to confirm on-chain via direct `eth_getTransactionReceipt` RPC.
///
/// Polls every 3s up to `max_wait_secs`. Returns Ok if the receipt's `status` field
/// is `0x1` (success). Bails if status is `0x0` (reverted) or the deadline elapses.
///
/// Why direct RPC vs `onchainos wallet history`:
///   - The chain itself is the source of truth — bypasses any history-cache lag
///   - `onchainos wallet history --tx-hash` requires `--address` which we don't always
///     have wired up at every call site
pub async fn wait_for_tx_receipt(tx_hash: &str, chain_id: u64, max_wait_secs: u64) -> Result<()> {
    let rpc = crate::config::Urls::rpc_for_chain(chain_id)
        .ok_or_else(|| anyhow::anyhow!("No RPC URL configured for chain {}", chain_id))?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionReceipt",
        "params": [tx_hash],
        "id": 1
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(max_wait_secs);
    loop {
        if std::time::Instant::now() > deadline {
            anyhow::bail!(
                "Tx {} not confirmed within {}s on chain {}. \
                 The tx may still land later — check the chain's block explorer.",
                tx_hash, max_wait_secs, chain_id
            );
        }
        let resp = reqwest::Client::new()
            .post(&rpc)
            .json(&body)
            .send()
            .await;
        if let Ok(r) = resp {
            if let Ok(v) = r.json::<Value>().await {
                if let Some(receipt) = v.get("result") {
                    if !receipt.is_null() {
                        match receipt["status"].as_str() {
                            Some("0x1") => return Ok(()),
                            Some("0x0") => {
                                anyhow::bail!(
                                    "Tx {} mined but reverted on-chain (status 0x0). \
                                     Check the chain's block explorer for the revert reason.",
                                    tx_hash
                                );
                            }
                            _ => {} // unknown status field — keep polling
                        }
                    }
                    // receipt == null means tx not yet mined; continue polling
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}
