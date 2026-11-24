// src/onchainos.rs
use std::process::Command;
use serde_json::Value;

/// `--biz-type` / `--strategy`: attribution to the onchainos backend.
/// Source-of-truth for the plugin name is Cargo.toml's `[package]` `name`.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Query the currently logged-in wallet address for the given EVM chain.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", &chain_str])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}", e))?;
    let addr = json["data"]["evm"][0]["address"].as_str().unwrap_or("").to_string();
    Ok(addr)
}

/// Submit a contract call via onchainos wallet contract-call.
/// ⚠️  dry_run=true returns a simulated response immediately — contract-call does NOT support --dry-run.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
    dry_run: bool,
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

    let output = tokio::process::Command::new("onchainos").args(&args).output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Extract txHash from wallet contract-call response: {"ok":true,"data":{"txHash":"0x..."}}
/// Returns an error if the response indicates failure or if no txHash is present.
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

/// ERC-20 approve via wallet contract-call (approve(address,uint256) selector = 0x095ea7b3)
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let spender_padded = format!("{:0>64}", &spender[2..]);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run).await
}

/// Poll eth_getTransactionReceipt until the tx is confirmed or timeout.
/// Uses 20 attempts × 2s = 40s — sufficient for Base (~2s blocks) and Arbitrum (~0.25s blocks).
pub async fn wait_for_tx(tx_hash: &str, rpc_url: &str) {
    let client = reqwest::Client::new();
    for _ in 0..20u32 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });
        if let Ok(resp) = client.post(rpc_url).json(&body).send().await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if json.get("result").map(|r| !r.is_null()).unwrap_or(false) {
                    return;
                }
            }
        }
    }
    // Timeout — continue anyway; balance read may be slightly stale
}

/// wallet balance — returns native JSON output from onchainos.
pub fn wallet_balance(chain_id: u64) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    Ok(serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?)
}
