/// onchainos CLI wrappers for Polymarket on-chain operations.
use anyhow::{Context, Result};
use serde_json::Value;

const CHAIN: &str = "137";

/// Sign an EIP-712 structured data JSON via `onchainos sign-message --type eip712`.
///
/// The JSON must include EIP712Domain in the `types` field — this is required for correct
/// hash computation (per Hyperliquid root-cause finding).
///
/// Returns the 0x-prefixed signature hex string.
pub async fn sign_eip712(structured_data_json: &str) -> Result<String> {
    // Resolve the wallet address to pass as --from
    let wallet_addr = get_wallet_address().await
        .context("Failed to resolve wallet address for sign-message")?;

    let output = tokio::process::Command::new("onchainos")
        .args([
            "wallet", "sign-message",
            "--type", "eip712",
            "--message", structured_data_json,
            "--chain", CHAIN,
            "--from", &wallet_addr,
            "--force",
        ])
        .output()
        .await
        .context("Failed to spawn onchainos wallet sign-message")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("onchainos sign-message failed ({}): {}", output.status, stderr.trim());
    }

    let v: Value = serde_json::from_str(stdout.trim())
        .with_context(|| format!("parsing sign-message output: {}", stdout.trim()))?;

    // Try data.signature first, then top-level signature
    v["data"]["signature"]
        .as_str()
        .or_else(|| v["signature"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no signature in onchainos output: {}", stdout.trim()))
}

/// Call `onchainos wallet contract-call --chain 137 --to <to> --input-data <data> --force`
pub async fn wallet_contract_call(to: &str, input_data: &str) -> Result<Value> {
    let output = tokio::process::Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            CHAIN,
            "--to",
            to,
            "--input-data",
            input_data,
            "--force",
        ])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("wallet contract-call parse error: {}\nraw: {}", e, stdout))
}

/// Extract txHash from wallet contract-call response.
pub fn extract_tx_hash(result: &Value) -> anyhow::Result<String> {
    if result["ok"].as_bool() != Some(true) {
        let msg = result["error"]
            .as_str()
            .or_else(|| result["message"].as_str())
            .unwrap_or("unknown error");
        return Err(anyhow::anyhow!("contract-call failed: {}", msg));
    }
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no txHash in contract-call response"))
}

/// Get the wallet address from `onchainos wallet addresses --chain 137`.
/// Parses: data.evm[0].address
pub async fn get_wallet_address() -> Result<String> {
    let output = tokio::process::Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", CHAIN])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}\nraw: {}", e, stdout))?;
    v["data"]["evm"][0]["address"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not determine wallet address from onchainos output"))
}

/// Pad a hex address to 32 bytes (64 hex chars), no 0x prefix.
fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u256 value to 32 bytes (64 hex chars), no 0x prefix.
fn pad_u256(val: u128) -> String {
    format!("{:064x}", val)
}

/// ABI-encode and submit USDC.e approve(spender, amount).
/// Selector: 0x095ea7b3
/// To: USDC.e contract
pub async fn usdc_approve(usdc_addr: &str, spender: &str, amount: u128) -> Result<String> {
    let spender_padded = pad_address(spender);
    let amount_padded = pad_u256(amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_padded);
    let result = wallet_contract_call(usdc_addr, &calldata).await?;
    extract_tx_hash(&result)
}

/// ABI-encode and submit CTF setApprovalForAll(operator, true).
/// Selector: 0xa22cb465
/// To: CTF contract
pub async fn ctf_set_approval_for_all(ctf_addr: &str, operator: &str) -> Result<String> {
    let operator_padded = pad_address(operator);
    // approved = true = 1
    let approved_padded = pad_u256(1);
    let calldata = format!("0xa22cb465{}{}", operator_padded, approved_padded);
    let result = wallet_contract_call(ctf_addr, &calldata).await?;
    extract_tx_hash(&result)
}

/// Approve USDC.e allowance before a BUY order.
///
/// For neg_risk=false: approves CTF Exchange only.
/// For neg_risk=true: approves BOTH NEG_RISK_CTF_EXCHANGE and NEG_RISK_ADAPTER —
/// the CLOB checks both contracts in the settlement path for neg_risk markets.
/// Returns the tx hash of the last approval submitted.
pub async fn approve_usdc(neg_risk: bool, amount: u64) -> Result<String> {
    use crate::config::Contracts;
    let usdc = Contracts::USDC_E;
    if neg_risk {
        usdc_approve(usdc, Contracts::NEG_RISK_CTF_EXCHANGE, amount as u128).await?;
        usdc_approve(usdc, Contracts::NEG_RISK_ADAPTER, amount as u128).await
    } else {
        usdc_approve(usdc, Contracts::CTF_EXCHANGE, amount as u128).await
    }
}

/// Approve CTF tokens for sell orders.
///
/// For neg_risk=false: approves CTF_EXCHANGE only.
/// For neg_risk=true: approves BOTH NEG_RISK_CTF_EXCHANGE and NEG_RISK_ADAPTER —
/// the CLOB checks setApprovalForAll on both contracts for neg_risk markets (mirrors
/// the approve_usdc pattern for USDC.e allowance).
/// Returns the tx hash of the last approval submitted.
pub async fn approve_ctf(neg_risk: bool) -> Result<String> {
    use crate::config::Contracts;
    let ctf = Contracts::CTF;
    if neg_risk {
        ctf_set_approval_for_all(ctf, Contracts::NEG_RISK_CTF_EXCHANGE).await?;
        ctf_set_approval_for_all(ctf, Contracts::NEG_RISK_ADAPTER).await
    } else {
        ctf_set_approval_for_all(ctf, Contracts::CTF_EXCHANGE).await
    }
}

/// Check if the CTF contract has setApprovalForAll set for owner → operator.
/// Makes a direct eth_call to the Polygon RPC to read isApprovedForAll(owner, operator).
/// Returns true if already approved, false if not approved or on RPC failure (fail-open:
/// falling back to approving is always safe — setApprovalForAll is idempotent).
pub async fn is_ctf_approved_for_all(owner: &str, operator: &str) -> bool {
    use crate::config::{Contracts, Urls};
    // isApprovedForAll(address,address) selector = 0xe985e9c5
    let data = format!("0xe985e9c5{}{}", pad_address(owner), pad_address(operator));
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": Contracts::CTF, "data": data }, "latest"],
        "id": 1
    });
    let client = reqwest::Client::new();
    let resp = match client.post(Urls::POLYGON_RPC).json(&body).send().await {
        Ok(r) => r,
        Err(_) => return false,
    };
    let v: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return false,
    };
    // ABI-encoded bool: 32 bytes. Approved = 0x0000...0001, Not approved = 0x0000...0000
    let hex = v["result"].as_str().unwrap_or("0x").trim_start_matches("0x");
    !hex.is_empty() && hex.trim_start_matches('0') == "1"
}

