// src/config.rs — Chain config and contract addresses

/// Return the RPC URL for a given chain ID.
pub fn rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        8453 => "https://base-rpc.publicnode.com",
        42161 => "https://arb1.arbitrum.io/rpc",
        _ => "https://base-rpc.publicnode.com",
    }
}

/// Clanker Factory address keyed by chain ID (v4.0.0).
/// Used to dynamically resolve the fee locker via `feeLockerForToken(address)`.
pub fn factory_address(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        8453 => Some("0xE85A59c628F7d27878ACeB4bf3b35733630083a9"),
        // Arbitrum factory — resolve at runtime via gitbook docs; omit hardcode
        _ => None,
    }
}

/// Fallback ClankerFeeLocker address for Base v4.0 (used if factory lookup fails).
/// 0x63D2DfEA64b3433F4071A98665bcD7Ca14d93496 is the verified V4 locker used by recent Clanker tokens.
/// It exposes tokenRewards(address) and collectRewards(address) (not collectFees).
pub fn fallback_fee_locker(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        8453 => Some("0x63D2DfEA64b3433F4071A98665bcD7Ca14d93496"),
        _ => None,
    }
}
