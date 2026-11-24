// curve_abi.rs — ABI encoding for CurveRouterNG and pool functions

/// Encode a 20-byte address as 32-byte padded hex (no 0x prefix)
pub fn encode_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Encode a u128 as 32-byte padded hex (no 0x prefix)
pub fn encode_uint256_u128(val: u128) -> String {
    format!("{:064x}", val)
}

/// Encode a u64 as 32-byte padded hex (no 0x prefix)
pub fn encode_uint256_u64(val: u64) -> String {
    format!("{:064x}", val)
}

/// Zero address constant
pub const ZERO_ADDR: &str = "0x0000000000000000000000000000000000000000";

/// Build the ABI calldata for Curve pool get_dy(int128,int128,uint256) -> uint256
/// Selector: keccak256("get_dy(int128,int128,uint256)") = 0x5e0d443f
/// Used for old-style StableSwap pools (3pool, etc.)
pub fn encode_get_dy(i: i64, j: i64, amount: u128) -> String {
    let mut encoded = String::from("0x5e0d443f");
    // int128 encoded as 32-byte two's complement (positive = zero-padded u64)
    if i >= 0 {
        encoded.push_str(&format!("{:064x}", i as u64));
    } else {
        encoded.push_str(&format!("{:064x}", (i as i128) as u128));
    }
    if j >= 0 {
        encoded.push_str(&format!("{:064x}", j as u64));
    } else {
        encoded.push_str(&format!("{:064x}", (j as i128) as u128));
    }
    encoded.push_str(&encode_uint256_u128(amount));
    encoded
}

/// Build the ABI calldata for Curve pool exchange(int128,int128,uint256,uint256)
/// Selector: keccak256("exchange(int128,int128,uint256,uint256)") = 0x3df02124
/// Used for old-style StableSwap pools (3pool, etc.)
pub fn encode_exchange(i: i64, j: i64, amount_in: u128, min_out: u128) -> String {
    let mut encoded = String::from("0x3df02124");
    if i >= 0 {
        encoded.push_str(&format!("{:064x}", i as u64));
    } else {
        encoded.push_str(&format!("{:064x}", (i as i128) as u128));
    }
    if j >= 0 {
        encoded.push_str(&format!("{:064x}", j as u64));
    } else {
        encoded.push_str(&format!("{:064x}", (j as i128) as u128));
    }
    encoded.push_str(&encode_uint256_u128(amount_in));
    encoded.push_str(&encode_uint256_u128(min_out));
    encoded
}

/// Build the ABI calldata for Curve pool exchange(uint256,uint256,uint256,uint256)
/// Selector: keccak256("exchange(uint256,uint256,uint256,uint256)") = 0x5b41b908
/// Used for factory v2 / CryptoSwap pools that use uint256 indices
pub fn encode_exchange_uint256(i: u64, j: u64, amount_in: u128, min_out: u128) -> String {
    let mut encoded = String::from("0x5b41b908");
    encoded.push_str(&encode_uint256_u64(i));
    encoded.push_str(&encode_uint256_u64(j));
    encoded.push_str(&encode_uint256_u128(amount_in));
    encoded.push_str(&encode_uint256_u128(min_out));
    encoded
}

/// Build the ABI calldata for Curve pool get_dy(uint256,uint256,uint256) -> uint256
/// Selector: keccak256("get_dy(uint256,uint256,uint256)") = 0x556d6e9f
/// Used for factory v2 / CryptoSwap pools
pub fn encode_get_dy_uint256(i: u64, j: u64, amount: u128) -> String {
    let mut encoded = String::from("0x556d6e9f");
    encoded.push_str(&encode_uint256_u64(i));
    encoded.push_str(&encode_uint256_u64(j));
    encoded.push_str(&encode_uint256_u128(amount));
    encoded
}

