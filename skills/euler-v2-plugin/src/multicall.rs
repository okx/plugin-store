//! Multicall3 helper — bundle many `eth_call`s into one RPC round-trip.
//!
//! Multicall3 is deployed at the same address `0xcA11bde05977b3631167028862bE2a173976CA11`
//! on every EVM chain. We use `aggregate3((address,bool,bytes)[])` which lets us
//! configure per-call `allowFailure` so a single bad subcall doesn't poison the batch.
//!
//! Why we use it:
//!   - `positions` scans 100+ vaults: 200+ individual eth_calls → 1 multicall (~10× faster)
//!   - `health-factor` needs ~5 reads per position (balanceOf, previewRedeem, debtOf,
//!     LTVBorrow, oracle.getQuote): batching keeps it within a single RPC round-trip

use anyhow::{Context, Result};

/// Canonical Multicall3 address (same on every EVM chain).
pub const MULTICALL3: &str = "0xcA11bde05977b3631167028862bE2a173976CA11";

/// `aggregate3((address,bool,bytes)[])` selector.
/// keccak256("aggregate3((address,bool,bytes)[])")[:4] = 0x82ad56cb
const SEL_AGGREGATE3: &str = "82ad56cb";

/// One sub-call inside a Multicall3 batch.
#[derive(Clone, Debug)]
pub struct Call3 {
    pub target: String,
    pub allow_failure: bool,
    pub calldata: String, // 0x-prefixed hex
}

/// Result of one sub-call.
#[derive(Clone, Debug)]
pub struct Call3Result {
    pub success:    bool,
    pub return_data: Vec<u8>,
}

impl Call3Result {
    /// Decode return data as uint256 → u128 (saturating).
    pub fn as_u128(&self) -> Option<u128> {
        if !self.success || self.return_data.is_empty() { return None; }
        let hex = hex::encode(&self.return_data);
        Some(crate::rpc::parse_uint256_to_u128(&format!("0x{}", hex)))
    }

    /// Decode return data as a 20-byte address (with `0x` prefix).
    pub fn as_address(&self) -> Option<String> {
        if !self.success || self.return_data.len() < 32 { return None; }
        // address is right-aligned in the last 20 bytes of the 32-byte word
        let bytes = &self.return_data[12..32];
        Some(format!("0x{}", hex::encode(bytes)))
    }
}

/// Encode `aggregate3` calldata for `Call3[]`.
fn encode_aggregate3(calls: &[Call3]) -> String {
    // Layout (after 4-byte selector):
    //   [0]   offset to array data (always 0x20)
    //   [1]   array length
    //   [2..n+2]   per-call offsets (relative to start of array data section,
    //              which begins right after the length word; so offset is
    //              from word [2])
    //   ...   per-call tuples
    //
    // Each Call3 tuple (which contains a dynamic `bytes` field) is itself
    // dynamic, so the array uses offset-based encoding.

    let n = calls.len();
    let mut out = String::new();
    out.push_str("0x");
    out.push_str(SEL_AGGREGATE3);
    out.push_str(&format!("{:064x}", 0x20u32));   // offset to array
    out.push_str(&format!("{:064x}", n));         // array length

    // Compute offsets to each tuple. Tuples follow the offset table.
    // Offset is measured from start of the array data section (= after length word).
    // Offset table itself is N words.
    let offset_table_size_bytes = (n * 32) as u64;
    let mut tuples_hex: Vec<String> = Vec::with_capacity(n);

    let _accumulated_tuple_offset = offset_table_size_bytes;
    for c in calls {
        // Encode this one tuple
        // tuple layout (all 32-byte words):
        //   [0] target (address)
        //   [1] allowFailure (bool)
        //   [2] offset to bytes data (= 0x60, since 3 static slots × 32)
        //   [3] bytes data length
        //   [4..] bytes data padded to 32
        let cd_no_prefix = c.calldata.trim_start_matches("0x");
        let cd_bytes_len = cd_no_prefix.len() / 2;
        let data_padded_words = (cd_bytes_len + 31) / 32;
        let data_padded_chars = data_padded_words * 64;

        let target_padded = crate::rpc::pad_address(&c.target);
        let allow_failure_padded = format!("{:064x}", if c.allow_failure { 1u8 } else { 0u8 });

        let mut t = String::new();
        t.push_str(&target_padded);
        t.push_str(&allow_failure_padded);
        t.push_str(&format!("{:064x}", 0x60u32));            // bytes offset within tuple
        t.push_str(&format!("{:064x}", cd_bytes_len));
        t.push_str(cd_no_prefix);
        t.push_str(&"0".repeat(data_padded_chars - cd_no_prefix.len()));

        tuples_hex.push(t);

        // Tuple length is recomputed in the offset table below.
    }

    // Now compute and emit offset table, then concat all tuples
    let mut next_offset = offset_table_size_bytes;
    let mut offset_table_hex = String::new();
    for t in &tuples_hex {
        offset_table_hex.push_str(&format!("{:064x}", next_offset));
        next_offset += (t.len() / 2) as u64;
    }
    out.push_str(&offset_table_hex);
    for t in &tuples_hex {
        out.push_str(t);
    }
    out
}

