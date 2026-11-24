use anyhow::Context;
use std::process::Command;

pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()
        .context("onchainos not found — install from https://docs.onchainos.com")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim())
        .context("Could not parse onchainos wallet output")?;
    let chain_str = chain_id.to_string();
    if let Some(evm_list) = v["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_str) {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
    }
    anyhow::bail!("No EVM wallet found for chain {}. Run: onchainos wallet login", chain_id)
}

pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    value: &str,   // hex (0x...) or decimal string — will be converted to decimal wei
    _wait: bool,
    dry_run: bool,
    from: Option<&str>,
) -> anyhow::Result<String> {
    if dry_run {
        // Return a stub result without calling onchainos
        return Ok(serde_json::json!({
            "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "dry_run": true
        }).to_string());
    }

    // Convert value from hex or decimal to decimal integer for --amt
    let amt = if value.starts_with("0x") || value.starts_with("0X") {
        u128::from_str_radix(value.trim_start_matches("0x").trim_start_matches("0X"), 16)
            .unwrap_or(0)
            .to_string()
    } else if value.is_empty() {
        "0".to_string()
    } else {
        value.to_string()
    };

    let mut args = vec![
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(), chain_id.to_string(),
        "--to".to_string(), to.to_string(),
        "--input-data".to_string(), input_data.to_string(),
        "--amt".to_string(), amt,
        "--biz-type".to_string(), "dapp".to_string(),
        "--strategy".to_string(), "relay-plugin".to_string(),
    ];
    if let Some(f) = from { args.extend(["--from".to_string(), f.to_string()]); }

    let output = Command::new("onchainos").args(&args).output()
        .context("onchainos not found")?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        anyhow::bail!("onchainos error: {}", stderr.trim());
    }
    // Surface onchainos errors rather than returning zero tx_hash silently
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(stdout.trim()) {
        if v.get("ok").and_then(|o| o.as_bool()) == Some(false) {
            let msg = v.get("error").and_then(|e| e.as_str()).unwrap_or("unknown error");
            anyhow::bail!("onchainos error: {}", msg);
        }
    }
    Ok(stdout)
}

pub fn extract_tx_hash(output: &str) -> String {
    serde_json::from_str::<serde_json::Value>(output.trim())
        .ok()
        .and_then(|v| {
            // onchainos returns {"ok":true,"data":{"txHash":"0x..."}}
            v.pointer("/data/txHash")
                .or_else(|| v.pointer("/data/tx_hash"))
                .or_else(|| v.get("txHash"))
                .or_else(|| v.get("tx_hash"))
                .and_then(|h| h.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "0x0000000000000000000000000000000000000000000000000000000000000000".to_string())
}
