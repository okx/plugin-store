use serde_json::json;

use crate::config::{resolve_token_address, pad_address, LB_QUOTER, RPC_URL};
use crate::rpc::eth_call;

/// Version enum mapping (from ILBRouter.Version)
pub fn version_name(v: u8) -> &'static str {
    match v {
        0 => "V1",
        1 => "V2",
        2 => "V2_1",
        3 => "V2_2",
        _ => "Unknown",
    }
}

/// Decode a hex string into u128.
/// Read a 32-byte word at given byte offset in hex string (no 0x), return as usize.
fn read_word_usize(data: &str, byte_offset: usize) -> usize {
    let hex_off = byte_offset * 2;
    if data.len() < hex_off + 64 {
        return 0;
    }
    usize::from_str_radix(&data[hex_off..hex_off + 64], 16).unwrap_or(0)
}

/// Read a 32-byte word at given byte offset, return as u128 (lower 16 bytes).
fn read_word_u128(data: &str, byte_offset: usize) -> u128 {
    let hex_off = byte_offset * 2;
    if data.len() < hex_off + 64 {
        return 0;
    }
    // take last 32 hex chars (16 bytes) for u128
    u128::from_str_radix(&data[hex_off + 32..hex_off + 64], 16).unwrap_or(0)
}

/// Decode a dynamic uint128[] starting at absolute byte offset `arr_start` in data.
/// (arr_start points to the length word, followed by elements)
fn decode_u128_array_at(data: &str, arr_start: usize) -> Vec<u128> {
    let len = read_word_usize(data, arr_start);
    if len > 100 { return vec![]; } // sanity cap
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let elem_start = arr_start + 32 + i * 32;
        out.push(read_word_u128(data, elem_start));
    }
    out
}

/// Decode a dynamic uint256[] (as u128) at absolute byte offset.
fn decode_u256_array_at(data: &str, arr_start: usize) -> Vec<u128> {
    decode_u128_array_at(data, arr_start)
}

/// Decode a dynamic address[] at absolute byte offset.
fn decode_address_array_at(data: &str, arr_start: usize) -> Vec<String> {
    let len = read_word_usize(data, arr_start);
    if len > 100 { return vec![]; } // sanity cap
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let elem_start = arr_start + 32 + i * 32;
        let hex_off = elem_start * 2;
        if data.len() < hex_off + 64 {
            break;
        }
        let elem_hex = &data[hex_off..hex_off + 64];
        out.push(format!("0x{}", &elem_hex[24..]));
    }
    out
}

/// Build findBestPathFromAmountIn calldata.
/// Selector: 0x0f902a40
/// Signature: findBestPathFromAmountIn(address[],uint128)
pub fn build_find_best_path_in_calldata(route: &[&str], amount_in: u128) -> String {
    // ABI encoding for (address[], uint128):
    // The function has two args: dynamic (address[]) and static (uint128)
    // Layout:
    //   [0x00] offset to address[] = 0x40 (2 words = 64 bytes)
    //   [0x20] amountIn (uint128 padded to 32 bytes)
    //   [0x40] address[].length
    //   [0x60+] address elements
    let offset_to_array = format!("{:0>64x}", 0x40usize);
    let amount_hex = format!("{:0>64x}", amount_in);
    let array_len = format!("{:0>64x}", route.len());
    let mut elements = String::new();
    for addr in route {
        elements.push_str(&pad_address(addr));
    }
    format!(
        "0x0f902a40{}{}{}{}",
        offset_to_array, amount_hex, array_len, elements
    )
}

/// Parsed quote result.
#[derive(Debug)]
pub struct QuoteResult {
    pub route: Vec<String>,
    pub pairs: Vec<String>,
    pub bin_steps: Vec<u128>,
    pub versions: Vec<u8>,
    pub amounts: Vec<u128>,
    pub fees: Vec<u128>,
}

impl QuoteResult {
    pub fn amount_in(&self) -> u128 {
        self.amounts.first().copied().unwrap_or(0)
    }
    pub fn amount_out(&self) -> u128 {
        self.amounts.last().copied().unwrap_or(0)
    }
}

/// Call LBQuoter.findBestPathFromAmountIn and parse the result.
pub async fn find_best_path_in(
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    rpc_url: &str,
) -> anyhow::Result<QuoteResult> {
    let route = vec![token_in, token_out];
    let calldata = build_find_best_path_in_calldata(&route, amount_in);
    let hex = eth_call(LB_QUOTER, &calldata, rpc_url).await?;
    parse_quote_response(&hex)
}

