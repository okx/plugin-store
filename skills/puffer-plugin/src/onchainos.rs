use std::process::Command;
use serde_json::Value;

/// `--biz-type` / `--strategy`: attribution to the onchainos backend (since
///   onchainos 3.0.0) so analytics can group calls by source plugin.
/// Single source of truth: `env!` resolves Cargo.toml's `name` field at compile time.
/// CI invariant — Cargo.toml.name === plugin.yaml.name.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Helper: invoke `onchainos` with the given args, return parsed stdout JSON.
/// Fails if the process exits non-zero or stdout is not valid JSON.
fn onchainos_json(args: &[&str]) -> anyhow::Result<Value> {
    let output = Command::new("onchainos").args(args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("onchainos {:?} returned non-JSON stdout: {} (error: {})", args, stdout, e))?;
    if v.get("ok").and_then(|x| x.as_bool()) == Some(false) {
        let msg = v.pointer("/error")
            .or_else(|| v.pointer("/message"))
            .or_else(|| v.pointer("/data/message"))
            .and_then(|x| x.as_str())
            .unwrap_or("ok:false");
        // Include the sub-command but not the full args (keeps error readable when classifier does substring match).
        let cmd = args.iter().take(2).copied().collect::<Vec<_>>().join(" ");
        anyhow::bail!("onchainos {}: {}", cmd, msg);
    }
    Ok(v)
}

/// onchainos gateway gas --chain <chain>
/// Returns the "normal" gas price in wei as u128.
pub async fn gas_price_wei(chain: &str) -> anyhow::Result<u128> {
    let chain_owned = chain.to_string();
    tokio::task::spawn_blocking(move || {
        let v = onchainos_json(&["gateway", "gas", "--chain", &chain_owned])?;
        let s = v.pointer("/data/0/normal")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("gateway gas: missing data[0].normal: {}", v))?;
        s.parse::<u128>().map_err(|e| anyhow::anyhow!("gateway gas: parse normal '{}': {}", s, e))
    })
    .await
    .map_err(|e| anyhow::anyhow!("spawn_blocking error: {}", e))?
}

/// onchainos gateway gas-limit --from --to --chain [--amount] [--data]
/// Returns the gas limit as u128.
pub async fn gas_limit(
    chain: &str,
    from: &str,
    to: &str,
    value_wei: u128,
    data: &str,
) -> anyhow::Result<u128> {
    let chain = chain.to_string();
    let from = from.to_string();
    let to = to.to_string();
    let amount = value_wei.to_string();
    let data = data.to_string();
    tokio::task::spawn_blocking(move || {
        let v = onchainos_json(&[
            "gateway", "gas-limit",
            "--chain", &chain,
            "--from", &from,
            "--to", &to,
            "--amount", &amount,
            "--data", &data,
        ])?;
        let s = v.pointer("/data/0/gasLimit")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("gateway gas-limit: missing data[0].gasLimit: {}", v))?;
        s.parse::<u128>().map_err(|e| anyhow::anyhow!("gateway gas-limit: parse '{}': {}", s, e))
    })
    .await
    .map_err(|e| anyhow::anyhow!("spawn_blocking error: {}", e))?
}

/// onchainos wallet balance --chain <chain_id> [--token-address <addr>] [--force]
/// Returns the connected wallet's balance for the given token (or native ETH if token_address is None).
/// `force=true` bypasses onchainos's balance cache — use for post-tx reads.
///
/// For native ETH, looks up the first entry in data.details[0].tokenAssets where tokenAddress is empty.
/// For ERC-20, filters by tokenAddress (case-insensitive).
pub async fn wallet_balance(
    chain_id: u64,
    token_address: Option<&str>,
    force: bool,
) -> anyhow::Result<u128> {
    let chain = chain_id.to_string();
    let token = token_address.map(|s| s.to_lowercase());
    tokio::task::spawn_blocking(move || {
        let mut args: Vec<&str> = vec!["wallet", "balance", "--chain", &chain];
        let token_ref = token.as_deref();
        if let Some(t) = token_ref {
            args.push("--token-address");
            args.push(t);
        }
        if force {
            args.push("--force");
        }
        let v = onchainos_json(&args)?;
        let assets = v.pointer("/data/details/0/tokenAssets")
            .and_then(|x| x.as_array())
            .ok_or_else(|| anyhow::anyhow!("wallet balance: missing data.details[0].tokenAssets: {}", v))?;

        let target = token_ref.map(|s| s.to_lowercase());
        let found = assets.iter().find(|a| {
            let addr = a.get("tokenAddress").and_then(|x| x.as_str()).unwrap_or("").to_lowercase();
            match &target {
                Some(t) => &addr == t,
                None => addr.is_empty(), // native token has empty tokenAddress
            }
        });
        let raw = found
            .and_then(|a| a.get("rawBalance").or_else(|| a.get("balance")))
            .and_then(|x| x.as_str())
            .unwrap_or("0");
        raw.parse::<u128>().map_err(|e| anyhow::anyhow!("wallet balance: parse rawBalance '{}': {}", raw, e))
    })
    .await
    .map_err(|e| anyhow::anyhow!("spawn_blocking error: {}", e))?
}

