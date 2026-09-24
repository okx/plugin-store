use anyhow::Context;
use serde_json::Value;

/// `--biz-type` / `--strategy`: attribution to the onchainos backend.
/// Source-of-truth for the plugin name is Cargo.toml's `[package]` `name`.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Call `onchainos wallet contract-call` and return parsed JSON output.
/// Set `force=true` to append `--force` and broadcast immediately (use only for token approvals).
/// For main protocol operations (supply, borrow, repay, withdraw, claim), use `force=false` —
/// onchainos will present the transaction for user confirmation before broadcasting.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>,
    dry_run: bool,
    force: bool,
) -> anyhow::Result<Value> {
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
    let from_str;
    if let Some(f) = from {
        from_str = f.to_string();
        args.extend_from_slice(&["--from", &from_str]);
    }
    // In dry-run mode, just print the command that would be executed and return a simulated response.
    if dry_run {
        eprintln!("[morpho] [dry-run] Would run: onchainos {}", args.join(" "));
        return Ok(serde_json::json!({
            "ok": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }
        }));
    }

    if force {
        args.push("--force");
    }

    let output = tokio::process::Command::new("onchainos")
        .args(&args)
        .output()
        .await
        .context("Failed to spawn onchainos wallet contract-call")?;
    parse_command_output(
        output.status.success(),
        output.status.code(),
        &output.stdout,
        &output.stderr,
    )
}

fn output_summary(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    if trimmed.chars().count() <= 500 {
        trimmed.to_string()
    } else {
        format!("{}…", trimmed.chars().take(500).collect::<String>())
    }
}

fn parse_command_output(
    success: bool,
    exit_code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
) -> anyhow::Result<Value> {
    let stdout_summary = output_summary(stdout);
    let stderr_summary = output_summary(stderr);
    if !success {
        anyhow::bail!(
            "onchainos exited with status {}: stderr={} stdout={}",
            exit_code.unwrap_or(-1),
            if stderr_summary.is_empty() {
                "<empty>"
            } else {
                &stderr_summary
            },
            if stdout_summary.is_empty() {
                "<empty>"
            } else {
                &stdout_summary
            },
        );
    }

    let value: Value = serde_json::from_slice(stdout).with_context(|| {
        format!(
            "Failed to parse onchainos JSON output: {}",
            if stdout_summary.is_empty() {
                "<empty>"
            } else {
                &stdout_summary
            }
        )
    })?;
    if value.get("ok").and_then(Value::as_bool) == Some(false) {
        let message = value
            .get("error")
            .and_then(Value::as_str)
            .or_else(|| value.get("message").and_then(Value::as_str))
            .unwrap_or("unknown structured error");
        anyhow::bail!("onchainos wallet contract-call failed: {}", message);
    }
    Ok(value)
}

/// Extract txHash from wallet contract-call response, returning an error if the call failed.
/// Response format: {"ok":true,"data":{"txHash":"0x..."}}
pub fn extract_tx_hash_or_err(result: &Value) -> anyhow::Result<String> {
    if result["ok"].as_bool() != Some(true) {
        let err_msg = result["error"]
            .as_str()
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

/// Encode and submit an ERC-20 approve call.
/// Selector: 0x095ea7b3
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // approve(address,uint256) selector = 0x095ea7b3
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    // Approvals always use --force: they are prerequisite steps, not the main user action
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run, true).await
}

/// Query wallet balance for the given chain. Returns raw JSON from onchainos.
pub async fn wallet_balance(chain_id: u64) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    let output = tokio::process::Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Resolve the caller's wallet address: use `from` if provided, otherwise
/// query the active onchainos wallet via `wallet addresses --chain <id>`.
pub async fn resolve_wallet(from: Option<&str>, chain_id: u64) -> anyhow::Result<String> {
    if let Some(addr) = from {
        return Ok(addr.to_string());
    }
    let chain_str = chain_id.to_string();
    let output = tokio::process::Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", &chain_str])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}\nraw: {}", e, stdout))?;
    let addr = v["data"]["evm"][0]["address"]
        .as_str()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Could not determine active EVM wallet address. Ensure onchainos is logged in."
            )
        })?
        .to_string();
    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonzero_subprocess_exit_preserves_stderr_and_stdout() {
        let err = parse_command_output(
            false,
            Some(17),
            br#"{"ok":false,"error":"wallet rejected"}"#,
            b"signer unavailable",
        )
        .unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("status 17"), "got: {message}");
        assert!(message.contains("signer unavailable"), "got: {message}");
        assert!(message.contains("wallet rejected"), "got: {message}");
    }

    #[test]
    fn structured_failure_uses_downstream_error() {
        let err = parse_command_output(
            true,
            Some(0),
            br#"{"ok":false,"error":"simulation reverted: insufficient allowance"}"#,
            b"",
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("simulation reverted: insufficient allowance"),
            "got: {err:#}"
        );
    }

    #[test]
    fn invalid_json_keeps_raw_output() {
        let err =
            parse_command_output(true, Some(0), b"not-json from downstream", b"").unwrap_err();
        assert!(
            err.to_string().contains("not-json from downstream"),
            "got: {err:#}"
        );
    }
}
