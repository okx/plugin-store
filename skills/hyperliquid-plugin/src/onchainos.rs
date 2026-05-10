use std::process::Command;
use serde_json::Value;
use sha3::{Digest, Keccak256};

/// `--biz-type` / `--strategy`: attribution to the onchainos backend.
/// Source-of-truth for the plugin name is Cargo.toml's `[package]` `name`.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

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
        "--biz-type".to_string(),
        BIZ_TYPE.to_string(),
        "--strategy".to_string(),
        STRATEGY.to_string(),
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
/// Falls back to the first EVM address if chain_id is not listed.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let (addr, _) = resolve_wallet_with_chain(chain_id)?;
    Ok(addr)
}

/// Like resolve_wallet but also returns the chain index that owns the resolved address.
/// Used when the signing chain must match the resolved wallet (e.g. user-signed actions).
pub fn resolve_wallet_with_chain(chain_id: u64) -> anyhow::Result<(String, u64)> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    let chain_id_str = chain_id.to_string();
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_id_str) {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok((addr.to_string(), chain_id));
                }
            }
        }
        // Fallback: use first EVM address + its chain index
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                let chain = first["chainIndex"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(1);
                return Ok((addr.to_string(), chain));
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

/// Sign a Hyperliquid withdraw3 action via onchainos (user-signed EIP-712).
/// domain: HyperliquidSignTransaction, chainId 421614 (0x66eee).
pub fn onchainos_hl_sign_withdraw(
    destination: &str,
    amount: &str,
    nonce: u64,
    wallet: &str,
    wallet_chain_id: u64,
) -> anyhow::Result<Value> {
    let eip712_message = serde_json::json!({
        "domain": {
            "chainId": 421614,  // 0x66eee — matches action.signatureChainId
            "name": "HyperliquidSignTransaction",
            "verifyingContract": "0x0000000000000000000000000000000000000000",
            "version": "1"
        },
        "types": {
            "HyperliquidTransaction:Withdraw": [
                { "name": "hyperliquidChain", "type": "string"  },
                { "name": "destination",      "type": "string"  },
                { "name": "amount",           "type": "string"  },
                { "name": "time",             "type": "uint64"  }
            ],
            "EIP712Domain": [
                { "name": "name",              "type": "string"  },
                { "name": "version",           "type": "string"  },
                { "name": "chainId",           "type": "uint256" },
                { "name": "verifyingContract", "type": "address" }
            ]
        },
        "primaryType": "HyperliquidTransaction:Withdraw",
        "message": {
            "hyperliquidChain": "Mainnet",
            "destination": destination,
            "amount": amount,
            "time": nonce
        }
    });

    let eip712_str = serde_json::to_string(&eip712_message)?;
    let wallet_chain_str = wallet_chain_id.to_string();

    let output = Command::new("onchainos")
        .args([
            "wallet", "sign-message",
            "--type", "eip712",
            "--message", &eip712_str,
            "--chain", &wallet_chain_str,
            "--from", wallet,
        ])
        .output()?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stdout.trim().is_empty() { stderr.to_string() } else { stdout.to_string() };
        anyhow::bail!("onchainos sign-message failed: {}", detail.trim());
    }

    let sign_result: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse sign-message output: {}", e))?;

    let signature = sign_result["data"]["signature"]
        .as_str()
        .or_else(|| sign_result["signature"].as_str())
        .ok_or_else(|| anyhow::anyhow!("No signature in sign-message response: {}", serde_json::to_string(&sign_result).unwrap_or_default()))?;

    let sig_hex = signature.trim_start_matches("0x");
    if sig_hex.len() != 130 {
        anyhow::bail!("Expected 130-char hex signature, got {} chars", sig_hex.len());
    }
    let r = format!("0x{}", &sig_hex[0..64]);
    let s = format!("0x{}", &sig_hex[64..128]);
    let v: u64 = u64::from_str_radix(&sig_hex[128..130], 16)
        .map_err(|e| anyhow::anyhow!("Failed to parse v byte: {}", e))?;

    let action = serde_json::json!({
        "type": "withdraw3",
        "hyperliquidChain": "Mainnet",
        "signatureChainId": "0x66eee",
        "destination": destination,
        "amount": amount,
        "time": nonce
    });

    Ok(serde_json::json!({
        "action":       action,
        "nonce":        nonce,
        "signature":    { "r": r, "s": s, "v": v },
        "vaultAddress": null
    }))
}

