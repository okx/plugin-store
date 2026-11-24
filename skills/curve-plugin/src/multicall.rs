// multicall.rs — Multicall3 batch eth_calls
// aggregate3((address target, bool allowFailure, bytes callData)[])
// Deployed at 0xcA11bde05977b3631167028862bE2a173976CA11 on all major chains

use crate::rpc;

const MULTICALL3: &str = "0xcA11bde05977b3631167028862bE2a173976CA11";

/// Encode aggregate3((address,bool,bytes)[]) calldata.
/// Each call is (target_address, calldata_bytes). allowFailure=true for all.
fn encode_aggregate3(calls: &[(String, Vec<u8>)]) -> String {
    let n = calls.len();

    // Each element encoding size for calldata length d:
    // target(32) + allowFailure(32) + bytes_offset(32=0x60) + bytes_len(32) + ceil(d/32)*32
    let element_size = |d: usize| 128 + ((d + 31) / 32) * 32;

    // Offsets relative to start of head section (= after the array length word).
    // Head section = N * 32 bytes (one offset word per element).
    let head_size = n * 32;
    let mut offsets = Vec::with_capacity(n);
    let mut acc = head_size;
    for (_, cd) in calls.iter() {
        offsets.push(acc);
        acc += element_size(cd.len());
    }

    let mut buf: Vec<u8> = Vec::new();

    // selector: aggregate3((address,bool,bytes)[]) = 0x82ad56cb
    buf.extend_from_slice(&[0x82, 0xad, 0x56, 0xcb]);
    // offset to calls[] = 32
    buf.extend_from_slice(&u256(32));
    // array length
    buf.extend_from_slice(&u256(n as u64));
    // element offsets (head section)
    for &off in &offsets {
        buf.extend_from_slice(&u256(off as u64));
    }
    // element data (tail section)
    for (target, cd) in calls.iter() {
        let addr = hex::decode(target.trim_start_matches("0x")).unwrap_or_default();
        let mut padded_addr = [0u8; 32];
        padded_addr[32 - addr.len()..].copy_from_slice(&addr);
        buf.extend_from_slice(&padded_addr);           // target
        buf.extend_from_slice(&u256(1));               // allowFailure = true
        buf.extend_from_slice(&u256(0x60));            // offset to bytes = 96 (always)
        buf.extend_from_slice(&u256(cd.len() as u64)); // bytes length
        let padded_len = ((cd.len() + 31) / 32) * 32;
        let mut padded = vec![0u8; padded_len.max(32)];
        padded[..cd.len()].copy_from_slice(cd);
        buf.extend_from_slice(&padded);                // bytes data
    }

    format!("0x{}", hex::encode(&buf))
}

/// Decode aggregate3 return value.
/// Returns Vec<Option<Vec<u8>>> — None means the individual call failed.
fn decode_aggregate3_result(hex_result: &str) -> Vec<Option<Vec<u8>>> {
    let raw = hex_result.trim_start_matches("0x");
    let bytes = match hex::decode(raw) {
        Ok(b) => b,
        Err(_) => return vec![],
    };
    // bytes[0..32]  = offset to array = 0x20
    // bytes[32..64] = array length N
    if bytes.len() < 64 {
        return vec![];
    }
    let n = u64_from_slice(&bytes[32..64]) as usize;
    if bytes.len() < 64 + n * 32 {
        return vec![];
    }

    // Head section starts at byte 64; each word is offset relative to head start.
    let head_start = 64usize;
    let mut results = Vec::with_capacity(n);

    for i in 0..n {
        let off_pos = head_start + i * 32;
        let elem_off = u64_from_slice(&bytes[off_pos..off_pos + 32]) as usize;
        let elem_start = head_start + elem_off;

        if bytes.len() < elem_start + 64 {
            results.push(None);
            continue;
        }
        // Result[i] = (bool success, bytes returnData)
        // success  at elem_start      (32 bytes, low bit)
        // bytes_off at elem_start+32  (32 bytes, relative to elem_start)
        let success = bytes[elem_start + 31] != 0;
        let bytes_rel_off = u64_from_slice(&bytes[elem_start + 32..elem_start + 64]) as usize;
        let bytes_start = elem_start + bytes_rel_off;

        if bytes.len() < bytes_start + 32 {
            results.push(None);
            continue;
        }
        let bytes_len = u64_from_slice(&bytes[bytes_start..bytes_start + 32]) as usize;
        let data_start = bytes_start + 32;

        if !success || bytes.len() < data_start + bytes_len {
            results.push(None);
            continue;
        }
        results.push(Some(bytes[data_start..data_start + bytes_len].to_vec()));
    }
    results
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn u256(val: u64) -> [u8; 32] {
    let mut b = [0u8; 32];
    b[24..].copy_from_slice(&val.to_be_bytes());
    b
}

fn u64_from_slice(s: &[u8]) -> u64 {
    let start = s.len().saturating_sub(8);
    let arr: [u8; 8] = s[start..].try_into().unwrap_or([0u8; 8]);
    u64::from_be_bytes(arr)
}

// ── public API ────────────────────────────────────────────────────────────────

/// Execute a batch of eth_calls via Multicall3 aggregate3.
/// Returns one Option<Vec<u8>> per call; None = call reverted (allowFailure=true).
pub async fn batch_call(
    calls: Vec<(String, Vec<u8>)>,
    rpc_url: &str,
) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
    if calls.is_empty() {
        return Ok(vec![]);
    }
    let calldata = encode_aggregate3(&calls);
    let result = rpc::eth_call(MULTICALL3, &calldata, rpc_url).await?;
    Ok(decode_aggregate3_result(&result))
}

/// Extract a 20-byte address from a 32-byte ABI-encoded word.
/// Falls back to `fallback` on error (used for pool.token() → LP token address).
pub fn decode_address(data: &[u8], fallback: &str) -> String {
    if data.len() < 32 {
        return fallback.to_string();
    }
    let addr = format!("0x{}", hex::encode(&data[12..32]));
    if addr == "0x0000000000000000000000000000000000000000" {
        fallback.to_string()
    } else {
        addr
    }
}

/// Extract a u128 from a 32-byte ABI-encoded uint256 (takes low 16 bytes).
pub fn decode_u128(data: &[u8]) -> u128 {
    if data.len() < 32 {
        return 0;
    }
    u128::from_be_bytes(data[16..32].try_into().unwrap_or([0u8; 16]))
}
