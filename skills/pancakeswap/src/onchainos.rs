/// Wrapper for `onchainos wallet contract-call` CLI.

pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
    dry_run: bool,
    confirm: bool,
) -> anyhow::Result<serde_json::Value> {
    if dry_run {
        // Return a preview without broadcasting — caller already prints dry-run info
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "calldata": input_data
        }));
    }

    if !confirm {
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "message": "Add --confirm to broadcast"
        }));
    }

    let chain_str = chain_id.to_string();
    let mut args = vec![
        "wallet",
        "contract-call",
        "--chain",
        &chain_str,
        "--to",
        to,
        "--input-data",
        input_data,
    ];
    if confirm {
        args.push("--force");
    }

    let amt_str: String;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }

    let from_str: String;
    if let Some(f) = from {
        from_str = f.to_string();
        args.extend_from_slice(&["--from", &from_str]);
    }

    let out = tokio::process::Command::new("onchainos")
        .args(&args)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.trim().is_empty() {
        // Return stderr in a structured way for debugging
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("onchainos returned empty output. stderr: {}", stderr);
    }

    let v: serde_json::Value = serde_json::from_str(&stdout)?;
    if v.get("ok").and_then(|b| b.as_bool()) == Some(false) {
        let msg = v.get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown onchainos error");
        anyhow::bail!("onchainos error: {}", msg);
    }
    Ok(v)
}

pub fn extract_tx_hash(r: &serde_json::Value) -> &str {
    r["data"]["txHash"]
        .as_str()
        .or_else(|| r["txHash"].as_str())
        .unwrap_or("pending")
}

/// Poll eth_getTransactionReceipt until the tx is mined (up to ~60s), then
/// return Err if the receipt shows status 0x0 (reverted). This prevents
/// false-success reporting when a broadcast tx reverts on-chain (e.g. the
/// mint() "Price slippage check" revert that was previously reported as
/// "LP position minted successfully!").
pub async fn wait_and_check_receipt(tx_hash: &str, rpc_url: &str) -> anyhow::Result<()> {
    if !tx_hash.starts_with("0x") || tx_hash.len() < 10 {
        anyhow::bail!(
            "Transaction was not broadcast (invalid tx hash: '{}').",
            tx_hash
        );
    }
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionReceipt",
        "params": [tx_hash],
        "id": 1
    });

    for attempt in 0..12u32 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
        let resp: serde_json::Value = match client.post(rpc_url).json(&body).send().await {
            Ok(r) => match r.json().await {
                Ok(v) => v,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        let result = &resp["result"];
        if result.is_null() {
            continue; // not mined yet
        }

        let status = result["status"].as_str().unwrap_or("0x0");
        if status == "0x0" || status == "0" {
            anyhow::bail!(
                "Transaction {} reverted on-chain (status=0x0). \
                 Check slippage tolerance or tick range and retry.",
                tx_hash
            );
        }
        return Ok(());
    }

    // Timed out — warn but don't hard-fail
    eprintln!(
        "  [warn] Could not confirm receipt for {} within 60s — verify on-chain before assuming success.",
        tx_hash
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BSC_RPC: &str = "https://bsc-rpc.publicnode.com";

    /// A real BSC transaction that reverted on-chain (status=0x0).
    /// Verified via eth_getTransactionReceipt before adding this test.
    const REVERTED_TX: &str =
        "0x8b267fbff3eb29cac16e48a2a1ff920a72cce3361c74c42fd4ede04dbd28aa8f";

    /// A real BSC transaction that succeeded on-chain (status=0x1).
    /// From PR #100 T6: addLiquidityETH 0.5 USDT + 0.000825 BNB on BSC.
    const SUCCESS_TX: &str =
        "0xce2e4fa2d03339dc428d80bdc63ca2fc152397235abd66d21b588a96e1d86041";

    /// Core bug regression: a reverted tx must return Err, not Ok.
    /// Before this fix, wait_and_check_receipt did not exist — callers
    /// would print "LP position minted successfully!" even for status=0x0.
    #[tokio::test]
    async fn receipt_reverted_returns_err() {
        let result = wait_and_check_receipt(REVERTED_TX, BSC_RPC).await;
        assert!(
            result.is_err(),
            "Expected Err for reverted tx but got Ok — false-success bug is still present"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("reverted on-chain"),
            "Error message should mention 'reverted on-chain', got: {msg}"
        );
    }

    /// Happy path: a successful tx must still return Ok so normal flow is unaffected.
    #[tokio::test]
    async fn receipt_success_returns_ok() {
        let result = wait_and_check_receipt(SUCCESS_TX, BSC_RPC).await;
        assert!(
            result.is_ok(),
            "Expected Ok for successful tx but got Err: {:?}",
            result.unwrap_err()
        );
    }

    /// extract_tx_hash must work for both response shapes onchainos can return.
    #[test]
    fn extract_tx_hash_nested_data() {
        let v = serde_json::json!({"data": {"txHash": "0xabc"}});
        assert_eq!(extract_tx_hash(&v), "0xabc");
    }

    #[test]
    fn extract_tx_hash_flat() {
        let v = serde_json::json!({"txHash": "0xdef"});
        assert_eq!(extract_tx_hash(&v), "0xdef");
    }

    #[test]
    fn extract_tx_hash_missing_falls_back_to_pending() {
        let v = serde_json::json!({"ok": false});
        assert_eq!(extract_tx_hash(&v), "pending");
    }

    /// If onchainos returns ok:false (simulation rejection), wait_and_check_receipt
    /// must immediately fail rather than polling and timing out as a soft-success.
    #[tokio::test]
    async fn receipt_pending_hash_returns_err() {
        let result = wait_and_check_receipt("pending", BSC_RPC).await;
        assert!(
            result.is_err(),
            "Expected Err for 'pending' hash but got Ok — ok:false path would silently succeed"
        );
    }

    #[tokio::test]
    async fn receipt_empty_hash_returns_err() {
        let result = wait_and_check_receipt("", BSC_RPC).await;
        assert!(result.is_err());
    }
}

/// Fetch the wallet's EVM address for a given chain via `onchainos wallet addresses`.
/// Returns the first EVM address found (all chains share the same EVM address).
pub async fn get_wallet_address() -> anyhow::Result<String> {
    let out = tokio::process::Command::new("onchainos")
        .args(&["wallet", "addresses"])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse wallet addresses: {}", e))?;
    v["data"]["evm"][0]["address"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not find EVM address in wallet addresses response"))
}
