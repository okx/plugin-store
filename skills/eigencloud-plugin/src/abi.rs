use tiny_keccak::{Hasher, Keccak};

/// Compute the 4-byte function selector from a canonical signature string.
pub fn selector(sig: &str) -> [u8; 4] {
    let mut k = Keccak::v256();
    k.update(sig.as_bytes());
    let mut out = [0u8; 32];
    k.finalize(&mut out);
    [out[0], out[1], out[2], out[3]]
}

/// Encode an Ethereum address as 32 bytes (left-padded with zeros).
pub fn encode_address(addr: &str) -> [u8; 32] {
    let addr = addr.trim_start_matches("0x").trim_start_matches("0X");
    let bytes = hex::decode(addr).unwrap_or_default();
    let mut out = [0u8; 32];
    let start = 32 - bytes.len().min(20);
    out[start..start + bytes.len().min(20)].copy_from_slice(&bytes[..bytes.len().min(20)]);
    out
}

/// Encode a u128 as a 32-byte big-endian uint256.
pub fn encode_uint256(n: u128) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[16..].copy_from_slice(&n.to_be_bytes());
    out
}

/// Encode a u256 given as a hex string (0x...) into 32 bytes.
pub fn encode_uint256_hex(s: &str) -> [u8; 32] {
    let s = s.trim_start_matches("0x").trim_start_matches("0X");
    let bytes = hex::decode(format!("{:0>64}", s)).unwrap_or_default();
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes[..32]);
    out
}

/// Zero bytes32
pub fn zero32() -> [u8; 32] {
    [0u8; 32]
}

/// Build calldata: selector + concatenated 32-byte slots.
pub fn calldata(sel: [u8; 4], slots: &[[u8; 32]]) -> String {
    let mut bytes = sel.to_vec();
    for slot in slots {
        bytes.extend_from_slice(slot);
    }
    format!("0x{}", hex::encode(bytes))
}

/// Encode address as hex string for use in eth_call data (no 0x prefix, 64 chars).
pub fn hex_encode(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selectors_match_known() {
        // Verified against on-chain calls (0x94f649dd works for getDeposits on StrategyManager)
        assert_eq!(hex::encode(selector("getDeposits(address)")), "94f649dd");
        assert_eq!(hex::encode(selector("approve(address,uint256)")), "095ea7b3");
        assert_eq!(hex::encode(selector("depositIntoStrategy(address,address,uint256)")), "e7a050aa");
    }
}
