/// EIP-712 signing for Polymarket CTF Exchange and ClobAuth.
///
/// Uses a local k256 signing key (~/.config/polymarket/signing_key.hex) for all
/// EIP-712 operations. The key is auto-generated on first run, similar to the
/// Hyperliquid plugin pattern. The onchainos wallet remains the fund holder; the
/// local key is registered as an operator via CTFExchange.setOperatorApproval().
use anyhow::{Context, Result};
use k256::ecdsa::{RecoveryId, Signature, SigningKey};
use num_bigint::BigUint;
use tiny_keccak::{Hasher, Keccak};

use crate::config::{signing_key_address, Contracts};

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(data);
    hasher.finalize(&mut output);
    output
}

// ─── Domain separators ────────────────────────────────────────────────────────

/// ClobAuth domain separator (Polygon chain 137, no verifyingContract)
fn clob_auth_domain_sep() -> [u8; 32] {
    // EIP712Domain(string name,string version,uint256 chainId)
    // ABI-encode: [typeHash(32), keccak(name)(32), keccak(version)(32), chainId(32)] = 128 bytes
    let type_hash = keccak256(b"EIP712Domain(string name,string version,uint256 chainId)");
    let name_hash = keccak256(b"ClobAuthDomain");
    let version_hash = keccak256(b"1");

    let mut buf = [0u8; 128]; // 4 × 32-byte slots
    buf[..32].copy_from_slice(&type_hash);
    buf[32..64].copy_from_slice(&name_hash);
    buf[64..96].copy_from_slice(&version_hash);
    // chainId = 137 = 0x89, right-aligned in the 4th 32-byte slot
    buf[127] = 0x89; // 137
    keccak256(&buf)
}

/// CTF Exchange domain separator (Polygon chain 137)
fn ctf_exchange_domain_sep(verifying_contract: &str) -> Result<[u8; 32]> {
    // EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)
    // ABI-encode: [typeHash(32), keccak(name)(32), keccak(version)(32), chainId(32), address(32)] = 160 bytes
    let type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    // Polymarket CTF Exchange on-chain domain name (verified via contract domainSeparator())
    let name_hash = keccak256(b"Polymarket CTF Exchange");
    let version_hash = keccak256(b"1");

    let contract_bytes = hex::decode(
        verifying_contract
            .strip_prefix("0x")
            .unwrap_or(verifying_contract),
    )
    .context("decoding verifyingContract address")?;
    anyhow::ensure!(contract_bytes.len() == 20, "verifyingContract must be 20 bytes");

    let mut buf = [0u8; 160]; // 5 × 32-byte slots
    buf[..32].copy_from_slice(&type_hash);
    buf[32..64].copy_from_slice(&name_hash);
    buf[64..96].copy_from_slice(&version_hash);
    // chainId = 137 = 0x89, right-aligned in 4th slot [96..128]
    buf[127] = 0x89;
    // verifyingContract, right-aligned in 5th slot [128..160]
    buf[140..160].copy_from_slice(&contract_bytes);
    Ok(keccak256(&buf))
}

// ─── ClobAuth signing ─────────────────────────────────────────────────────────