/// Poll onchainos wallet history until txStatus is SUCCESS or FAILED (or 90s timeout).
/// Uses spawn_blocking so Command::output() doesn't block the Tokio runtime thread.
pub async fn wait_for_tx(tx_hash: String, wallet_addr: String) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || wait_for_tx_sync(&tx_hash, &wallet_addr))
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking error: {}", e))?
}

fn wait_for_tx_sync(tx_hash: &str, wallet_addr: &str) -> anyhow::Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    loop {
        if std::time::Instant::now() > deadline {
            anyhow::bail!("Timeout (90s) waiting for tx {} to confirm", tx_hash);
        }
        let output = Command::new("onchainos")
            .args([
                "wallet", "history",
                "--tx-hash", tx_hash,
                "--address", wallet_addr,
                "--chain", "1",
            ])
            .output();
        if let Ok(out) = output {
            if let Ok(v) = serde_json::from_str::<Value>(&String::from_utf8_lossy(&out.stdout)) {
                if let Some(entry) = v["data"].as_array().and_then(|a| a.first()) {
                    match entry["txStatus"].as_str() {
                        Some("SUCCESS") => return Ok(()),
                        Some("FAILED") => {
                            let reason = entry["failReason"].as_str().unwrap_or("");
                            anyhow::bail!("approve tx {} failed on-chain: {}", tx_hash, reason);
                        }
                        _ => {} // PENDING — keep polling
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}

/// Resolve the EVM wallet address for Ethereum (chain_id=1) from the onchainos CLI.
/// Parses `onchainos wallet addresses` JSON and returns the first matching EVM address.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    let chain_id_str = chain_id.to_string();
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_id_str) {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
        // Fallback: use first EVM address
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not resolve wallet address for chain {}", chain_id)
}

/// Execute a contract call via `onchainos wallet contract-call`.
///
/// Parameters:
/// - `chain_id`    — Ethereum chain ID (1 for mainnet)
/// - `to`          — target contract address
/// - `input_data`  — ABI-encoded calldata (0x-prefixed hex)
/// - `value_wei`   — native ETH to send as msg.value (0 for non-payable calls)
/// - `confirm`     — if false, returns a preview JSON without broadcasting;
///                   if true, broadcasts the transaction
/// - `dry_run`     — if true, returns mock response without calling onchainos
///
/// **Confirm gate**: Write operations always preview first. The caller must pass
/// `confirm=true` (via `--confirm` flag) to actually broadcast.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    value_wei: u128,
    confirm: bool,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {"txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"},
            "calldata": input_data,
            "value": value_wei.to_string()
        }));
    }

    if !confirm {
        // Preview mode: show what would be sent but do NOT broadcast
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "message": "Run with --confirm to broadcast this transaction.",
            "to": to,
            "calldata": input_data,
            "value_wei": value_wei.to_string(),
            "chain_id": chain_id
        }));
    }

    let chain_str = chain_id.to_string();
    let value_str = value_wei.to_string();
    let output = Command::new("onchainos")
        .args([
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
            "--amt",
            &value_str,
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| serde_json::json!({"ok": false, "raw": stdout.to_string()}));

    // Propagate ok:false as an error — prevents "pending" txHash on simulation rejection
    if result["ok"].as_bool() == Some(false) {
        let msg = result["message"].as_str()
            .or_else(|| result["data"]["message"].as_str())
            .or_else(|| result["raw"].as_str())
            .unwrap_or("onchainos wallet contract-call failed (ok: false)");
        anyhow::bail!("{}", msg);
    }

    Ok(result)
}

/// Extract txHash from a wallet_contract_call response.
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}
