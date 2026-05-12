// src/onchainos.rs — onchainos CLI wrapper (verified against v2.2.6)
use std::process::Command;
use serde_json::Value;

/// `--biz-type` / `--strategy`: attribution to the onchainos backend.
/// Source-of-truth for the plugin name is Cargo.toml's `[package]` `name`.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Run an onchainos sub-command, check exit code, parse stdout as JSON.
fn run_onchainos(args: &[&str]) -> anyhow::Result<Value> {
    let output = Command::new("onchainos").args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow::anyhow!(
            "onchainos {} failed (exit {}): {}",
            args.first().unwrap_or(&""),
            output.status.code().unwrap_or(-1),
            if stderr.trim().is_empty() { stdout } else { stderr }
        ));
    }
    Ok(serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?)
}

/// Resolve the current logged-in wallet EVM address via `wallet addresses`.
pub fn resolve_wallet(_chain_id: u64) -> anyhow::Result<String> {
    let json = run_onchainos(&["wallet", "addresses"])?;
    Ok(json["data"]["evmAddress"].as_str().unwrap_or("").to_string())
}

/// Call `onchainos wallet contract-call`.
///
/// ⚠️ dry_run=true returns a simulated response immediately — contract-call does NOT
///    accept --dry-run and would fail if we passed it.
/// ⚠️ Add --force for DEX/reward operations to prevent "pending" txHash.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
    force: bool,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" },
            "calldata": input_data,
            "to": to
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

    let amt_str;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }

    let from_owned;
    if let Some(f) = from {
        from_owned = f.to_string();
        args.extend_from_slice(&["--from", &from_owned]);
    }

    if force {
        args.push("--force");
    }

    let output = Command::new("onchainos").args(&args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow::anyhow!(
            "onchainos wallet contract-call failed (exit {}): {}",
            output.status.code().unwrap_or(-1),
            if stderr.trim().is_empty() { stdout } else { stderr }
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Extract txHash from `wallet contract-call` response, or return an error if the call failed.
pub fn extract_tx_hash_or_err(result: &Value) -> anyhow::Result<String> {
    if result["ok"].as_bool() != Some(true) {
        let err_msg = result["error"].as_str()
            .or_else(|| result["message"].as_str())
            .unwrap_or("unknown error");
        return Err(anyhow::anyhow!("contract-call failed: {}", err_msg));
    }
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no txHash in contract-call response"))
}

/// Run `onchainos security token-scan` and return the parsed JSON result.
/// Uses `--tokens <chainId>:<address>` format as required by the onchainos CLI.
pub fn security_token_scan(chain_id: u64, token_addr: &str) -> anyhow::Result<Value> {
    let tokens_arg = format!("{}:{}", chain_id, token_addr);
    run_onchainos(&["security", "token-scan", "--tokens", &tokens_arg])
}

/// Run `onchainos token info` for a contract address.
pub fn token_info(chain_id: u64, token_addr: &str) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    run_onchainos(&["token", "info", "--address", token_addr, "--chain", &chain_str])
}

/// Run `onchainos token price-info` for a contract address.
pub fn token_price_info(chain_id: u64, token_addr: &str) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    run_onchainos(&["token", "price-info", "--address", token_addr, "--chain", &chain_str])
}

/// Run `onchainos wallet status` and return JSON.
pub fn wallet_status() -> anyhow::Result<Value> {
    run_onchainos(&["wallet", "status"])
}

/// Run `onchainos wallet addresses` and return the first EVM address.
pub fn wallet_addresses() -> anyhow::Result<String> {
    let json = run_onchainos(&["wallet", "addresses"])?;
    Ok(json["data"]["evm"]
        .get(0)
        .and_then(|v| v["address"].as_str())
        .unwrap_or("")
        .to_string())
}