/// Build calldata for add_liquidity(uint256[2],uint256) — 2-coin pool
/// Selector: 0x0b4c7e4d
pub fn encode_add_liquidity_2(amounts: [u128; 2], min_mint: u128) -> String {
    let mut s = String::from("0x0b4c7e4d");
    for &a in &amounts {
        s.push_str(&encode_uint256_u128(a));
    }
    s.push_str(&encode_uint256_u128(min_mint));
    s
}

/// Build calldata for add_liquidity(uint256[3],uint256) — 3-coin pool
/// Selector: 0x4515cef3
pub fn encode_add_liquidity_3(amounts: [u128; 3], min_mint: u128) -> String {
    let mut s = String::from("0x4515cef3");
    for &a in &amounts {
        s.push_str(&encode_uint256_u128(a));
    }
    s.push_str(&encode_uint256_u128(min_mint));
    s
}

/// Build calldata for add_liquidity(uint256[4],uint256) — 4-coin pool
/// Selector: keccak256("add_liquidity(uint256[4],uint256)") -> 0x029b2f34
pub fn encode_add_liquidity_4(amounts: [u128; 4], min_mint: u128) -> String {
    let mut s = String::from("0x029b2f34");
    for &a in &amounts {
        s.push_str(&encode_uint256_u128(a));
    }
    s.push_str(&encode_uint256_u128(min_mint));
    s
}

/// Build calldata for remove_liquidity(uint256,uint256[2]) — 2-coin
/// Selector: 0x5b36389c
pub fn encode_remove_liquidity_2(lp_amount: u128, min_amounts: [u128; 2]) -> String {
    let mut s = String::from("0x5b36389c");
    s.push_str(&encode_uint256_u128(lp_amount));
    for &a in &min_amounts {
        s.push_str(&encode_uint256_u128(a));
    }
    s
}

/// Build calldata for remove_liquidity(uint256,uint256[4]) — 4-coin
/// Selector: keccak256("remove_liquidity(uint256,uint256[4])") = 0x7d49d875
pub fn encode_remove_liquidity_4(lp_amount: u128, min_amounts: [u128; 4]) -> String {
    let mut s = String::from("0x7d49d875");
    s.push_str(&encode_uint256_u128(lp_amount));
    for &a in &min_amounts {
        s.push_str(&encode_uint256_u128(a));
    }
    s
}

/// Build calldata for remove_liquidity(uint256,uint256[3]) — 3-coin
/// Selector: keccak256("remove_liquidity(uint256,uint256[3])") = 0xecb586a5
pub fn encode_remove_liquidity_3(lp_amount: u128, min_amounts: [u128; 3]) -> String {
    let mut s = String::from("0xecb586a5");
    s.push_str(&encode_uint256_u128(lp_amount));
    for &a in &min_amounts {
        s.push_str(&encode_uint256_u128(a));
    }
    s
}

/// Build calldata for remove_liquidity_one_coin(uint256,int128,uint256)
/// Selector: keccak256("remove_liquidity_one_coin(uint256,int128,uint256)") = 0x1a4d01d2
pub fn encode_remove_liquidity_one_coin(lp_amount: u128, coin_index: i64, min_amount: u128) -> String {
    let mut s = String::from("0x1a4d01d2");
    s.push_str(&encode_uint256_u128(lp_amount));
    // int128 encoded as 32-byte two's complement
    if coin_index >= 0 {
        s.push_str(&format!("{:064x}", coin_index as u64));
    } else {
        // two's complement for negative (pad with f's)
        s.push_str(&format!("{:064x}", (coin_index as i128) as u128));
    }
    s.push_str(&encode_uint256_u128(min_amount));
    s
}

/// Build calldata for calc_withdraw_one_coin(uint256,int128) -> uint256
/// Selector: 0xcc2b27d7
pub fn encode_calc_withdraw_one_coin(lp_amount: u128, coin_index: i64) -> String {
    let mut s = String::from("0xcc2b27d7");
    s.push_str(&encode_uint256_u128(lp_amount));
    if coin_index >= 0 {
        s.push_str(&format!("{:064x}", coin_index as u64));
    } else {
        s.push_str(&format!("{:064x}", (coin_index as i128) as u128));
    }
    s
}
