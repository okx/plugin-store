/// onchainos CLI wrappers for fourmeme-plugin.
///
/// `--force` (ONC-001):
/// - For prerequisite ERC-20 approves (when quote token is non-BNB) we pass `--force`
///   because the user-facing approve is internal to the buy flow — Agent already
///   authorized the trade.
/// - For the user-facing buy/sell tx we pass `force=false` and let onchainos surface
///   any backend confirmation prompts naturally.
/// `--biz-type` / `--strategy`: attribution to the onchainos backend (since
///   onchainos 3.0.0) so analytics can group calls by source plugin.
///
/// Tx confirmation uses direct RPC `eth_getTransactionReceipt` (NOT `onchainos wallet
/// history --tx-hash` — that command requires `--address` which isn't always wired
/// up at every call site).

use anyhow::{Context, Result};
use serde_json::Value;

/// Single source of truth: `env!` resolves Cargo.toml's `name` field at compile time.
/// CI invariant — Cargo.toml.name === plugin.yaml.name.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Call `onchainos wallet contract-call`.
///
/// `amt = Some(wei)` populates `--amt` (used to attach BNB as msg.value).
/// `force = true` skips backend confirmation prompts (use for token approvals).
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>,
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

/// Extract the txHash from a wallet contract-call response.
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

/// Wallet address for the given chain via `onchainos wallet addresses`.
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

/// Poll `eth_getTransactionReceipt` until the tx confirms with status 0x1, or bail
/// on revert (0x0) / timeout. Source of truth = chain itself; bypasses any history-
/// cache lag in `onchainos wallet history`.
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
                 The tx may still land later — check BSCScan.",
                tx_hash, max_wait_secs, chain_id
            );
        }
        if let Ok(r) = reqwest::Client::new().post(&rpc).json(&body).send().await {
            if let Ok(v) = r.json::<Value>().await {
                if let Some(receipt) = v.get("result") {
                    if !receipt.is_null() {
                        match receipt["status"].as_str() {
                            Some("0x1") => return Ok(()),
                            Some("0x0") => {
                                anyhow::bail!(
                                    "Tx {} mined but reverted on-chain (status 0x0). \
                                     Check BSCScan for revert reason.",
                                    tx_hash
                                );
                            }
                            _ => {} // unknown status — keep polling
                        }
                    }
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}
