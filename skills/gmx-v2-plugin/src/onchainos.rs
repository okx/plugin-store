use std::process::Command;
use std::time::{Duration, Instant};
use serde_json::Value;

/// `--biz-type` / `--strategy`: attribution to the onchainos backend.
/// Source-of-truth for the plugin name is Cargo.toml's `[package]` `name`.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Resolve the current logged-in wallet address via onchainos wallet addresses.
/// Matches the EVM address for the given chain_id.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    // Find the EVM entry matching chain_id
    let chain_str = chain_id.to_string();
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_str) {
                let addr = entry["address"].as_str().unwrap_or("").to_string();
                if !addr.is_empty() {
                    return Ok(addr);
                }
            }
        }
        // All EVM addresses are the same; fall back to first entry
        if let Some(first) = evm_list.first() {
            return Ok(first["address"].as_str().unwrap_or("").to_string());
        }
    }
    Ok(String::new())
}

/// Call onchainos wallet contract-call.
/// dry_run=true returns a simulated response immediately without calling onchainos.
/// NOTE: onchainos wallet contract-call does NOT support --dry-run parameter.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>,
    dry_run: bool,
    confirm: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" },
            "calldata": input_data
        }));
    }

    let chain_str = chain_id.to_string();
    let mut args: Vec<String> = vec![
        "wallet".into(),
        "contract-call".into(),
        "--biz-type".into(),
        BIZ_TYPE.into(),
        "--strategy".into(),
        STRATEGY.into(),
        "--chain".into(),
        chain_str.clone(),
        "--to".into(),
        to.into(),
        "--input-data".into(),
        input_data.into(),
    ];
    if let Some(v) = amt {
        args.push("--amt".into());
        args.push(v.to_string());
    }
    if let Some(f) = from {
        args.push("--from".into());
        args.push(f.into());
    }
    if confirm {
        args.push("--force".into());
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: Value = serde_json::from_str(&stdout)?;
    if val["ok"].as_bool() == Some(false) {
        let err = val["error"].as_str().unwrap_or("unknown error");
        anyhow::bail!("onchainos contract-call failed: {}", err);
    }
    Ok(val)
}

/// Like wallet_contract_call but with an explicit gas limit to bypass estimation failures.
pub async fn wallet_contract_call_with_gas(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>,
    dry_run: bool,
    confirm: bool,
    gas: Option<u64>,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" },
            "calldata": input_data
        }));
    }

    let chain_str = chain_id.to_string();
    let mut args: Vec<String> = vec![
        "wallet".into(),
        "contract-call".into(),
        "--biz-type".into(),
        BIZ_TYPE.into(),
        "--strategy".into(),
        STRATEGY.into(),
        "--chain".into(),
        chain_str.clone(),
        "--to".into(),
        to.into(),
        "--input-data".into(),
        input_data.into(),
    ];
    if let Some(v) = amt {
        args.push("--amt".into());
        args.push(v.to_string());
    }
    if let Some(f) = from {
        args.push("--from".into());
        args.push(f.into());
    }
    if let Some(g) = gas {
        args.push("--gas-limit".into());
        args.push(g.to_string());
    }
    if confirm {
        args.push("--force".into());
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("onchainos contract-call returned empty stdout (exit {}): {}", output.status.code().unwrap_or(-1), stderr.trim());
    }
    let val: Value = serde_json::from_str(&stdout)?;
    if val["ok"].as_bool() == Some(false) {
        let err = val["error"].as_str().unwrap_or("unknown error");
        anyhow::bail!("onchainos contract-call failed: {}", err);
    }
    Ok(val)
}

/// Wait for a transaction to be confirmed (txStatus = SUCCESS or FAIL).
/// Polls `onchainos wallet history --tx-hash` every 2s up to `timeout_secs`.
/// Returns Ok(()) on SUCCESS, Err on FAIL or timeout.
pub fn wait_for_tx(chain_id: u64, tx_hash: &str, address: &str, timeout_secs: u64) -> anyhow::Result<()> {
    if tx_hash.is_empty() || tx_hash == "pending" {
        return Ok(());
    }
    let chain_str = chain_id.to_string();
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    eprintln!("Waiting for tx {} to confirm...", tx_hash);
    loop {
        let output = Command::new("onchainos")
            .args(["wallet", "history", "--chain", &chain_str,
                   "--address", address, "--tx-hash", tx_hash])
            .output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if let Ok(val) = serde_json::from_str::<Value>(&stdout) {
                let status = val["data"][0]["txStatus"].as_str().unwrap_or("");
                match status {
                    "SUCCESS" => { eprintln!("Tx confirmed."); return Ok(()); }
                    "FAIL" => anyhow::bail!("Approval transaction failed on-chain: {}", tx_hash),
                    _ => {}
                }
            }
        }
        if Instant::now() >= deadline {
            anyhow::bail!("Timeout waiting for tx {} to confirm ({}s)", tx_hash, timeout_secs);
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// Extract txHash from wallet contract-call response: {"ok":true,"data":{"txHash":"0x..."}}
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}

/// ERC-20 approve via wallet contract-call (no onchainos dex approve command)
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
    confirm: bool,
) -> anyhow::Result<Value> {
    // approve(address,uint256) selector = 0x095ea7b3
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run, confirm).await
}

/// Check ERC-20 allowance via eth_call
pub async fn check_allowance(
    rpc_url: &str,
    token_addr: &str,
    owner: &str,
    spender: &str,
) -> anyhow::Result<u128> {
    // allowance(address,address) selector = 0xdd62ed3e
    let owner_clean = owner.trim_start_matches("0x");
    let spender_clean = spender.trim_start_matches("0x");
    let calldata = format!(
        "0xdd62ed3e{:0>64}{:0>64}",
        owner_clean, spender_clean
    );
    let result = crate::rpc::eth_call(token_addr, &calldata, rpc_url).await?;
    // result is 32-byte hex
    let hex = result.trim_start_matches("0x");
    if hex.len() < 64 {
        return Ok(0);
    }
    let val = u128::from_str_radix(&hex[hex.len().saturating_sub(32)..], 16).unwrap_or(0);
    Ok(val)
}

/// wallet balance (for display)
pub fn wallet_balance(chain_id: u64) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    Ok(serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?)
}
