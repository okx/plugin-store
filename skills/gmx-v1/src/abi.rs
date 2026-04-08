/// ABI encoding helpers for GMX V1 contracts.
/// All functions return hex-encoded calldata (0x-prefixed).

/// Pad an address to 32 bytes (ABI encoding).
fn pad_address(addr: &str) -> anyhow::Result<String> {
    let clean = addr.trim_start_matches("0x");
    if clean.len() != 40 {
        anyhow::bail!("Invalid address: {}", addr);
    }
    Ok(format!("{:0>64}", clean))
}

/// Pad a u128 to 32 bytes.
fn pad_u128(v: u128) -> String {
    format!("{:064x}", v)
}

/// Pad a u64 to 32 bytes.
fn pad_u64(v: u64) -> String {
    format!("{:064x}", v)
}

/// Pad a bool to 32 bytes.
fn pad_bool(v: bool) -> String {
    format!("{:064x}", if v { 1u64 } else { 0u64 })
}

/// Pad a bytes32 to 32 bytes (already 32 bytes, just zero-pad if shorter).
fn pad_bytes32(v: &str) -> String {
    let clean = v.trim_start_matches("0x");
    format!("{:0<64}", clean)
}

/// Encode a dynamic address array for ABI.
/// Returns the offset word + length word + padded elements.
fn encode_address_array(arr: &[&str], base_offset: usize) -> anyhow::Result<(String, String)> {
    // Returns (offset_word, data_block)
    let offset = format!("{:064x}", base_offset);
    let mut data = format!("{:064x}", arr.len());
    for addr in arr {
        data.push_str(&pad_address(addr)?);
    }
    Ok((offset, data))
}

/// Router.swap(address[] path, uint256 amountIn, uint256 minOut, address receiver)
/// Selector: 0x6023e966
pub fn encode_swap(
    path: &[&str],
    amount_in: u128,
    min_out: u128,
    receiver: &str,
) -> anyhow::Result<String> {
    let selector = "6023e966";
    // path is dynamic — offset is 4 static params * 32 = 128? No:
    // Static layout: [offset_path(32), amountIn(32), minOut(32), receiver(32)] = 4 * 32 = 128 bytes
    // Dynamic data starts at offset 128 = 0x80
    let (offset_path, path_data) = encode_address_array(path, 128)?;
    let calldata = format!(
        "0x{}{}{}{}{}{}",
        selector,
        offset_path,
        pad_u128(amount_in),
        pad_u128(min_out),
        pad_address(receiver)?,
        path_data
    );
    Ok(calldata)
}

/// Router.swapETHToTokens(address[] path, uint256 minOut, address receiver)
/// Selector: 0xabe68eaa
/// ETH value passed via --amt
#[allow(dead_code)]
pub fn encode_swap_eth_to_tokens(
    path: &[&str],
    min_out: u128,
    receiver: &str,
) -> anyhow::Result<String> {
    let selector = "abe68eaa";
    // Static: [offset_path(32), minOut(32), receiver(32)] = 3 * 32 = 96 bytes
    let (offset_path, path_data) = encode_address_array(path, 96)?;
    let calldata = format!(
        "0x{}{}{}{}{}",
        selector,
        offset_path,
        pad_u128(min_out),
        pad_address(receiver)?,
        path_data
    );
    Ok(calldata)
}

/// Router.swapTokensToETH(address[] path, uint256 amountIn, uint256 minOut, address payable receiver)
/// Selector: 0x2d4ba6a7
#[allow(dead_code)]
pub fn encode_swap_tokens_to_eth(
    path: &[&str],
    amount_in: u128,
    min_out: u128,
    receiver: &str,
) -> anyhow::Result<String> {
    let selector = "2d4ba6a7";
    // Static: [offset_path(32), amountIn(32), minOut(32), receiver(32)] = 128
    let (offset_path, path_data) = encode_address_array(path, 128)?;
    let calldata = format!(
        "0x{}{}{}{}{}{}",
        selector,
        offset_path,
        pad_u128(amount_in),
        pad_u128(min_out),
        pad_address(receiver)?,
        path_data
    );
    Ok(calldata)
}

/// RewardRouter.mintAndStakeGlp(address token, uint256 amount, uint256 minUsdg, uint256 minGlp)
/// Selector: 0x364e2311
pub fn encode_mint_and_stake_glp(
    token: &str,
    amount: u128,
    min_usdg: u128,
    min_glp: u128,
) -> anyhow::Result<String> {
    let selector = "364e2311";
    Ok(format!(
        "0x{}{}{}{}{}",
        selector,
        pad_address(token)?,
        pad_u128(amount),
        pad_u128(min_usdg),
        pad_u128(min_glp)
    ))
}