/// Sign a Hyperliquid usdClassTransfer action (perp ↔ spot) via onchainos (user-signed EIP-712).
/// domain: HyperliquidSignTransaction, chainId 421614 (0x66eee).
pub fn onchainos_hl_sign_usd_class_transfer(
    action: &Value,
    nonce: u64,
    wallet: &str,
    wallet_chain_id: u64,
    confirm: bool,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true, "dry_run": true,
            "action": action, "nonce": nonce,
            "note": "Dry run - not signed or submitted"
        }));
    }
    if !confirm {
        return Ok(serde_json::json!({
            "ok": true, "preview": true,
            "action": action, "nonce": nonce,
            "note": "Preview only - add --confirm to sign and submit"
        }));
    }

    let amount = action["amount"].as_str()
        .ok_or_else(|| anyhow::anyhow!("action.amount must be a string"))?;
    let to_perp = action["toPerp"].as_bool()
        .ok_or_else(|| anyhow::anyhow!("action.toPerp must be a bool"))?;

    let eip712_message = serde_json::json!({
        "domain": {
            "chainId": 421614,  // 0x66eee — matches action.signatureChainId
            "name": "HyperliquidSignTransaction",
            "verifyingContract": "0x0000000000000000000000000000000000000000",
            "version": "1"
        },
        "types": {
            "HyperliquidTransaction:UsdClassTransfer": [
                { "name": "hyperliquidChain", "type": "string"  },
                { "name": "amount",           "type": "string"  },
                { "name": "toPerp",           "type": "bool"    },
                { "name": "nonce",            "type": "uint64"  }
            ],
            "EIP712Domain": [
                { "name": "name",              "type": "string"  },
                { "name": "version",           "type": "string"  },
                { "name": "chainId",           "type": "uint256" },
                { "name": "verifyingContract", "type": "address" }
            ]
        },
        "primaryType": "HyperliquidTransaction:UsdClassTransfer",
        "message": {
            "hyperliquidChain": "Mainnet",
            "amount": amount,
            "toPerp": to_perp,
            "nonce": nonce
        }
    });

    let eip712_str = serde_json::to_string(&eip712_message)?;
    let wallet_chain_str = wallet_chain_id.to_string();

    let output = Command::new("onchainos")
        .args([
            "wallet", "sign-message",
            "--type", "eip712",
            "--message", &eip712_str,
            "--chain", &wallet_chain_str,
            "--from", wallet,
        ])
        .output()?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stdout.trim().is_empty() { stderr.to_string() } else { stdout.to_string() };
        anyhow::bail!("onchainos sign-message failed: {}", detail.trim());
    }

    let sign_result: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse sign-message output: {}", e))?;

    let signature = sign_result["data"]["signature"]
        .as_str()
        .or_else(|| sign_result["signature"].as_str())
        .ok_or_else(|| anyhow::anyhow!("No signature in sign-message response: {}", serde_json::to_string(&sign_result).unwrap_or_default()))?;

    let sig_hex = signature.trim_start_matches("0x");
    if sig_hex.len() != 130 {
        anyhow::bail!("Expected 130-char hex signature, got {} chars", sig_hex.len());
    }
    let r = format!("0x{}", &sig_hex[0..64]);
    let s = format!("0x{}", &sig_hex[64..128]);
    let v: u64 = u64::from_str_radix(&sig_hex[128..130], 16)
        .map_err(|e| anyhow::anyhow!("Failed to parse v byte: {}", e))?;

    Ok(serde_json::json!({
        "action":       action,
        "nonce":        nonce,
        "signature":    { "r": r, "s": s, "v": v },
        "vaultAddress": null
    }))
}

