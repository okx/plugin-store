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

/// ABI-encode and submit CTF redeemPositions(collateralToken, parentCollectionId, conditionId, indexSets).
///
/// Redeems all outcome positions for the given conditionId. indexSets [1, 2] covers both
/// YES (bit 0) and NO (bit 1) outcomes — the CTF contract only pays out for winning tokens
/// and silently no-ops for losing ones, so passing both is safe.
/// For neg_risk (multi-outcome) markets use the NEG_RISK_ADAPTER path (not implemented here).
pub async fn ctf_redeem_positions(condition_id: &str) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    // Compute the 4-byte function selector: keccak256("redeemPositions(address,bytes32,bytes32,uint256[])")
    let selector = Keccak256::digest(b"redeemPositions(address,bytes32,bytes32,uint256[])");
    let selector_hex = hex::encode(&selector[..4]);

    // ABI-encode the four parameters.
    // Slots 0-2 are static (address and bytes32); slot 3 is the offset to the dynamic uint256[] array.
    let collateral  = pad_address(Contracts::USDC_E);         // address padded to 32 bytes
    let parent_id   = format!("{:064x}", 0u128);               // bytes32(0) — null parent collection
    let cond_id_hex = condition_id.trim_start_matches("0x");
    let cond_id_pad = format!("{:0>64}", cond_id_hex);         // conditionId as bytes32
    let array_offset = pad_u256(4 * 32);                       // 4 static slots → offset = 128

    // Dynamic array: length=2, [1, 2] (YES indexSet=1, NO indexSet=2)
    let array_len  = pad_u256(2);
    let index_yes  = pad_u256(1);  // outcome 0, indexSet bit 0
    let index_no   = pad_u256(2);  // outcome 1, indexSet bit 1

    let calldata = format!(
        "0x{}{}{}{}{}{}{}{}",
        selector_hex, collateral, parent_id, cond_id_pad,
        array_offset, array_len, index_yes, index_no
    );

    let result = wallet_contract_call(Contracts::CTF, &calldata).await?;
    extract_tx_hash(&result)
}

/// Check if the CTF contract has setApprovalForAll set for owner → operator.
/// Makes a direct eth_call to the Polygon RPC to read isApprovedForAll(owner, operator).
///
/// Returns Ok(true) if approved, Ok(false) if not approved, Err if the RPC call fails.
/// Callers should treat Err as "unknown — approve to be safe" (setApprovalForAll is idempotent).
pub async fn is_ctf_approved_for_all(owner: &str, operator: &str) -> Result<bool> {
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
    let resp = client
        .post(Urls::POLYGON_RPC)
        .json(&body)
        .send()
        .await
        .context("Polygon RPC request failed")?;
    let v: serde_json::Value = resp.json().await
        .context("parsing Polygon RPC response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("Polygon RPC error: {}", err);
    }
    // ABI-encoded bool: 32 bytes. Approved = 0x0000...0001, Not approved = 0x0000...0000
    let hex = v["result"].as_str().unwrap_or("0x").trim_start_matches("0x");
    Ok(!hex.is_empty() && hex.trim_start_matches('0') == "1")
}

