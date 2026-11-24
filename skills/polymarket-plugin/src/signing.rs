/// EIP-712 order signing for Polymarket CTF Exchange via onchainos.
///
/// V1: legacy exchange (0x4bFb41...), domain version "1", struct has taker/nonce/feeRateBps.
/// V2: new exchange (0xE11118...), domain version "2", struct has timestamp/metadata/builder.
///
/// All signing is delegated to `onchainos wallet sign-message --type eip712`.
/// No local private key is used or stored by this module.
use anyhow::Result;

// ─── V1 ───────────────────────────────────────────────────────────────────────

/// Parameters for a Polymarket V1 limit order.
pub struct OrderParams {
    pub salt: u64,
    pub maker: String,
    pub signer: String,
    pub taker: String,
    pub token_id: String,
    pub maker_amount: u64,
    pub taker_amount: u64,
    pub expiration: u64,
    pub nonce: u64,
    pub fee_rate_bps: u64,
    pub side: u8,           // 0=BUY, 1=SELL
    pub signature_type: u8, // 0=EOA, 1=Proxy
}

/// Sign a V1 Polymarket order via `onchainos sign-message --type eip712`.
pub async fn sign_order_via_onchainos(order: &OrderParams, neg_risk: bool) -> Result<String> {
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
    .expect("V1 Order EIP-712 JSON serialization failed");

    crate::onchainos::sign_eip712(&json).await
}

// ─── V2 ───────────────────────────────────────────────────────────────────────

/// Parameters for a Polymarket V2 limit order.
///
/// Key differences from V1:
///   - No `taker`, `nonce`, or `feeRateBps` (fees are now protocol-enforced)
///   - `expiration` is NOT in the signed struct (it goes in the outer API request wrapper)
///   - `timestamp_ms`: millisecond Unix timestamp added to the signed struct
///   - `metadata`: bytes32 optional metadata (zero for standard orders)
///   - `builder`: bytes32 builder code for fee attribution (zero for non-builders)
pub struct OrderParamsV2 {
    pub salt: u64,
    pub maker: String,
    pub signer: String,
    pub token_id: String,
    pub maker_amount: u64,
    pub taker_amount: u64,
    pub side: u8,           // 0=BUY, 1=SELL
    pub signature_type: u8, // 0=EOA, 1=Proxy, 2=GnosisSafe, 3=POLY_1271
    pub timestamp_ms: u64,  // millisecond Unix timestamp
    pub metadata: String,   // bytes32 hex: "0x000...000" for standard orders
    pub builder: String,    // bytes32 hex: "0x000...000" for non-builders
}

/// Sign a V2 Polymarket order via `onchainos sign-message --type eip712`.
///
/// Uses domain version "2" and the new V2 exchange contract address.
/// `expiration` is not part of the signed struct in V2 — pass it separately in the API body.
pub async fn sign_order_v2_via_onchainos(order: &OrderParamsV2, neg_risk: bool) -> Result<String> {
    use crate::config::Contracts;
    let verifying_contract = Contracts::exchange_for_v2(neg_risk);

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
                {"name": "tokenId", "type": "uint256"},
                {"name": "makerAmount", "type": "uint256"},
                {"name": "takerAmount", "type": "uint256"},
                {"name": "side", "type": "uint8"},
                {"name": "signatureType", "type": "uint8"},
                {"name": "timestamp", "type": "uint256"},
                {"name": "metadata", "type": "bytes32"},
                {"name": "builder", "type": "bytes32"}
            ]
        },
        "primaryType": "Order",
        "domain": {
            "name": "Polymarket CTF Exchange",
            "version": "2",
            "chainId": 137,
            "verifyingContract": verifying_contract
        },
        "message": {
            "salt": order.salt.to_string(),
            "maker": order.maker,
            "signer": order.signer,
            "tokenId": order.token_id,
            "makerAmount": order.maker_amount.to_string(),
            "takerAmount": order.taker_amount.to_string(),
            "side": order.side,
            "signatureType": order.signature_type,
            "timestamp": order.timestamp_ms.to_string(),
            "metadata": order.metadata,
            "builder": order.builder
        }
    }))
    .expect("V2 Order EIP-712 JSON serialization failed");

    crate::onchainos::sign_eip712(&json).await
}

/// Bytes32 zero value — used as default metadata and builder in V2 orders.
pub const BYTES32_ZERO: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

// ─── Deposit Wallet (sig_type=3 / POLY_1271) ─────────────────────────────────

/// A single call in a DepositWallet batch.
pub struct WalletCall {
    pub target: String, // contract to call
    pub value: u64,
    pub data: String, // hex-encoded calldata
}

