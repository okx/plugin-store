use tokio::process::Command;
use serde_json::Value;

pub async fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output().await?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    // Try data.address first (legacy), then data.details[0].tokenAssets[0].address
    if let Some(addr) = json["data"]["address"].as_str() {
        return Ok(addr.to_string());
    }
    if let Some(addr) = json["data"]["details"][0]["tokenAssets"][0]["address"].as_str() {
        return Ok(addr.to_string());
    }
    // Also try addresses endpoint via wallet addresses
    Ok(String::new())
}

/// dry_run=true: early return simulated response. Never pass --dry-run to onchainos.
/// force=true: pass --force to onchainos to bypass confirmation prompts and actually broadcast.
/// `--biz-type` / `--strategy`: attribution to the onchainos backend.
/// Source-of-truth for the plugin name is Cargo.toml's `[package]` `name`.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>,
    force: bool,
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
        input_data
    ];
    let amt_str;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }
    let from_str_owned;
    if let Some(f) = from {
        from_str_owned = f.to_string();
        args.extend_from_slice(&["--from", &from_str_owned]);
    }
        if force {
        args.push("--force");
    }
    let output = Command::new("onchainos").args(&args).output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Read-only eth_call via direct JSON-RPC to public Ethereum RPC endpoint.
/// onchainos wallet contract-call does not support --read-only; use direct RPC instead.
pub async fn eth_call(chain_id: u64, to: &str, input_data: &str) -> anyhow::Result<Value> {
    let rpc_url = match chain_id {
        1 => "https://ethereum.publicnode.com",
        _ => anyhow::bail!("Unsupported chain_id for eth_call: {}", chain_id),
    };
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": input_data },
            "latest"
        ],
        "id": 1
    });
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    // Return in a shape compatible with rpc::extract_return_data
    let result_hex = resp["result"].as_str().unwrap_or("0x").to_string();
    Ok(serde_json::json!({
        "ok": true,
        "data": { "result": result_hex }
    }))
}

/// Poll eth_getTransactionReceipt until the TX is mined or timeout expires.
/// Returns Ok(()) if status=1 (success), Err if reverted or not mined in time.
pub async fn wait_for_receipt(chain_id: u64, tx_hash: &str, timeout_secs: u64) -> anyhow::Result<()> {
    let rpc_url = match chain_id {
        1 => "https://ethereum.publicnode.com",
        _ => anyhow::bail!("Unsupported chain_id for receipt polling: {}", chain_id),
    };
    let client = reqwest::Client::new();
    let interval = std::time::Duration::from_secs(3);
    let max_attempts = (timeout_secs / 3).max(1);

    for attempt in 0..max_attempts {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });
        let resp: Value = client.post(rpc_url).json(&body).send().await?.json().await?;
        if let Some(receipt) = resp["result"].as_object() {
            let status = receipt["status"].as_str().unwrap_or("0x0");
            if status == "0x1" {
                let block = receipt["blockNumber"].as_str().unwrap_or("?");
                let block_num = u64::from_str_radix(block.trim_start_matches("0x"), 16).unwrap_or(0);
                eprintln!("  ✓ confirmed in block {}", block_num);
                return Ok(());
            } else {
                anyhow::bail!("Transaction {} reverted (status=0x0)", tx_hash);
            }
        }
        if attempt < max_attempts - 1 {
            eprintln!("  waiting for {} to be mined... ({}/{})", &tx_hash[..10], attempt + 1, max_attempts);
            tokio::time::sleep(interval).await;
        }
    }
    anyhow::bail!(
        "Transaction {} not mined after {}s — it may have been dropped. Check gas price and retry.",
        tx_hash, timeout_secs
    )
}

/// Query native ETH balance via eth_getBalance JSON-RPC.
pub async fn eth_get_balance(address: &str, chain_id: u64) -> anyhow::Result<u128> {
    let rpc_url = match chain_id {
        1 => "https://ethereum.publicnode.com",
        _ => anyhow::bail!("Unsupported chain_id for eth_getBalance: {}", chain_id),
    };
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [address, "latest"],
        "id": 1
    });
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_getBalance RPC error: {}", err);
    }
    let hex = resp["result"].as_str().unwrap_or("0x0");
    let hex = hex.trim_start_matches("0x");
    Ok(u128::from_str_radix(hex, 16).unwrap_or(0))
}

pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}

/// Like extract_tx_hash but returns an error if the hash is missing or "pending".
/// Use this for write operations where a missing hash means the TX was not broadcast.
pub fn extract_tx_hash_or_err(result: &Value, label: &str) -> anyhow::Result<String> {
    let hash = result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending");
    if hash == "pending" || hash.is_empty() {
        anyhow::bail!(
            "{} transaction was not broadcast (txHash missing). Response: {}",
            label,
            result
        );
    }
    Ok(hash.to_string())
}
