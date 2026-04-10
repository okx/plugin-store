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

/// Approve a bounded USDC.e amount to CTF Exchange. Used before BUY orders.
/// Approves exactly `amount` (the order's maker_amount raw units) to avoid granting
/// unlimited spending power to the exchange contract.
pub async fn approve_usdc(neg_risk: bool, amount: u64) -> Result<String> {
    use crate::config::Contracts;
    let usdc = Contracts::USDC_E;
    let exchange = Contracts::exchange_for(neg_risk);
    usdc_approve(usdc, exchange, amount as u128).await
}

/// Approve CTF tokens for CTF Exchange. Used before SELL orders.
pub async fn approve_ctf(neg_risk: bool) -> Result<String> {
    use crate::config::Contracts;
    let ctf = Contracts::CTF;
    let exchange = Contracts::exchange_for(neg_risk);
    ctf_set_approval_for_all(ctf, exchange).await
}