/// Decode `aggregate3` return data: an array of `(bool success, bytes returnData)` tuples.
fn decode_aggregate3(hex_result: &str) -> Result<Vec<Call3Result>> {
    let raw = hex_result.trim_start_matches("0x");
    if raw.len() < 128 {
        anyhow::bail!("aggregate3 result too short to contain even array header");
    }
    let bytes = hex::decode(raw).context("decoding aggregate3 hex result")?;

    // [0..32]   offset to array data (should be 0x20)
    // [32..64]  array length
    let array_len = u64::from_be_bytes(bytes[56..64].try_into().unwrap()) as usize;

    // After the length word, we have N offset words pointing to each tuple
    let array_data_start = 64; // bytes index where the offset table starts
    let mut results = Vec::with_capacity(array_len);
    for i in 0..array_len {
        let off_idx = array_data_start + i * 32;
        let item_offset = u64::from_be_bytes(bytes[off_idx + 24..off_idx + 32].try_into().unwrap()) as usize;
        // Tuple location = end of length word + item_offset
        let tuple_start = array_data_start + item_offset;

        // Tuple layout: [0] success (bool, 32 bytes), [1] offset to bytes (32 bytes),
        //               [2] bytes length, [3+] bytes data
        let success = bytes[tuple_start + 31] != 0;
        let bytes_offset = u64::from_be_bytes(bytes[tuple_start + 32 + 24..tuple_start + 64].try_into().unwrap()) as usize;
        let bytes_data_start = tuple_start + bytes_offset;
        let bytes_len = u64::from_be_bytes(bytes[bytes_data_start + 24..bytes_data_start + 32].try_into().unwrap()) as usize;
        let return_data = bytes[bytes_data_start + 32..bytes_data_start + 32 + bytes_len].to_vec();

        results.push(Call3Result { success, return_data });
    }
    Ok(results)
}

/// Run a batch of read-only calls in a single eth_call to Multicall3.
///
/// All calls run in parallel on-chain in the same eth_call invocation.
/// `allow_failure: true` means a sub-call's revert returns `success=false`
/// instead of poisoning the entire batch.
pub async fn aggregate3(chain_id: u64, calls: &[Call3]) -> Result<Vec<Call3Result>> {
    if calls.is_empty() { return Ok(Vec::new()); }
    let calldata = encode_aggregate3(calls);
    let result_hex = crate::rpc::eth_call(chain_id, MULTICALL3, &calldata).await?;
    decode_aggregate3(&result_hex)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha3::{Digest, Keccak256};

    #[test]
    fn aggregate3_selector_matches() {
        let s = Keccak256::digest(b"aggregate3((address,bool,bytes)[])");
        assert_eq!(hex::encode(&s[..4]), SEL_AGGREGATE3);
    }

    #[test]
    fn encode_single_call_shape() {
        let calls = vec![Call3 {
            target: "0x1111111111111111111111111111111111111111".into(),
            allow_failure: true,
            calldata: "0x70a082310000000000000000000000002222222222222222222222222222222222222222".into(),
        }];
        let cd = encode_aggregate3(&calls);
        assert!(cd.starts_with("0x82ad56cb"));
        // Top-level offset 0x20 + length 1 = first 128 hex after selector
        assert!(cd.contains("0000000000000000000000000000000000000000000000000000000000000020"));
        assert!(cd.contains("0000000000000000000000000000000000000000000000000000000000000001"));
    }
}