/// RewardRouter.unstakeAndRedeemGlp(address tokenOut, uint256 glpAmount, uint256 minOut, address receiver)
/// Selector: 0x0f3aa554
pub fn encode_unstake_and_redeem_glp(
    token_out: &str,
    glp_amount: u128,
    min_out: u128,
    receiver: &str,
) -> anyhow::Result<String> {
    let selector = "0f3aa554";
    Ok(format!(
        "0x{}{}{}{}{}",
        selector,
        pad_address(token_out)?,
        pad_u128(glp_amount),
        pad_u128(min_out),
        pad_address(receiver)?
    ))
}

/// PositionRouter.createIncreasePosition(
///   address[] path, address indexToken, uint256 amountIn, uint256 minOut,
///   uint256 sizeDelta, bool isLong, uint256 acceptablePrice,
///   uint256 executionFee, bytes32 referralCode, address callbackTarget
/// )
/// Selector: 0xf2ae372f
/// ETH value = executionFee (via --amt)
pub fn encode_create_increase_position(
    path: &[&str],
    index_token: &str,
    amount_in: u128,
    min_out: u128,
    size_delta: u128,  // in USD * 10^30
    is_long: bool,
    acceptable_price: u128,
    execution_fee: u64,
) -> anyhow::Result<String> {
    let selector = "f2ae372f";
    // Static params (9 fixed-size + 1 dynamic = 10 params):
    // [offset_path(32), indexToken(32), amountIn(32), minOut(32), sizeDelta(32),
    //  isLong(32), acceptablePrice(32), executionFee(32), referralCode(32), callbackTarget(32)]
    // = 10 * 32 = 320 bytes offset for path data
    let (offset_path, path_data) = encode_address_array(path, 320)?;
    let calldata = format!(
        "0x{}{}{}{}{}{}{}{}{}{}{}{}",
        selector,
        offset_path,
        pad_address(index_token)?,
        pad_u128(amount_in),
        pad_u128(min_out),
        pad_u128(size_delta),
        pad_bool(is_long),
        pad_u128(acceptable_price),
        pad_u64(execution_fee),
        pad_bytes32("0000000000000000000000000000000000000000000000000000000000000000"), // referralCode
        pad_address("0x0000000000000000000000000000000000000000")?, // callbackTarget
        path_data
    );
    Ok(calldata)
}

/// PositionRouter.createDecreasePosition(
///   address[] path, address indexToken, uint256 collateralDelta,
///   uint256 sizeDelta, bool isLong, address receiver,
///   uint256 acceptablePrice, uint256 minOut,
///   uint256 executionFee, bool withdrawETH, address callbackTarget
/// )
/// Selector: 0x7be7d141
/// ETH value = executionFee (via --amt)
pub fn encode_create_decrease_position(
    path: &[&str],
    index_token: &str,
    collateral_delta: u128,
    size_delta: u128,
    is_long: bool,
    receiver: &str,
    acceptable_price: u128,
    min_out: u128,
    execution_fee: u64,
    withdraw_eth: bool,
) -> anyhow::Result<String> {
    let selector = "7be7d141";
    // Static params (10 fixed-size + 1 dynamic = 11):
    // [offset_path(32), indexToken(32), collateralDelta(32), sizeDelta(32), isLong(32),
    //  receiver(32), acceptablePrice(32), minOut(32), executionFee(32), withdrawETH(32), callbackTarget(32)]
    // = 11 * 32 = 352 bytes
    let (offset_path, path_data) = encode_address_array(path, 352)?;
    let calldata = format!(
        "0x{}{}{}{}{}{}{}{}{}{}{}{}{}",
        selector,
        offset_path,
        pad_address(index_token)?,
        pad_u128(collateral_delta),
        pad_u128(size_delta),
        pad_bool(is_long),
        pad_address(receiver)?,
        pad_u128(acceptable_price),
        pad_u128(min_out),
        pad_u64(execution_fee),
        pad_bool(withdraw_eth),
        pad_address("0x0000000000000000000000000000000000000000")?, // callbackTarget
        path_data
    );
    Ok(calldata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_approve() {
        // approve(address,uint256) selector = 0x095ea7b3
        let result = crate::onchainos::encode_approve(
            "0xaBBc5F99639c9B6bCb58544ddf04CF3C176D2B00",
            u128::MAX,
        ).unwrap();
        assert!(result.starts_with("0x095ea7b3"));
    }

    #[test]
    fn test_encode_mint_and_stake_glp() {
        let result = encode_mint_and_stake_glp(
            "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
            5_000_000,
            0,
            0,
        ).unwrap();
        assert!(result.starts_with("0x364e2311"));
    }

    #[test]
    fn test_encode_swap() {
        let path = [
            "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
            "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
        ];
        let result = encode_swap(&path, 10_000_000, 0, "0x87fb0647faabea33113eaf1d80d67acb1c491b90").unwrap();
        assert!(result.starts_with("0x6023e966"));
    }
}
