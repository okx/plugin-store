use std::process::Command;
use serde_json::Value;
use sha3::{Digest, Keccak256};

/// Execute an EVM contract call via onchainos wallet contract-call.
/// chain_id: the EVM chain (e.g. 42161 for Arbitrum).
/// to: contract address.
/// calldata: hex-encoded calldata (0x-prefixed).
/// value_wei: optional ETH value to send.
/// confirm: if false, preview only; if true, broadcast.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    calldata: &str,
    value_wei: Option<u128>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "chain": chain_id,
            "to": to,
            "data": calldata,
            "note": "Dry run — not submitted"
        }));
    }

    let mut args = vec![
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_id.to_string(),
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        calldata.to_string(),
    ];
    if let Some(v) = value_wei {
        args.push("--amt".to_string());
        args.push(v.to_string());
    }
    // Note: --force is intentionally omitted — onchainos handles its own confirmation.
    // The plugin's --confirm flag already gates whether this call is made at all.

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        // onchainos returns error as JSON to stdout; stderr is usually empty
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stdout.trim().is_empty() { stderr.to_string() } else { stdout.to_string() };
        anyhow::bail!("onchainos wallet contract-call failed: {}", detail.trim());
    }
    let result: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|_| serde_json::json!({"raw": stdout.to_string()}));
    Ok(result)
}

/// Resolve the wallet address from the onchainos CLI.
/// For Hyperliquid, we query EVM addresses (HyperEVM chain_id=999).
/// Falls back to the first EVM address if chain_id 999 is not listed.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    let chain_id_str = chain_id.to_string();
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        // Try to find exact chain match
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_id_str) {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
        // Fallback: use first EVM address (Hyperliquid uses same key as Ethereum)
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not resolve wallet address for chain {}", chain_id)
}

/// Sign an EIP-712 typed data message via onchainos and return the hex signature.
/// Returns 65-byte hex signature (0x-prefixed, r+s+v).
pub fn onchainos_sign_eip712(typed_data: &serde_json::Value, wallet: &str) -> anyhow::Result<String> {
    let message_str = serde_json::to_string(typed_data)?;
    let output = Command::new("onchainos")
        .args([
            "wallet",
            "sign-message",
            "--type",
            "eip712",
            "--message",
            &message_str,
            "--chain",
            "42161",
            "--from",
            wallet,
        ])
        .output()?;

    // onchainos outputs JSON to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stdout.trim().is_empty() { stderr.to_string() } else { stdout.to_string() };
        anyhow::bail!("onchainos sign-message failed: {}", detail);
    }

    let result: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| anyhow::anyhow!("Failed to parse sign-message output: {} — raw: {}", e, stdout))?;

    // onchainos returns {"ok":true,"data":{"signature":"0x..."}} or {"signature":"0x..."}
    let sig = result["data"]["signature"]
        .as_str()
        .or_else(|| result["signature"].as_str())
        .ok_or_else(|| anyhow::anyhow!("No signature in sign-message response: {}", stdout))?;

    Ok(sig.to_string())
}

/// Sign a Hyperliquid L1 action via onchainos and submit it.
///
/// Uses `onchainos wallet sign-message --type eip712` with the Hyperliquid
/// EIP-712 typed data structure for L1 action signing.
///
/// wallet_chain_id: the EVM chain ID used to resolve the wallet (passed to --chain so
///                  onchainos selects the same key it resolved the wallet with).
/// dry_run: if true, returns the unsigned preview payload without submitting.
/// confirm: if false (no --confirm flag), returns the preview payload for review.
///          if true, proceeds to sign and submit.
pub fn onchainos_hl_sign(
    action: &Value,
    nonce: u64,
    wallet: &str,
    wallet_chain_id: u64,
    confirm: bool,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "action": action,
            "nonce": nonce,
            "note": "Dry run - not signed or submitted"
        }));
    }

    if !confirm {
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "action": action,
            "nonce": nonce,
            "note": "Preview only - add --confirm to sign and submit"
        }));
    }

    // Build Hyperliquid EIP-712 typed data for L1 action signing.
    // Hyperliquid L1 uses a phantom agent pattern: sign an Agent struct
    // that commits to the connection ID = keccak256(msgpack(action) + nonce_be + 0x00).
    // This matches the official HL Python SDK action_hash() function.
    let action_bytes = rmp_serde::to_vec(action)
        .map_err(|e| anyhow::anyhow!("msgpack encode failed: {}", e))?;
    let nonce_be = nonce.to_be_bytes();
    let mut hash_input = Vec::with_capacity(action_bytes.len() + 9);
    hash_input.extend_from_slice(&action_bytes);
    hash_input.extend_from_slice(&nonce_be);
    hash_input.push(0x00u8);
    let mut hasher = Keccak256::new();
    hasher.update(&hash_input);
    let digest = hasher.finalize();
    let connection_id = format!("0x{}", hex::encode(digest));


    // EIP-712 typed data — field order matches HL Python SDK exactly:
    // domain: chainId, name, verifyingContract, version
    // types: Agent first, EIP712Domain second (required by onchainos for correct hash)
    let eip712_message = serde_json::json!({
        "domain": {
            "chainId": 1337,
            "name": "Exchange",
            "verifyingContract": "0x0000000000000000000000000000000000000000",
            "version": "1"
        },
        "types": {
            "Agent": [
                { "name": "source",       "type": "string"  },
                { "name": "connectionId", "type": "bytes32" }
            ],
            "EIP712Domain": [
                { "name": "name",              "type": "string"  },
                { "name": "version",           "type": "string"  },
                { "name": "chainId",           "type": "uint256" },
                { "name": "verifyingContract", "type": "address" }
            ]
        },
        "primaryType": "Agent",
        "message": {
            "source":       "a",
            "connectionId": connection_id
        }
    });

    let eip712_str = serde_json::to_string(&eip712_message)?;

    let wallet_chain_str = wallet_chain_id.to_string();
    let output = Command::new("onchainos")
        .args([
            "wallet",
            "sign-message",
            "--type",
            "eip712",
            "--message",
            &eip712_str,
            "--chain",
            &wallet_chain_str,
            "--from",
            wallet,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "onchainos sign-message failed: {}. \
             Ensure onchainos CLI is configured with a valid wallet. \
             Use --dry-run to preview the unsigned payload.",
            stderr
        );
    }

    // Parse the signature from onchainos output
    let sign_result: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse sign-message output: {}", e))?;

    let signature = sign_result["data"]["signature"]
        .as_str()
        .or_else(|| sign_result["signature"].as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No signature in sign-message response: {}",
                serde_json::to_string(&sign_result).unwrap_or_default()
            )
        })?;

    // Parse r, s, v from the 65-byte hex signature (no external crate needed)
    let sig_hex = signature.trim_start_matches("0x");
    if sig_hex.len() != 130 {
        anyhow::bail!(
            "Expected 130-char hex signature (65 bytes), got {} chars",
            sig_hex.len()
        );
    }
    let r = format!("0x{}", &sig_hex[0..64]);
    let s = format!("0x{}", &sig_hex[64..128]);
    let v: u64 = u64::from_str_radix(&sig_hex[128..130], 16)
        .map_err(|e| anyhow::anyhow!("Failed to parse v byte: {}", e))?;

    // Build the final Hyperliquid exchange request body
    Ok(serde_json::json!({
        "action":       action,
        "nonce":        nonce,
        "signature":    { "r": r, "s": s, "v": v },
        "vaultAddress": null
    }))
}
