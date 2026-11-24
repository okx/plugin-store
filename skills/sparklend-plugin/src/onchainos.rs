use anyhow::Context;
use serde_json::Value;
use std::process::Command;

/// Build a base Command for onchainos, explicitly adding ~/.local/bin to PATH.
fn base_cmd() -> Command {
    let mut cmd = Command::new("onchainos");
    let home = std::env::var("HOME").unwrap_or_default();
    let existing_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}/.local/bin:{}", home, existing_path);
    cmd.env("PATH", path);
    cmd
}

/// Run a Command and return its stdout as a parsed JSON Value.
fn run_cmd(mut cmd: Command) -> anyhow::Result<Value> {
    let output = cmd.output().context("Failed to spawn onchainos process")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "onchainos exited with status {}: stderr={} stdout={}",
            output.status.code().unwrap_or(-1),
            stderr.trim(),
            stdout.trim()
        );
    }
    serde_json::from_str(stdout.trim())
        .with_context(|| format!("Failed to parse onchainos JSON output: {}", stdout.trim()))
}

/// Resolve a token symbol or address to (contract_address, decimals).
/// Queries onchainos token search to get actual decimals.
pub fn resolve_token(asset: &str, _chain_id: u64) -> anyhow::Result<(String, u8)> {
    let is_address = asset.starts_with("0x") && asset.len() == 42;
    let mut cmd = base_cmd();
    cmd.args(["token", "search", "--query", asset, "--chain", "ethereum"]);
    let result = run_cmd(cmd)?;

    let tokens = result
        .as_array()
        .or_else(|| result.get("data").and_then(|d| d.as_array()))
        .ok_or_else(|| anyhow::anyhow!("No tokens found for '{}' on Ethereum", asset))?;

    let first = tokens.first().ok_or_else(|| {
        anyhow::anyhow!("No token match for '{}' on Ethereum", asset)
    })?;

    let addr = if is_address {
        asset.to_lowercase()
    } else {
        first["tokenContractAddress"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing tokenContractAddress in token search result"))?
            .to_lowercase()
    };

    let decimals = first["decimal"]
        .as_str()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(18);

    Ok((addr, decimals))
}

/// Submit a contract call via onchainos wallet contract-call.
///
/// If dry_run is true, prints the command that would be run and returns a mock
/// success JSON without actually executing it.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let mut args: Vec<String> = vec![
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_id.to_string(),
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        input_data.to_string(),
    ];
    if let Some(addr) = from {
        args.push("--from".to_string());
        args.push(addr.to_string());
    }
    args.push("--biz-type".to_string());
    args.push("dapp".to_string());
    args.push("--strategy".to_string());
    args.push("sparklend-plugin".to_string());
    if dry_run {
        args.push("--dry-run".to_string());
        let cmd_str = format!("onchainos {}", args.join(" "));
        eprintln!("[dry-run] would execute: {}", cmd_str);
        return Ok(serde_json::json!({
            "ok": true,
            "dryRun": true,
            "simulatedCommand": cmd_str
        }));
    }
    args.push("--force".to_string());
    let mut cmd = base_cmd();
    cmd.args(&args);
    run_cmd(cmd)
}

/// Same as wallet_contract_call but attaches a native ETH value (--amt).
/// Used for WETH.deposit() and similar payable calls.
pub fn wallet_contract_call_with_value(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    value_wei: u128,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let mut args: Vec<String> = vec![
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_id.to_string(),
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        input_data.to_string(),
        "--amt".to_string(),
        value_wei.to_string(),
    ];
    if let Some(addr) = from {
        args.push("--from".to_string());
        args.push(addr.to_string());
    }
    args.push("--biz-type".to_string());
    args.push("dapp".to_string());
    args.push("--strategy".to_string());
    args.push("sparklend-plugin".to_string());
    if dry_run {
        args.push("--dry-run".to_string());
        let cmd_str = format!("onchainos {}", args.join(" "));
        eprintln!("[dry-run] would execute: {}", cmd_str);
        return Ok(serde_json::json!({
            "ok": true,
            "dryRun": true,
            "simulatedCommand": cmd_str
        }));
    }
    args.push("--force".to_string());
    let mut cmd = base_cmd();
    cmd.args(&args);
    run_cmd(cmd)
}

/// Get the currently active EVM wallet address for the given chain.
pub fn wallet_address(chain_id: u64) -> anyhow::Result<String> {
    let mut cmd = base_cmd();
    cmd.args(["wallet", "addresses", "--chain", &chain_id.to_string()]);
    let result = run_cmd(cmd)?;
    result["data"]["evm"][0]["address"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not resolve wallet address from onchainos wallet addresses"))
}