/// Parameters for signing a DepositWallet batch transaction.
///
/// The batch is submitted via the Polymarket relayer (POST /submit, type=WALLET).
/// The owner EOA signs an EIP-712 `Batch` struct; the deposit wallet contract
/// validates via ERC-1271 (`isValidSignature`).
///
/// Domain: name="DepositWallet", version="1", chainId=137, verifyingContract=<wallet_addr>
pub struct BatchParams {
    pub wallet: String,   // deposit wallet address (verifying contract)
    pub nonce: u64,
    pub deadline: u64,    // Unix timestamp
    pub calls: Vec<WalletCall>,
}

/// Sign a DepositWallet batch via `onchainos wallet sign-message --type eip712`.
pub async fn sign_batch_via_onchainos(params: &BatchParams) -> Result<String> {
    let calls_json: Vec<serde_json::Value> = params.calls.iter().map(|c| {
        serde_json::json!({
            "target": c.target,
            "value":  c.value.to_string(),
            "data":   c.data,
        })
    }).collect();

    let json = serde_json::to_string(&serde_json::json!({
        "types": {
            "EIP712Domain": [
                {"name": "name",             "type": "string"},
                {"name": "version",          "type": "string"},
                {"name": "chainId",          "type": "uint256"},
                {"name": "verifyingContract","type": "address"}
            ],
            "Call": [
                {"name": "target", "type": "address"},
                {"name": "value",  "type": "uint256"},
                {"name": "data",   "type": "bytes"}
            ],
            "Batch": [
                {"name": "wallet",   "type": "address"},
                {"name": "nonce",    "type": "uint256"},
                {"name": "deadline", "type": "uint256"},
                {"name": "calls",    "type": "Call[]"}
            ]
        },
        "primaryType": "Batch",
        "domain": {
            "name": "DepositWallet",
            "version": "1",
            "chainId": 137,
            "verifyingContract": params.wallet,
        },
        "message": {
            "wallet":   params.wallet,
            "nonce":    params.nonce.to_string(),
            "deadline": params.deadline.to_string(),
            "calls":    calls_json,
        }
    }))
    .expect("Batch EIP-712 JSON serialization failed");

    crate::onchainos::sign_eip712(&json).await
}