/// Sign a Polymarket ClobAuth EIP-712 message with the local signing key.
/// Returns (signer_address, signature_hex, timestamp, nonce).
pub fn sign_clob_auth(
    key: &SigningKey,
    nonce: u64,
) -> Result<(String, String, u64, u64)> {
    let signer_addr = signing_key_address(key);
    let timestamp = chrono::Utc::now().timestamp() as u64;

    // ClobAuth struct type hash
    // ClobAuth(address address,string timestamp,uint256 nonce,string message)
    let type_hash = keccak256(
        b"ClobAuth(address address,string timestamp,uint256 nonce,string message)",
    );

    // ABI-encode the struct fields
    // address: 32-byte slot (12 zero padding bytes + 20 address bytes)
    let addr_bytes = hex::decode(
        signer_addr
            .strip_prefix("0x")
            .unwrap_or(&signer_addr),
    )
    .context("decoding signer address")?;

    let ts_str = timestamp.to_string();
    let message_str = "This message attests that I control the given wallet";

    let mut struct_buf = Vec::with_capacity(5 * 32);
    struct_buf.extend_from_slice(&type_hash);
    // address: left-pad to 32 bytes
    let mut addr_slot = [0u8; 32];
    addr_slot[12..].copy_from_slice(&addr_bytes);
    struct_buf.extend_from_slice(&addr_slot);
    // timestamp: keccak256(string)
    struct_buf.extend_from_slice(&keccak256(ts_str.as_bytes()));
    // nonce: uint256, big-endian in 32 bytes
    let mut nonce_slot = [0u8; 32];
    nonce_slot[24..].copy_from_slice(&nonce.to_be_bytes());
    struct_buf.extend_from_slice(&nonce_slot);
    // message: keccak256(string)
    struct_buf.extend_from_slice(&keccak256(message_str.as_bytes()));

    let struct_hash = keccak256(&struct_buf);
    let domain_sep = clob_auth_domain_sep();

    // Final EIP-712 digest: keccak256("\x19\x01" || domainSep || structHash)
    let mut digest_buf = [0u8; 66];
    digest_buf[0] = 0x19;
    digest_buf[1] = 0x01;
    digest_buf[2..34].copy_from_slice(&domain_sep);
    digest_buf[34..66].copy_from_slice(&struct_hash);
    let digest = keccak256(&digest_buf);

    let sig_hex = sign_digest(key, &digest)?;
    Ok((signer_addr, sig_hex, timestamp, nonce))
}

// ─── Order signing ────────────────────────────────────────────────────────────

/// Parameters for a Polymarket limit order.
pub struct OrderParams {
    pub salt: u64,
    pub maker: String,      // Polymarket proxy wallet address (holds USDC.e / outcome tokens)
    pub signer: String,     // onchainos wallet address (approved operator of proxy wallet)
    pub taker: String,
    pub token_id: String,
    pub maker_amount: u64,
    pub taker_amount: u64,
    pub expiration: u64,
    pub nonce: u64,
    pub fee_rate_bps: u64,
    pub side: u8,           // 0=BUY, 1=SELL
    pub signature_type: u8, // 0=EOA
}

/// Sign a Polymarket order EIP-712 via `onchainos sign-message --type eip712`.
///
/// Builds a complete EIP-712 structured data JSON with EIP712Domain in `types`
/// (required for correct hash computation — per Hyperliquid root-cause finding).
pub async fn sign_order_via_onchainos(order: &OrderParams, neg_risk: bool) -> anyhow::Result<String> {
    use crate::config::Contracts;
    let verifying_contract = Contracts::exchange_for(neg_risk);

    let json = serde_json::to_string(&serde_json::json!({
        "types": {
            "EIP712Domain": [
                {"name": "name", "type": "string"},
                {"name": "version", "type": "string"},
                {"name": "chainId", "type": "uint256"},
                {"name": "verifyingContract", "type": "address"}
            ],
            "Order": [
                {"name": "salt", "type": "uint256"},
                {"name": "maker", "type": "address"},
                {"name": "signer", "type": "address"},
                {"name": "taker", "type": "address"},
                {"name": "tokenId", "type": "uint256"},
                {"name": "makerAmount", "type": "uint256"},
                {"name": "takerAmount", "type": "uint256"},
                {"name": "expiration", "type": "uint256"},
                {"name": "nonce", "type": "uint256"},
                {"name": "feeRateBps", "type": "uint256"},
                {"name": "side", "type": "uint8"},
                {"name": "signatureType", "type": "uint8"}
            ]
        },
        "primaryType": "Order",
        "domain": {
            "name": "Polymarket CTF Exchange",
            "version": "1",
            "chainId": 137,
            "verifyingContract": verifying_contract
        },
        "message": {
            "salt": order.salt.to_string(),
            "maker": order.maker,
            "signer": order.signer,
            "taker": order.taker,
            "tokenId": order.token_id,
            "makerAmount": order.maker_amount.to_string(),
            "takerAmount": order.taker_amount.to_string(),
            "expiration": order.expiration.to_string(),
            "nonce": order.nonce.to_string(),
            "feeRateBps": order.fee_rate_bps.to_string(),
            "side": order.side,
            "signatureType": order.signature_type
        }
    }))
    .expect("Order EIP-712 JSON serialization failed");

    crate::onchainos::sign_eip712(&json).await
}

