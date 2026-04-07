use anyhow::Context;
use serde_json::Value;
use std::process::Command;

/// Build a base Command for onchainos, adding ~/.local/bin to PATH.
fn base_cmd() -> Command {
    let mut cmd = Command::new("onchainos");
    let home = std::env::var("HOME").unwrap_or_default();
    let existing_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}/.local/bin:{}", home, existing_path);
    cmd.env("PATH", path);
    cmd
}

/// Run a Command and return its stdout as a parsed JSON Value.
/// Handles exit code 2 (onchainos confirming response) by retrying with --force.
fn run_cmd(mut cmd: Command) -> anyhow::Result<Value> {
    let output = cmd.output().context("Failed to spawn onchainos process")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let exit_code = output.status.code().unwrap_or(-1);

    if exit_code == 2 {
        let confirming: Value = serde_json::from_str(stdout.trim())
            .unwrap_or(serde_json::json!({"confirming": true}));
        if confirming
            .get("confirming")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let mut force_cmd = cmd;
            force_cmd.arg("--force");
            let force_output = force_cmd
                .output()
                .context("Failed to spawn onchainos --force process")?;
            let force_stdout = String::from_utf8_lossy(&force_output.stdout);
            return serde_json::from_str(force_stdout.trim()).with_context(|| {
                format!(
                    "Failed to parse onchainos --force JSON: {}",
                    force_stdout.trim()
                )
            });
        }
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "onchainos exited {}: stderr={} stdout={}",
            exit_code,
            stderr.trim(),
            stdout.trim()
        );
    }
    serde_json::from_str(stdout.trim())
        .with_context(|| format!("Failed to parse onchainos JSON: {}", stdout.trim()))
}

/// Resolve the wallet address for a given chain.
/// If dry_run is true, returns the zero address without calling onchainos.
pub fn resolve_wallet(chain_id: u64, dry_run: bool) -> anyhow::Result<String> {
    if dry_run {
        return Ok("0x0000000000000000000000000000000000000000".to_string());
    }
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()
        .context("Failed to run onchainos wallet addresses")?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .context("Failed to parse wallet addresses JSON")?;
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
    anyhow::bail!("Could not resolve wallet address for chain {}", chain_id)
}

/// Submit a contract call via `onchainos wallet contract-call`.
/// --force is always appended for write operations.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        let cmd_str = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
            chain_id, to, input_data
        );
        eprintln!("[dry-run] {}", cmd_str);
        return Ok(serde_json::json!({
            "ok": true,
            "dryRun": true,
            "simulatedCommand": cmd_str
        }));
    }
    let mut cmd = base_cmd();
    cmd.args([
        "wallet",
        "contract-call",
        "--chain",
        &chain_id.to_string(),
        "--to",
        to,
        "--input-data",
        input_data,
        "--force",
    ]);
    run_cmd(cmd)
}

/// Extract txHash from onchainos contract-call response.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .or_else(|| result["hash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// Encode ERC-20 approve(spender, uint256.max) calldata.
pub fn encode_approve(spender: &str, amount: u128) -> String {
    let spender_padded = crate::rpc::encode_address(spender);
    let amount_padded = crate::rpc::encode_u256(amount);
    format!("0x095ea7b3{}{}", spender_padded, amount_padded)
}
