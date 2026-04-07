// onchainos CLI wrapper for Yearn Finance plugin
// All on-chain writes go through onchainos wallet contract-call

use anyhow::Result;
use serde_json::Value;
use std::process::Command;

/// Resolve the EVM wallet address for the given chain via onchainos
pub fn resolve_wallet(chain_id: u64) -> Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse wallet addresses: {}: {}", e, stdout))?;

    let chain_index = chain_id.to_string();
    // Look through data.evm[] for matching chainIndex
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_index)
                || entry["chainIndex"].as_u64() == Some(chain_id)
            {
                if let Some(addr) = entry["address"].as_str() {
                    if !addr.is_empty() {
                        return Ok(addr.to_string());
                    }
                }
            }
        }
        // fallback: return first EVM address
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                if !addr.is_empty() {
                    return Ok(addr.to_string());
                }
            }
        }
    }
    anyhow::bail!(
        "Could not resolve wallet address for chain {}. Make sure onchainos is logged in.",
        chain_id
    )
}

/// Call onchainos wallet contract-call
/// dry_run=true returns a simulated response without calling onchainos
/// (onchainos wallet contract-call does NOT accept --dry-run)
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    dry_run: bool,
) -> Result<Value> {
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
    let output = Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            &chain_str,
            "--to",
            to,
            "--input-data",
            input_data,
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("onchainos returned empty output. stderr: {}", stderr);
    }
    serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos response: {}: {}", e, stdout))
}

/// Extract txHash from onchainos response
/// Checks: data.txHash (primary for EVM)
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .unwrap_or("pending")
        .to_string()
}

/// Encode ERC-20 approve(address,uint256) calldata
/// selector: 0x095ea7b3
pub fn encode_approve(spender: &str, amount: u128) -> String {
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    format!("0x095ea7b3{}{}", spender_padded, amount_hex)
}

/// Encode ERC-4626 deposit(uint256 assets, address receiver) calldata
/// selector: 0x6e553f65
pub fn encode_deposit(assets: u128, receiver: &str) -> String {
    let receiver_clean = receiver.trim_start_matches("0x");
    let assets_hex = format!("{:064x}", assets);
    let receiver_padded = format!("{:0>64}", receiver_clean);
    format!("0x6e553f65{}{}", assets_hex, receiver_padded)
}

/// Encode ERC-4626 redeem(uint256 shares, address receiver, address owner) calldata
/// selector: 0xba087652
pub fn encode_redeem(shares: u128, receiver: &str, owner: &str) -> String {
    let receiver_clean = receiver.trim_start_matches("0x");
    let owner_clean = owner.trim_start_matches("0x");
    let shares_hex = format!("{:064x}", shares);
    let receiver_padded = format!("{:0>64}", receiver_clean);
    let owner_padded = format!("{:0>64}", owner_clean);
    format!("0xba087652{}{}{}", shares_hex, receiver_padded, owner_padded)
}

/// Encode balanceOf(address) call
/// selector: 0x70a08231
pub fn encode_balance_of(owner: &str) -> String {
    let owner_clean = owner.trim_start_matches("0x");
    let owner_padded = format!("{:0>64}", owner_clean);
    format!("0x70a08231{}", owner_padded)
}