/// Sign a Polymarket order EIP-712 with the local signing key.
/// Returns the 0x-prefixed hex signature.
pub fn sign_order(
    key: &SigningKey,
    order: &OrderParams,
    neg_risk: bool,
) -> Result<String> {
    let verifying_contract = Contracts::exchange_for(neg_risk);
    let domain_sep = ctf_exchange_domain_sep(verifying_contract)?;

    // Order type hash
    // Order(uint256 salt,address maker,address signer,address taker,uint256 tokenId,
    //        uint256 makerAmount,uint256 takerAmount,uint256 expiration,uint256 nonce,
    //        uint256 feeRateBps,uint8 side,uint8 signatureType)
    let type_hash = keccak256(
        b"Order(uint256 salt,address maker,address signer,address taker,\
uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint256 expiration,\
uint256 nonce,uint256 feeRateBps,uint8 side,uint8 signatureType)",
    );

    let mut struct_buf = Vec::with_capacity(13 * 32);
    struct_buf.extend_from_slice(&type_hash);

    // uint256 salt
    let mut slot = [0u8; 32];
    let salt_bytes = order.salt.to_be_bytes(); // 8 bytes
    slot[24..].copy_from_slice(&salt_bytes);
    struct_buf.extend_from_slice(&slot);

    // address maker
    struct_buf.extend_from_slice(&addr_to_slot(&order.maker)?);
    // address signer
    struct_buf.extend_from_slice(&addr_to_slot(&order.signer)?);
    // address taker
    struct_buf.extend_from_slice(&addr_to_slot(&order.taker)?);

    // uint256 tokenId (full 256-bit decimal string)
    let token_id_big = order.token_id.parse::<BigUint>().context("parsing tokenId")?;
    let token_id_bytes = token_id_big.to_bytes_be();
    anyhow::ensure!(token_id_bytes.len() <= 32, "tokenId exceeds 32 bytes");
    let mut slot = [0u8; 32];
    slot[32 - token_id_bytes.len()..].copy_from_slice(&token_id_bytes);
    struct_buf.extend_from_slice(&slot);

    // uint256 makerAmount
    struct_buf.extend_from_slice(&u64_to_slot(order.maker_amount));
    // uint256 takerAmount
    struct_buf.extend_from_slice(&u64_to_slot(order.taker_amount));
    // uint256 expiration
    struct_buf.extend_from_slice(&u64_to_slot(order.expiration));
    // uint256 nonce
    struct_buf.extend_from_slice(&u64_to_slot(order.nonce));
    // uint256 feeRateBps
    struct_buf.extend_from_slice(&u64_to_slot(order.fee_rate_bps));
    // uint8 side
    struct_buf.extend_from_slice(&u8_to_slot(order.side));
    // uint8 signatureType
    struct_buf.extend_from_slice(&u8_to_slot(order.signature_type));

    let struct_hash = keccak256(&struct_buf);

    let mut digest_buf = [0u8; 66];
    digest_buf[0] = 0x19;
    digest_buf[1] = 0x01;
    digest_buf[2..34].copy_from_slice(&domain_sep);
    digest_buf[34..66].copy_from_slice(&struct_hash);
    let digest = keccak256(&digest_buf);

    sign_digest(key, &digest)
}

// ─── ECDSA helpers ────────────────────────────────────────────────────────────

fn sign_digest(key: &SigningKey, digest: &[u8; 32]) -> Result<String> {
    let (sig, rec_id): (Signature, RecoveryId) = key
        .sign_prehash_recoverable(digest)
        .context("signing digest")?;
    let sig_bytes = sig.to_bytes();
    let v = u8::from(rec_id) + 27u8; // Ethereum: 27 or 28

    // Concatenate r(32) + s(32) + v(1) = 65 bytes
    let mut full = [0u8; 65];
    full[..32].copy_from_slice(&sig_bytes[..32]);
    full[32..64].copy_from_slice(&sig_bytes[32..]);
    full[64] = v;
    Ok(format!("0x{}", hex::encode(full)))
}

fn addr_to_slot(addr: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(addr.strip_prefix("0x").unwrap_or(addr))
        .with_context(|| format!("decoding address {}", addr))?;
    anyhow::ensure!(bytes.len() == 20, "address must be 20 bytes");
    let mut slot = [0u8; 32];
    slot[12..].copy_from_slice(&bytes);
    Ok(slot)
}

fn u64_to_slot(v: u64) -> [u8; 32] {
    let mut slot = [0u8; 32];
    slot[24..].copy_from_slice(&v.to_be_bytes());
    slot
}