/// Parse the Quote struct returned by findBestPathFromAmountIn.
///
/// The ABI-encoded response has a 1-level outer tuple wrapper:
/// - Word 0 (byte 0x00): outer struct offset = 0x20
/// - From byte 0x20: 7 pointer words (one per array field in Quote)
/// - Each pointer is RELATIVE TO the struct start (byte 0x20)
///
/// Quote struct field order:
///   [0] route: address[]
///   [1] pairs: address[]
///   [2] binSteps: uint256[]
///   [3] versions: uint8[] (ABI-encoded as uint256 per element)
///   [4] amounts: uint128[]
///   [5] virtualAmountsWithoutSlippage: uint128[]
///   [6] fees: uint128[]
pub fn parse_quote_response(hex: &str) -> anyhow::Result<QuoteResult> {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 128 {
        anyhow::bail!("Empty or too-short quote response (got {} hex chars)", clean.len());
    }

    // Word 0: outer offset (should be 0x20 = 32 bytes)
    let outer_offset = read_word_usize(clean, 0); // typically 32

    // Struct header starts at outer_offset (in bytes).
    // It has 7 pointer slots (7 * 32 = 224 bytes), then array data.
    // Pointers are relative to the struct start.
    let struct_base = outer_offset;

    // Read 7 pointers (each is a byte offset relative to struct_base)
    let ptr_route       = read_word_usize(clean, struct_base + 0 * 32);
    let ptr_pairs       = read_word_usize(clean, struct_base + 1 * 32);
    let ptr_bin_steps   = read_word_usize(clean, struct_base + 2 * 32);
    let ptr_versions    = read_word_usize(clean, struct_base + 3 * 32);
    let ptr_amounts     = read_word_usize(clean, struct_base + 4 * 32);
    // slot 5 = virtualAmountsWithoutSlippage (skip)
    let ptr_fees        = read_word_usize(clean, struct_base + 6 * 32);

    // Absolute byte offsets for each array
    let abs_route     = struct_base + ptr_route;
    let abs_pairs     = struct_base + ptr_pairs;
    let abs_bin_steps = struct_base + ptr_bin_steps;
    let abs_versions  = struct_base + ptr_versions;
    let abs_amounts   = struct_base + ptr_amounts;
    let abs_fees      = struct_base + ptr_fees;

    let route     = decode_address_array_at(clean, abs_route);
    let pairs     = decode_address_array_at(clean, abs_pairs);
    let bin_steps = decode_u256_array_at(clean, abs_bin_steps);
    let version_vals = decode_u256_array_at(clean, abs_versions);
    let versions: Vec<u8> = version_vals.iter().map(|v| *v as u8).collect();
    let amounts   = decode_u128_array_at(clean, abs_amounts);
    let fees      = decode_u128_array_at(clean, abs_fees);

    Ok(QuoteResult {
        route,
        pairs,
        bin_steps,
        versions,
        amounts,
        fees,
    })
}

/// quote command: get best quote for token swap on Trader Joe LB.
pub async fn run(
    token_in: &str,
    token_out: &str,
    amount: f64,
    decimals_in: u8,
    chain_id: u64,
) -> anyhow::Result<()> {
    let rpc_url = RPC_URL;
    let addr_in = resolve_token_address(token_in, chain_id);
    let addr_out = resolve_token_address(token_out, chain_id);

    // Convert human amount to raw atomic units
    let amount_in_raw = (amount * 10f64.powi(decimals_in as i32)) as u128;

    let q = find_best_path_in(&addr_in, &addr_out, amount_in_raw, rpc_url).await?;

    if q.amount_out() == 0 {
        anyhow::bail!("No route found or quote is zero. Pair may not exist on Trader Joe.");
    }

    let fee_bps: f64 = if let Some(&f) = q.fees.first() {
        // fees are in 1e18 units representing the fee fraction
        // fee fraction = fees[0] / 1e18 → percentage = * 100 → bps = * 10000
        f as f64 / 1e14 // convert to basis points
    } else {
        0.0
    };

    let result = json!({
        "ok": true,
        "data": {
            "tokenIn": token_in.to_uppercase(),
            "tokenOut": token_out.to_uppercase(),
            "tokenInAddress": addr_in,
            "tokenOutAddress": addr_out,
            "amountIn": amount,
            "amountInRaw": q.amount_in().to_string(),
            "amountOut": q.amount_out() as f64 / 10f64.powi(decimals_out_for_token(token_out, chain_id) as i32),
            "amountOutRaw": q.amount_out().to_string(),
            "binStep": q.bin_steps.first().copied().unwrap_or(0),
            "version": q.versions.first().map(|v| version_name(*v)).unwrap_or("unknown"),
            "pair": q.pairs.first().cloned().unwrap_or_default(),
            "feeBps": fee_bps,
            "route": q.route
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Get decimals for well-known tokens.
fn decimals_out_for_token(symbol: &str, _chain_id: u64) -> u8 {
    match symbol.to_uppercase().as_str() {
        "USDT" | "USDT0" | "USD₮0" | "USDC" => 6,
        "WBTC" => 8,
        "ETH" | "WETH" | "ARB" => 18,
        _ => 18, // default
    }
}