/// HIP-3: Sign a Hyperliquid `sendAsset` action via onchainos (user-signed EIP-712).
/// Used to move USDC between perp DEXs (default <-> builder DEX like xyz/flx/etc).
/// Each builder DEX has a SEPARATE clearinghouse balance — the sender must explicitly
/// route USDC to the destination DEX before they can trade RWA markets there.
///
/// EIP-712 schema (8 fields, "HyperliquidTransaction:SendAsset"):
///   hyperliquidChain (string), destination (address as string), sourceDex (string),
///   destinationDex (string), token (string, e.g. "USDC:0x..."), amount (string),
///   fromSubAccount (string, "" for none), nonce (uint64)
///
/// Signing-spike-verified 2026-04-30 (offline ecrecover round-trip OK).
pub fn onchainos_hl_sign_send_asset(
    action: &Value,
    nonce: u64,
    wallet: &str,
    wallet_chain_id: u64,
    confirm: bool,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true, "dry_run": true,
            "action": action, "nonce": nonce,
            "note": "Dry run - sendAsset not signed or submitted"
        }));
    }
    if !confirm {
        return Ok(serde_json::json!({
            "ok": true, "preview": true,
            "action": action, "nonce": nonce,
            "note": "Preview only - add --confirm to sign and submit"
        }));
    }

    let destination = action["destination"].as_str()
        .ok_or_else(|| anyhow::anyhow!("action.destination must be a string"))?;
    let source_dex = action["sourceDex"].as_str()
        .ok_or_else(|| anyhow::anyhow!("action.sourceDex must be a string"))?;
    let destination_dex = action["destinationDex"].as_str()
        .ok_or_else(|| anyhow::anyhow!("action.destinationDex must be a string"))?;
    let token = action["token"].as_str()
        .ok_or_else(|| anyhow::anyhow!("action.token must be a string (e.g. 'USDC:0x...')"))?;
    let amount = action["amount"].as_str()
        .ok_or_else(|| anyhow::anyhow!("action.amount must be a string"))?;
    let from_sub_account = action["fromSubAccount"].as_str().unwrap_or("");

    let eip712_message = serde_json::json!({
        "domain": {
            "chainId": 421614,  // 0x66eee
            "name": "HyperliquidSignTransaction",
            "verifyingContract": "0x0000000000000000000000000000000000000000",
            "version": "1"
        },
        "types": {
            "HyperliquidTransaction:SendAsset": [
                { "name": "hyperliquidChain", "type": "string" },
                { "name": "destination",      "type": "string" },
                { "name": "sourceDex",        "type": "string" },
                { "name": "destinationDex",   "type": "string" },
                { "name": "token",            "type": "string" },
                { "name": "amount",           "type": "string" },
                { "name": "fromSubAccount",   "type": "string" },
                { "name": "nonce",            "type": "uint64" }
            ],
            "EIP712Domain": [
                { "name": "name",              "type": "string"  },
                { "name": "version",           "type": "string"  },
                { "name": "chainId",           "type": "uint256" },
                { "name": "verifyingContract", "type": "address" }
            ]
        },
        "primaryType": "HyperliquidTransaction:SendAsset",
        "message": {
            "hyperliquidChain": "Mainnet",
            "destination":      destination,
            "sourceDex":        source_dex,
            "destinationDex":   destination_dex,
            "token":            token,
            "amount":           amount,
            "fromSubAccount":   from_sub_account,
            "nonce":            nonce
        }
    });

    let eip712_str = serde_json::to_string(&eip712_message)?;
    let wallet_chain_str = wallet_chain_id.to_string();

    let output = Command::new("onchainos")
        .args([
            "wallet", "sign-message",
            "--type", "eip712",
            "--message", &eip712_str,
            "--chain", &wallet_chain_str,
            "--from", wallet,
        ])
        .output()?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stdout.trim().is_empty() { stderr.to_string() } else { stdout.to_string() };
        anyhow::bail!("onchainos sign-message failed (sendAsset): {}", detail.trim());
    }

    let sign_result: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse sign-message output: {}", e))?;

    let signature = sign_result["data"]["signature"]
        .as_str()
        .or_else(|| sign_result["signature"].as_str())
        .ok_or_else(|| anyhow::anyhow!("No signature in sign-message response: {}", serde_json::to_string(&sign_result).unwrap_or_default()))?;

    let sig_hex = signature.trim_start_matches("0x");
    if sig_hex.len() != 130 {
        anyhow::bail!("Expected 130-char hex signature, got {} chars", sig_hex.len());
    }
    let r = format!("0x{}", &sig_hex[0..64]);
    let s = format!("0x{}", &sig_hex[64..128]);
    let v: u64 = u64::from_str_radix(&sig_hex[128..130], 16)
        .map_err(|e| anyhow::anyhow!("Failed to parse v byte: {}", e))?;

    Ok(serde_json::json!({
        "action":       action,
        "nonce":        nonce,
        "signature":    { "r": r, "s": s, "v": v },
        "vaultAddress": null
    }))
}

/// Report plugin-level order metadata to the OKX backend for strategy attribution.
///
/// Shells out to `onchainos wallet report-plugin-info --plugin-parameter <json> --chain 42161`.
/// Non-fatal at the call site: the trade has already been submitted by the time this runs,
/// so callers should log and continue on error rather than propagate.
pub fn report_plugin_info(payload: &Value) -> anyhow::Result<()> {
    let payload_str = serde_json::to_string(payload)
        .map_err(|e| anyhow::anyhow!("serializing report-plugin-info payload: {}", e))?;
    let output = Command::new("onchainos")
        .args([
            "wallet", "report-plugin-info",
            "--plugin-parameter", &payload_str,
            "--chain", "42161",
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to spawn onchainos wallet report-plugin-info: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "onchainos report-plugin-info failed ({}): {}",
            output.status,
            stderr.trim()
        );
    }
    Ok(())
}
