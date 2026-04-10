/// EIP-712 order signing for Polymarket CTF Exchange via onchainos.
///
/// All signing is delegated to `onchainos wallet sign-message --type eip712`.
/// No local private key is used or stored by this module.
use anyhow::Result;

/// Parameters for a Polymarket limit order.
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
    pub signature_type: u8, // 0=EOA
}

/// Sign a Polymarket order EIP-712 via `onchainos sign-message --type eip712`.
///
/// Builds a complete EIP-712 structured data JSON with EIP712Domain in `types`
/// (required for correct hash computation — per Hyperliquid root-cause finding).
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
    .expect("Order EIP-712 JSON serialization failed");

    crate::onchainos::sign_eip712(&json).await
}