fn u8_to_slot(v: u8) -> [u8; 32] {
    let mut slot = [0u8; 32];
    slot[31] = v;
    slot
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;

    #[test]
    fn test_clob_auth_domain_sep_is_32_bytes() {
        let sep = clob_auth_domain_sep();
        assert_eq!(sep.len(), 32);
        assert_ne!(sep, [0u8; 32]);
    }

    #[test]
    fn test_sign_clob_auth_produces_valid_sig() {
        // Use a deterministic test key
        let key_bytes = [1u8; 32];
        let key = SigningKey::from_bytes(key_bytes.as_slice().into()).unwrap();
        let (addr, sig, ts, nonce) = sign_clob_auth(&key, 0).unwrap();
        assert!(addr.starts_with("0x"));
        assert_eq!(addr.len(), 42);
        assert!(sig.starts_with("0x"));
        assert_eq!(sig.len(), 132); // 0x + 65 bytes hex
        assert!(ts > 0);
        assert_eq!(nonce, 0);
    }
}

#[cfg(test)]
mod hash_tests {
    use super::*;

    // Reference values from Python eth_account (fixed timestamp + key)
    // key: ~/.config/polymarket/signing_key.hex
    // wallet: 0xd6E4Cee76C31355a768F886cF50192836bB02F7a
    // ts: 1775651400, nonce: 0
    // Expected domain_sep: cfc66be2a3b30464cb3b588324101f660c9a205fa76e8e5f83ee16a528e1c4cb
    // Expected struct_hash: 8f4f4c77d9ba1e1088e7704a98d365f687e48119469f3ccd7d47fc1760788452
    // Expected final_hash:  141bee5ae90013ed5520415d08bf79cf515f1df7e8e2312b3722127c0d6f26ff

    #[test]
    fn test_domain_sep_matches_python() {
        let sep = clob_auth_domain_sep();
        let expected = hex::decode("cfc66be2a3b30464cb3b588324101f660c9a205fa76e8e5f83ee16a528e1c4cb").unwrap();
        assert_eq!(&sep, expected.as_slice(), "Domain separator mismatch");
    }

    #[test]
    fn test_final_hash_matches_python() {
        let wallet = "0xd6E4Cee76C31355a768F886cF50192836bB02F7a";
        let ts = 1775651400u64;
        let nonce = 0u64;
        let message = "This message attests that I control the given wallet";

        let type_hash = keccak256(
            b"ClobAuth(address address,string timestamp,uint256 nonce,string message)",
        );
        let addr_bytes = hex::decode(wallet.strip_prefix("0x").unwrap()).unwrap();
        let ts_str = ts.to_string();

        let mut struct_buf = Vec::with_capacity(5 * 32);
        struct_buf.extend_from_slice(&type_hash);
        let mut addr_slot = [0u8; 32];
        addr_slot[12..].copy_from_slice(&addr_bytes);
        struct_buf.extend_from_slice(&addr_slot);
        struct_buf.extend_from_slice(&keccak256(ts_str.as_bytes()));
        let mut nonce_slot = [0u8; 32];
        nonce_slot[24..].copy_from_slice(&nonce.to_be_bytes());
        struct_buf.extend_from_slice(&nonce_slot);
        struct_buf.extend_from_slice(&keccak256(message.as_bytes()));

        let struct_hash = keccak256(&struct_buf);
        let expected_struct = hex::decode("8f4f4c77d9ba1e1088e7704a98d365f687e48119469f3ccd7d47fc1760788452").unwrap();
        assert_eq!(&struct_hash, expected_struct.as_slice(), "Struct hash mismatch: got {}", hex::encode(struct_hash));

        let domain_sep = clob_auth_domain_sep();
        let mut digest_buf = [0u8; 66];
        digest_buf[0] = 0x19;
        digest_buf[1] = 0x01;
        digest_buf[2..34].copy_from_slice(&domain_sep);
        digest_buf[34..66].copy_from_slice(&struct_hash);
        let final_hash = keccak256(&digest_buf);

        let expected_final = hex::decode("141bee5ae90013ed5520415d08bf79cf515f1df7e8e2312b3722127c0d6f26ff").unwrap();
        assert_eq!(&final_hash, expected_final.as_slice(), "Final hash mismatch: got {}", hex::encode(final_hash));
    }
}
