/// ABI calldata encoding for Morpho Blue and MetaMorpho contracts.
///
/// All Morpho Blue functions take a MarketParams struct as the first argument.
/// ABI encoding for structs is the same as a tuple — each field is a 32-byte slot.

#[derive(Debug, Clone)]
pub struct MarketParamsData {
    pub loan_token: String,
    pub collateral_token: String,
    pub oracle: String,
    pub irm: String,
    pub lltv: u128,
}

/// Encode a 20-byte address as a 32-byte hex slot (left-zero-padded, no 0x prefix).
fn encode_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Encode a u128 as a 32-byte hex slot (no 0x prefix).
fn encode_u256(val: u128) -> String {
    format!("{:064x}", val)
}

/// Encode the MarketParams struct as 5 × 32-byte slots.
fn encode_market_params(mp: &MarketParamsData) -> String {
    format!(
        "{}{}{}{}{}",
        encode_address(&mp.loan_token),
        encode_address(&mp.collateral_token),
        encode_address(&mp.oracle),
        encode_address(&mp.irm),
        encode_u256(mp.lltv),
    )
}

/// supplyCollateral(marketParams, assets, onBehalf, data)
/// Selector: 0x238d6579
/// Layout: selector(4) + marketParams(5×32) + assets(32) + onBehalf(32) + data_offset(32) + data_len(32)
/// data is empty bytes; offset = 0xc0 = 192 (number of bytes after selector before data field)
pub fn encode_supply_collateral(mp: &MarketParamsData, assets: u128, on_behalf: &str) -> String {
    // Offset for bytes data: after selector we have 7 fixed 32-byte words before the dynamic part
    // = 5 (marketParams) + 1 (assets) + 1 (onBehalf) = 7 words = 7 * 32 = 224 = 0xe0
    let data_offset = format!("{:064x}", 7u128 * 32u128);
    format!(
        "0x238d6579{}{}{}{}{}",
        encode_market_params(mp),
        encode_u256(assets),
        encode_address(on_behalf),
        data_offset,
        encode_u256(0), // data length = 0
    )
}

/// withdrawCollateral(marketParams, assets, onBehalf, receiver)
/// Selector: 0x8720316d
/// Layout: selector(4) + marketParams(5×32) + assets(32) + onBehalf(32) + receiver(32)
pub fn encode_withdraw_collateral(
    mp: &MarketParamsData,
    assets: u128,
    on_behalf: &str,
    receiver: &str,
) -> String {
    format!(
        "0x8720316d{}{}{}{}",
        encode_market_params(mp),
        encode_u256(assets),
        encode_address(on_behalf),
        encode_address(receiver),
    )
}

/// borrow(marketParams, assets, shares, onBehalf, receiver)
/// Selector: 0x50d8cd4b
/// Layout: selector(4) + marketParams(5×32) + assets(32) + shares(32) + onBehalf(32) + receiver(32)
pub fn encode_borrow(
    mp: &MarketParamsData,
    assets: u128,
    shares: u128,
    on_behalf: &str,
    receiver: &str,
) -> String {
    format!(
        "0x50d8cd4b{}{}{}{}{}",
        encode_market_params(mp),
        encode_u256(assets),
        encode_u256(shares),
        encode_address(on_behalf),
        encode_address(receiver),
    )
}

/// repay(marketParams, assets, shares, onBehalf, data)
/// Selector: 0x20b76e81
/// Layout: selector(4) + marketParams(5×32) + assets(32) + shares(32) + onBehalf(32) + data_offset(32) + data_len(32)
pub fn encode_repay(
    mp: &MarketParamsData,
    assets: u128,
    shares: u128,
    on_behalf: &str,
) -> String {
    // data_offset: 5 (marketParams) + 1 (assets) + 1 (shares) + 1 (onBehalf) = 8 words = 256 = 0x100
    let data_offset = format!("{:064x}", 8u128 * 32u128);
    format!(
        "0x20b76e81{}{}{}{}{}{}",
        encode_market_params(mp),
        encode_u256(assets),
        encode_u256(shares),
        encode_address(on_behalf),
        data_offset,
        encode_u256(0), // data length = 0
    )
}

/// supply(marketParams, assets, shares, onBehalf, data) — Morpho Blue supply lending
/// Selector: 0xa99aad89
/// Layout: selector(4) + marketParams(5×32) + assets(32) + shares(32) + onBehalf(32) + data_offset(32) + data_len(32)
pub fn encode_blue_supply(
    mp: &MarketParamsData,
    assets: u128,
    shares: u128,
    on_behalf: &str,
) -> String {
    let data_offset = format!("{:064x}", 8u128 * 32u128);
    format!(
        "0xa99aad89{}{}{}{}{}{}",
        encode_market_params(mp),
        encode_u256(assets),
        encode_u256(shares),
        encode_address(on_behalf),
        data_offset,
        encode_u256(0),
    )
}

/// ERC-4626 deposit(assets, receiver)
/// Selector: 0x6e553f65
pub fn encode_vault_deposit(assets: u128, receiver: &str) -> String {
    format!(
        "0x6e553f65{}{}",
        encode_u256(assets),
        encode_address(receiver),
    )
}

/// ERC-4626 withdraw(assets, receiver, owner)
/// Selector: 0xb460af94
pub fn encode_vault_withdraw(assets: u128, receiver: &str, owner: &str) -> String {
    format!(
        "0xb460af94{}{}{}",
        encode_u256(assets),
        encode_address(receiver),
        encode_address(owner),
    )
}