/// Sign a V2 order for a deposit wallet (POLY_1271 / ERC-1271, signature_type=3).
///
/// POLY_1271 uses Solady's ERC-7739 TypedDataSign composite signature format.
/// The deposit wallet's `isValidSignature` (Solady ERC-1271 + ERC-7739) requires:
///
/// Wire format (composite signature):
///   [65 bytes: ECDSA sig by EOA over TypedDataSign digest]
///   [32 bytes: app_domain_separator (CTF Exchange V2)]
///   [32 bytes: contents_hash (Order struct hash, without domain prefix)]
///   [N bytes: ORDER_TYPE_STRING as UTF-8]
///   [2 bytes: len(ORDER_TYPE_STRING) big-endian]
///
/// The EOA signs:
///   keccak256("\x19\x01" || CTF_Exchange_domain_sep || TypedDataSign_struct_hash)
/// where TypedDataSign_struct_hash encodes (contents_hash, wallet_name, wallet_version,
/// chainId, deposit_wallet_addr, salt=0) using the Solady POLY_1271 type string.
///
/// Verification flow (Solady ERC-7739 in deposit wallet):
///   1. Decode trailer → (app_domain_sep, contents_hash, ORDER_TYPE_STRING)
///   2. Check keccak256("\x19\x01" || app_domain_sep || contents_hash) == clob_passed_hash
///   3. Reconstruct TypedDataSign_struct_hash using wallet's own domain + decoded type string
///   4. Verify ecrecover(final_digest, 65_byte_sig) == owner EOA
pub async fn sign_order_v2_poly1271_via_onchainos(order: &OrderParamsV2, neg_risk: bool) -> Result<String> {
    use crate::config::Contracts;
    use sha3::{Digest, Keccak256};

    let exchange = Contracts::exchange_for_v2(neg_risk);

    // V2 order type string — must exactly match the on-chain ABI type hash
    const ORDER_TYPE_STRING: &str = "Order(uint256 salt,address maker,address signer,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint8 side,uint8 signatureType,uint256 timestamp,bytes32 metadata,bytes32 builder)";

    // ── Step 1: Sign via EIP-712 TypedDataSign JSON ───────────────────────────
    // Domain: CTF Exchange V2 (the "app" domain — outer \x19\x01 separator)
    // PrimaryType: TypedDataSign
    // Message:
    //   contents (Order): the actual order fields (hashed as nested struct by onchainos)
    //   name/version/chainId/verifyingContract/salt: deposit wallet's own EIP-712 domain
    //     (embedded inside the TypedDataSign struct for wallet-specific replay protection)
    let typed_data_sign_json = serde_json::to_string(&serde_json::json!({
        "types": {
            "EIP712Domain": [
                {"name": "name", "type": "string"},
                {"name": "version", "type": "string"},
                {"name": "chainId", "type": "uint256"},
                {"name": "verifyingContract", "type": "address"}
            ],
            "TypedDataSign": [
                {"name": "contents", "type": "Order"},
                {"name": "name", "type": "string"},
                {"name": "version", "type": "string"},
                {"name": "chainId", "type": "uint256"},
                {"name": "verifyingContract", "type": "address"},
                {"name": "salt", "type": "bytes32"}
            ],
            "Order": [
                {"name": "salt", "type": "uint256"},
                {"name": "maker", "type": "address"},
                {"name": "signer", "type": "address"},
                {"name": "tokenId", "type": "uint256"},
                {"name": "makerAmount", "type": "uint256"},
                {"name": "takerAmount", "type": "uint256"},
                {"name": "side", "type": "uint8"},
                {"name": "signatureType", "type": "uint8"},
                {"name": "timestamp", "type": "uint256"},
                {"name": "metadata", "type": "bytes32"},
                {"name": "builder", "type": "bytes32"}
            ]
        },
        "primaryType": "TypedDataSign",
        "domain": {
            "name": "Polymarket CTF Exchange",
            "version": "2",
            "chainId": 137,
            "verifyingContract": exchange
        },
        "message": {
            "contents": {
                "salt": order.salt.to_string(),
                "maker": order.maker,
                "signer": order.signer,
                "tokenId": order.token_id,
                "makerAmount": order.maker_amount.to_string(),
                "takerAmount": order.taker_amount.to_string(),
                "side": order.side,
                "signatureType": order.signature_type,
                "timestamp": order.timestamp_ms.to_string(),
                "metadata": order.metadata,
                "builder": order.builder
            },
            // Deposit wallet's own EIP-712 domain fields (Solady DepositWallet contract)
            "name": "DepositWallet",
            "version": "1",
            "chainId": 137,
            "verifyingContract": order.signer,  // deposit wallet address
            "salt": BYTES32_ZERO
        }
    })).expect("TypedDataSign EIP-712 JSON serialization failed");

    let ecdsa_sig_hex = crate::onchainos::sign_eip712(&typed_data_sign_json).await?;
    let ecdsa_bytes = hex::decode(ecdsa_sig_hex.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Failed to decode ERC-7739 ECDSA signature: {}", e))?;
    if ecdsa_bytes.len() != 65 {
        return Err(anyhow::anyhow!("Expected 65-byte ECDSA signature, got {} bytes", ecdsa_bytes.len()));
    }

    // ── Step 2: Compute app_domain_separator = CTF Exchange V2 domain separator ─
    // = keccak256(abi.encode(DOMAIN_TYPE_HASH, name_hash, version_hash, chainId, verifyingContract))
    // This must match what the CLOB computes when calling isValidSignature.
    let domain_type_hash = Keccak256::digest(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
    );
    let name_hash = Keccak256::digest(b"Polymarket CTF Exchange");
    let version_hash = Keccak256::digest(b"2");
    let exchange_addr_bytes = hex::decode(exchange.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid exchange address '{}': {}", exchange, e))?;

    let mut domain_enc = vec![0u8; 5 * 32]; // 5 fields × 32 bytes
    domain_enc[0..32].copy_from_slice(&domain_type_hash);
    domain_enc[32..64].copy_from_slice(&name_hash);
    domain_enc[64..96].copy_from_slice(&version_hash);
    // chainId = 137 as uint256 (right-aligned in 32-byte slot)
    domain_enc[124..128].copy_from_slice(&137u32.to_be_bytes());
    // verifyingContract = exchange addr (20 bytes, right-aligned: bytes 128+12..128+32)
    domain_enc[140..160].copy_from_slice(&exchange_addr_bytes);
    let mut app_domain_sep = [0u8; 32];
    app_domain_sep.copy_from_slice(&Keccak256::digest(&domain_enc));

    // ── Step 3: Compute contents_hash = Order struct hash ─────────────────────
    // = keccak256(abi.encode(ORDER_TYPE_HASH, salt, maker, signer, tokenId,
    //             makerAmount, takerAmount, side, signatureType, timestamp, metadata, builder))
    // Each field is ABI-encoded as a 32-byte word (addresses left-padded, uints right-aligned).
    let order_type_hash = Keccak256::digest(ORDER_TYPE_STRING.as_bytes());

    let maker_addr = hex::decode(order.maker.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid maker address: {}", e))?;
    let signer_addr = hex::decode(order.signer.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid signer address: {}", e))?;
    let token_id_bytes = decimal_to_u256_bytes(&order.token_id)
        .map_err(|e| anyhow::anyhow!("Invalid tokenId '{}': {}", order.token_id, e))?;
    let metadata_bytes = hex::decode(order.metadata.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid metadata: {}", e))?;
    let builder_bytes_dec = hex::decode(order.builder.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid builder: {}", e))?;

    let mut order_enc = vec![0u8; 12 * 32]; // 12 fields × 32 bytes

    // Field 0 (bytes   0-31): ORDER_TYPE_HASH (bytes32)
    order_enc[0..32].copy_from_slice(&order_type_hash);
    // Field 1 (bytes  32-63): salt (uint256, u64 → right-aligned)
    order_enc[56..64].copy_from_slice(&order.salt.to_be_bytes());
    // Field 2 (bytes  64-95): maker (address → 12 zero bytes + 20 addr bytes)
    order_enc[76..96].copy_from_slice(&maker_addr);
    // Field 3 (bytes  96-127): signer (address)
    order_enc[108..128].copy_from_slice(&signer_addr);
    // Field 4 (bytes 128-159): tokenId (uint256, full 32 bytes)
    order_enc[128..160].copy_from_slice(&token_id_bytes);
    // Field 5 (bytes 160-191): makerAmount (uint256, u64 → right-aligned)
    order_enc[184..192].copy_from_slice(&order.maker_amount.to_be_bytes());
    // Field 6 (bytes 192-223): takerAmount (uint256, u64 → right-aligned)
    order_enc[216..224].copy_from_slice(&order.taker_amount.to_be_bytes());
    // Field 7 (bytes 224-255): side (uint8 → right-aligned)
    order_enc[255] = order.side;
    // Field 8 (bytes 256-287): signatureType (uint8 → right-aligned)
    order_enc[287] = order.signature_type;
    // Field 9 (bytes 288-319): timestamp (uint256, u64 → right-aligned)
    order_enc[312..320].copy_from_slice(&order.timestamp_ms.to_be_bytes());
    // Field 10 (bytes 320-351): metadata (bytes32)
    order_enc[320..352].copy_from_slice(&metadata_bytes);
    // Field 11 (bytes 352-383): builder (bytes32)
    order_enc[352..384].copy_from_slice(&builder_bytes_dec);

    let mut contents_hash = [0u8; 32];
    contents_hash.copy_from_slice(&Keccak256::digest(&order_enc));

    // ── Step 4: Assemble Solady ERC-7739 composite signature wire format ───────
    // [65 bytes: ECDSA sig] || [32 bytes: app_domain_sep] || [32 bytes: contents_hash]
    // || [N bytes: ORDER_TYPE_STRING as UTF-8] || [2 bytes: len(ORDER_TYPE_STRING) big-endian]
    //
    // Solady's isValidSignature decodes this trailer to reconstruct and verify the hash,
    // then checks the 65-byte ECDSA sig against the TypedDataSign digest.
    let order_type_str_bytes = ORDER_TYPE_STRING.as_bytes();
    let order_type_str_len = order_type_str_bytes.len() as u16;

    let mut composite = Vec::with_capacity(65 + 32 + 32 + order_type_str_bytes.len() + 2);
    composite.extend_from_slice(&ecdsa_bytes);
    composite.extend_from_slice(&app_domain_sep);
    composite.extend_from_slice(&contents_hash);
    composite.extend_from_slice(order_type_str_bytes);
    composite.extend_from_slice(&order_type_str_len.to_be_bytes());

    Ok(format!("0x{}", hex::encode(&composite)))
}

/// Convert a decimal string to a 32-byte big-endian U256 array.
/// Used for tokenId encoding in EIP-712 ABI encoding (Polymarket token IDs exceed u128).
fn decimal_to_u256_bytes(s: &str) -> Result<[u8; 32]> {
    let mut result = [0u8; 32];
    for &byte in s.as_bytes() {
        if byte < b'0' || byte > b'9' {
            return Err(anyhow::anyhow!("Invalid decimal digit '{}' in '{}'", byte as char, s));
        }
        let digit = byte - b'0';
        // result = result * 10 + digit (big-endian multi-precision arithmetic)
        let mut carry: u32 = digit as u32;
        for i in (0..32).rev() {
            let val: u32 = result[i] as u32 * 10 + carry;
            result[i] = val as u8;
            carry = val >> 8;
        }
        if carry != 0 {
            return Err(anyhow::anyhow!("Value '{}' too large for 256-bit integer", s));
        }
    }
    Ok(result)
}
