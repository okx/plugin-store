//! EIP-712 signing for Hyperliquid exchange endpoint.
//!
//! Hyperliquid uses a "phantom agent" signing scheme:
//! 1. Msgpack-encode the action, append nonce (8 bytes BE) and vault flag
//! 2. Keccak-256 hash the result → `connectionId`
//! 3. Build an EIP-712 `Agent { source, connectionId }` struct
//!    - `source` = "a" (mainnet) or "b" (testnet)
//!    - Domain: name="Exchange", version="1", chainId=1337, verifyingContract=0x0
//! 4. Sign the EIP-712 typed-data hash and return `{ r, s, v }`

use alloy::signers::SignerSync;
use alloy_primitives::B256;
use alloy_signer_local::PrivateKeySigner;
use anyhow::{Context, Result};
use serde_json::Value;
use tiny_keccak::{Hasher, Keccak};

const DOMAIN_NAME: &str = "Exchange";
const DOMAIN_VERSION: &str = "1";
const DOMAIN_CHAIN_ID: u64 = 1337;

fn keccak(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut out = [0u8; 32];
    hasher.update(data);
    hasher.finalize(&mut out);
    out
}

fn domain_separator() -> [u8; 32] {
    let type_hash = keccak(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let name_hash = keccak(DOMAIN_NAME.as_bytes());
    let version_hash = keccak(DOMAIN_VERSION.as_bytes());

    let mut chain_id_bytes = [0u8; 32];
    chain_id_bytes[24..].copy_from_slice(&DOMAIN_CHAIN_ID.to_be_bytes());

    let verifying_contract = [0u8; 32];

    let mut encoded = Vec::with_capacity(160);
    encoded.extend_from_slice(&type_hash);
    encoded.extend_from_slice(&name_hash);
    encoded.extend_from_slice(&version_hash);
    encoded.extend_from_slice(&chain_id_bytes);
    encoded.extend_from_slice(&verifying_contract);

    keccak(&encoded)
}

fn agent_struct_hash(source: &str, connection_id: &[u8; 32]) -> [u8; 32] {
    let type_hash = keccak(b"Agent(string source,bytes32 connectionId)");
    let source_hash = keccak(source.as_bytes());

    let mut encoded = Vec::with_capacity(96);
    encoded.extend_from_slice(&type_hash);
    encoded.extend_from_slice(&source_hash);
    encoded.extend_from_slice(connection_id);

    keccak(&encoded)
}

fn action_hash(action: &Value, nonce: u64, vault_address: Option<&str>) -> Result<[u8; 32]> {
    let packed = rmp_serde::to_vec_named(action).context("msgpack encode failed")?;

    let mut data = packed;
    data.extend_from_slice(&nonce.to_be_bytes());

    match vault_address {
        None => {
            data.push(0x00);
        }
        Some(addr) => {
            data.push(0x01);
            let addr_hex = addr.strip_prefix("0x").unwrap_or(addr);
            let addr_bytes = hex::decode(addr_hex).context("invalid vault address")?;
            data.extend_from_slice(&addr_bytes);
        }
    }

    Ok(keccak(&data))
}

fn eip712_hash(struct_hash: &[u8; 32]) -> [u8; 32] {
    let domain_sep = domain_separator();
    let mut msg = Vec::with_capacity(66);
    msg.push(0x19);
    msg.push(0x01);
    msg.extend_from_slice(&domain_sep);
    msg.extend_from_slice(struct_hash);
    keccak(&msg)
}

pub fn is_mainnet(base_url: &str) -> bool {
    !base_url.contains("testnet")
}

pub fn sign_action(
    signer: &PrivateKeySigner,
    action: &Value,
    nonce: u64,
    vault_address: Option<&str>,
    mainnet: bool,
) -> Result<Value> {
    let conn_id = action_hash(action, nonce, vault_address)?;
    let source = if mainnet { "a" } else { "b" };
    let struct_hash = agent_struct_hash(source, &conn_id);
    let digest = eip712_hash(&struct_hash);

    let sig = signer
        .sign_hash_sync(&B256::from(digest))
        .context("EIP-712 signing failed")?;

    let v = sig.v() as u8 + 27;
    let r_bytes: [u8; 32] = sig.r().to_be_bytes();
    let s_bytes: [u8; 32] = sig.s().to_be_bytes();

    Ok(serde_json::json!({
        "r": format!("0x{}", hex::encode(r_bytes)),
        "s": format!("0x{}", hex::encode(s_bytes)),
        "v": v,
    }))
}