/// ERC-4626 redeem(shares, receiver, owner)
/// Selector: 0xba087652
pub fn encode_vault_redeem(shares: u128, receiver: &str, owner: &str) -> String {
    format!(
        "0xba087652{}{}{}",
        encode_u256(shares),
        encode_address(receiver),
        encode_address(owner),
    )
}

/// ERC-20 approve(spender, amount)
/// Selector: 0x095ea7b3
pub fn encode_approve(spender: &str, amount: u128) -> String {
    let spender_clean = spender.trim_start_matches("0x");
    format!(
        "0x095ea7b3{:0>64}{:064x}",
        spender_clean,
        amount,
    )
}

/// Merkl claim(users[], tokens[], claimable[], proofs[][])
/// Selector: 0x2e7ba6ef
/// This is a complex ABI-encoding with dynamic arrays.
pub fn encode_merkl_claim(
    user: &str,
    tokens: &[String],
    claimable: &[String],
    proofs: &[Vec<String>],
) -> String {
    // ABI encode: (address[], address[], uint256[], bytes32[][])
    // Single user, so users = [user]
    // We build the calldata manually.
    let mut out = String::from("0x2e7ba6ef");

    // All four params are dynamic arrays — head is 4 offsets (each 32 bytes = 128 bytes of head)
    // Offset for users array: 128 (0x80)
    // Offset for tokens array: 128 + 32 + 32*1 = 192 (0xc0)
    // Offset for claimable array: 192 + 32 + 32*len_tokens
    // Offset for proofs array: varies

    let n = tokens.len();
    // We encode:
    // [0x80] offset users
    // [0xa0+n*32] offset tokens
    // ... complex. Let's build it with a helper.

    let mut body = Vec::<String>::new(); // each element is a 32-byte hex chunk (no 0x)

    // users array (length 1, single user)
    let users_slot_start = 4; // index in body where users array data starts (after 4 offset slots)
    // Offsets are in bytes from start of ABI data (after selector).
    // 4 offset slots = 128 bytes
    // users array starts at byte 128
    let users_offset = 128usize; // 4 * 32

    // tokens array starts after users: 128 + 32 (len) + 32 (1 addr) = 192
    let tokens_offset = users_offset + 32 + 32 * 1;

    // claimable array starts after tokens
    let claimable_offset = tokens_offset + 32 + 32 * n;

    // proofs array starts after claimable
    let proofs_offset = claimable_offset + 32 + 32 * n;

    // 4 head offsets
    out.push_str(&format!("{:064x}", users_offset));
    out.push_str(&format!("{:064x}", tokens_offset));
    out.push_str(&format!("{:064x}", claimable_offset));
    out.push_str(&format!("{:064x}", proofs_offset));

    // users array: length=1, data=[user]
    out.push_str(&format!("{:064x}", 1usize)); // length
    out.push_str(&encode_address(user));

    // tokens array
    out.push_str(&format!("{:064x}", n));
    for t in tokens {
        out.push_str(&encode_address(t));
    }

    // claimable array
    out.push_str(&format!("{:064x}", n));
    for c in claimable {
        let val: u128 = c.parse().unwrap_or(0);
        out.push_str(&encode_u256(val));
    }

    // proofs array: bytes32[][] — array of arrays
    // Head: n offsets (relative to start of this outer array's data)
    // Each inner array: length + elements
    let inner_offsets_bytes = n * 32; // n offset slots for inner arrays
    let mut inner_offset = inner_offsets_bytes;
    out.push_str(&format!("{:064x}", n)); // outer array length

    // Compute inner offsets first
    let mut inner_offset_vals = Vec::new();
    for proof in proofs {
        inner_offset_vals.push(inner_offset);
        inner_offset += 32 + 32 * proof.len(); // length word + elements
    }
    for ov in &inner_offset_vals {
        out.push_str(&format!("{:064x}", ov));
    }
    // Then emit inner array data
    for proof in proofs {
        out.push_str(&format!("{:064x}", proof.len()));
        for p in proof {
            let clean = p.trim_start_matches("0x");
            out.push_str(&format!("{:0>64}", clean));
        }
    }

    let _ = body; // suppress warning
    out
}

/// Parse human-readable amount to raw token amount given decimals.
pub fn parse_amount(amount_str: &str, decimals: u8) -> anyhow::Result<u128> {
    // Handle decimal notation like "1.5"
    let parts: Vec<&str> = amount_str.split('.').collect();
    match parts.len() {
        1 => {
            let whole: u128 = parts[0].parse()?;
            Ok(whole * 10u128.pow(decimals as u32))
        }
        2 => {
            let whole: u128 = parts[0].parse()?;
            let frac_str = parts[1];
            let frac_len = frac_str.len() as u32;
            let frac: u128 = frac_str.parse()?;
            if frac_len > decimals as u32 {
                anyhow::bail!("Too many decimal places: {} (max {})", frac_len, decimals);
            }
            let frac_scaled = frac * 10u128.pow(decimals as u32 - frac_len);
            Ok(whole * 10u128.pow(decimals as u32) + frac_scaled)
        }
        _ => anyhow::bail!("Invalid amount: {}", amount_str),
    }
}

/// Format raw token amount to human-readable with given decimals.
pub fn format_amount(raw: u128, decimals: u8) -> String {
    if decimals == 0 {
        return raw.to_string();
    }
    let divisor = 10u128.pow(decimals as u32);
    let whole = raw / divisor;
    let frac = raw % divisor;
    if frac == 0 {
        format!("{}", whole)
    } else {
        let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
        let frac_trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, frac_trimmed)
    }
}
