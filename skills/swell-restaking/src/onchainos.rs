use std::process::Command;
use serde_json::Value;

/// Resolve the active EVM wallet address for the given chain.
/// ⚠️  --output json not supported on Ethereum mainnet (chain 1).
///     Uses `wallet balance --chain <id>` (no --output json) and parses JSON from stdout.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])  // no --output json for chain 1
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    // Try data.details[0].tokenAssets[0].address first (most reliable for chain 1)
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    // Fallback: data.address
    if let Some(addr) = json["data"]["address"].as_str() {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    anyhow::bail!("Could not resolve wallet address for chain {}. Is onchainos logged in?", chain_id)
}

/// Submit an EVM contract call through onchainos CLI.
///
/// ⚠️  dry_run=true: returns a simulated response immediately, without calling onchainos.
///     onchainos wallet contract-call does NOT support --dry-run.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>, // ETH value in wei (for payable calls)
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "calldata": input_data
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
        "--force",
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

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Read-only eth_call via direct JSON-RPC to the public Ethereum RPC endpoint.
/// onchainos does not support read-only calls; use direct RPC for all eth_call queries.
pub fn eth_call(chain_id: u64, to: &str, input_data: &str) -> anyhow::Result<Value> {
    let rpc_url = match chain_id {
        1 => "https://ethereum.publicnode.com",
        8453 => "https://base-rpc.publicnode.com",
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

    let client = build_client()?;
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()?
        .json()?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    let result_hex = resp["result"].as_str().unwrap_or("0x").to_string();
    Ok(serde_json::json!({
        "ok": true,
        "data": { "result": result_hex }
    }))
}

/// Extract txHash from an onchainos response.
/// Checks data.txHash first, then root txHash.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// Build a reqwest blocking client with proxy support.
/// ⚠️  reqwest does not read system proxy env vars by default.
///     This function explicitly reads HTTPS_PROXY / HTTP_PROXY.
fn build_client() -> anyhow::Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        builder = builder.proxy(reqwest::Proxy::https(&proxy_url)?);
    } else if let Ok(proxy_url) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
        builder = builder.proxy(reqwest::Proxy::http(&proxy_url)?);
    }
    Ok(builder.build()?)
}
