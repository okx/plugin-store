use serde_json::json;

use crate::config::{resolve_token_address, pad_address, LB_FACTORY, RPC_URL};
use crate::rpc::eth_call;

/// Build getAllLBPairs(tokenX, tokenY) calldata.
/// Selector: 0x6622e0d7
pub fn build_get_all_lb_pairs_calldata(token_x: &str, token_y: &str) -> String {
    // getAllLBPairs(address,address): two static address params
    format!(
        "0x6622e0d7{}{}",
        pad_address(token_x),
        pad_address(token_y)
    )
}

/// Build getActiveId() calldata on an LBPair.
/// Selector: cast sig "getActiveId()" = 0xdbe65edc
pub fn build_get_active_id_calldata() -> String {
    "0xdbe65edc".to_string()
}

/// Parse LBPairInformation[] ABI-encoded response from getAllLBPairs.
/// Each LBPairInformation struct is:
///   uint16 binStep  (slot 0, padded to 32 bytes)
///   ILBPair LBPair  (address, slot 1)
///   bool createdByOwner (slot 2)
///   bool ignoredForRouting (slot 3)
/// So each struct is 4 * 32 = 128 bytes = 256 hex chars.
pub fn parse_lb_pair_information(hex: &str) -> Vec<(u16, String, bool, bool)> {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 64 {
        return vec![];
    }
    // First word: offset to array start (should be 0x20)
    // Second word: array length
    let arr_ptr = usize::from_str_radix(&clean[0..64], 16).unwrap_or(0x20);
    let arr_start = arr_ptr * 2;
    if clean.len() < arr_start + 64 {
        return vec![];
    }
    let len = usize::from_str_radix(&clean[arr_start..arr_start + 64], 16).unwrap_or(0);
    let data_start = arr_start + 64;

    let mut pairs = Vec::new();
    for i in 0..len {
        let struct_offset = data_start + i * 256; // 4 slots * 64 hex chars each = 256
        if clean.len() < struct_offset + 256 {
            break;
        }
        // Slot 0: binStep (uint16, right-aligned)
        let bin_step_hex = &clean[struct_offset..struct_offset + 64];
        let bin_step = u16::from_str_radix(
            &bin_step_hex[bin_step_hex.len().saturating_sub(4)..],
            16,
        )
        .unwrap_or(0);
        // Slot 1: LBPair address
        let pair_hex = &clean[struct_offset + 64..struct_offset + 128];
        let pair_addr = format!("0x{}", &pair_hex[24..]);
        // Slot 2: createdByOwner (bool)
        let created_hex = &clean[struct_offset + 128..struct_offset + 192];
        let created_by_owner = created_hex.ends_with('1');
        // Slot 3: ignoredForRouting (bool)
        let ignored_hex = &clean[struct_offset + 192..struct_offset + 256];
        let ignored = ignored_hex.ends_with('1');
        pairs.push((bin_step, pair_addr, created_by_owner, ignored));
    }
    pairs
}

/// pools command: list all LB pairs for a token pair.
pub async fn run(
    token_x: &str,
    token_y: &str,
    chain_id: u64,
) -> anyhow::Result<()> {
    let rpc_url = RPC_URL;
    let addr_x = resolve_token_address(token_x, chain_id);
    let addr_y = resolve_token_address(token_y, chain_id);

    let calldata = build_get_all_lb_pairs_calldata(&addr_x, &addr_y);
    let hex = eth_call(LB_FACTORY, &calldata, rpc_url).await?;

    let pairs_info = parse_lb_pair_information(&hex);
    if pairs_info.is_empty() {
        let out = json!({
            "ok": true,
            "data": {
                "tokenX": token_x.to_uppercase(),
                "tokenY": token_y.to_uppercase(),
                "pools": []
            },
            "message": "No LB pools found for this token pair"
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // For each pair, fetch activeId
    let mut pools = Vec::new();
    for (bin_step, pair_addr, created_by_owner, ignored_for_routing) in &pairs_info {
        let active_id: Option<u32> = {
            let cd = build_get_active_id_calldata();
            match eth_call(pair_addr, &cd, rpc_url).await {
                Ok(h) => {
                    let clean = h.trim_start_matches("0x");
                    u32::from_str_radix(&clean[clean.len().saturating_sub(8)..], 16).ok()
                }
                Err(_) => None,
            }
        };

        pools.push(json!({
            "binStep": bin_step,
            "pairAddress": pair_addr,
            "activeId": active_id,
            "createdByOwner": created_by_owner,
            "ignoredForRouting": ignored_for_routing
        }));
    }

    let out = json!({
        "ok": true,
        "data": {
            "tokenX": token_x.to_uppercase(),
            "tokenXAddress": addr_x,
            "tokenY": token_y.to_uppercase(),
            "tokenYAddress": addr_y,
            "poolCount": pools.len(),
            "pools": pools
        }
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
