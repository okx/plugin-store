use alloy_primitives::{Address, U256};
use alloy_sol_types::{sol, SolCall};
use anyhow::Context;

sol! {
    function supply(
        address asset,
        uint256 amount,
        address onBehalfOf,
        uint16 referralCode
    ) external;

    function withdraw(
        address asset,
        uint256 amount,
        address to
    ) external returns (uint256);

    function borrow(
        address asset,
        uint256 amount,
        uint256 interestRateMode,
        uint16 referralCode,
        address onBehalfOf
    ) external;

    function repay(
        address asset,
        uint256 amount,
        uint256 interestRateMode,
        address onBehalfOf
    ) external returns (uint256);

    function setUserUseReserveAsCollateral(
        address asset,
        bool useAsCollateral
    ) external;

    function approve(
        address spender,
        uint256 amount
    ) external returns (bool);
}

fn parse_address(addr: &str) -> anyhow::Result<Address> {
    addr.parse::<Address>()
        .with_context(|| format!("Invalid address: {}", addr))
}

/// Encode Pool.supply() calldata.
/// referralCode is always 0.
pub fn encode_supply(asset: &str, amount: u128, on_behalf_of: &str) -> anyhow::Result<String> {
    let call = supplyCall {
        asset: parse_address(asset)?,
        amount: U256::from(amount),
        onBehalfOf: parse_address(on_behalf_of)?,
        referralCode: crate::config::REFERRAL_CODE,
    };
    let encoded = call.abi_encode();
    Ok(format!("0x{}", hex::encode(encoded)))
}

/// Encode Pool.withdraw() calldata.
/// Pass u128::MAX for full withdrawal (maps to type(uint256).max).
pub fn encode_withdraw(asset: &str, amount: u128, to: &str) -> anyhow::Result<String> {
    let amount_u256 = if amount == u128::MAX {
        U256::MAX
    } else {
        U256::from(amount)
    };
    let call = withdrawCall {
        asset: parse_address(asset)?,
        amount: amount_u256,
        to: parse_address(to)?,
    };
    let encoded = call.abi_encode();
    Ok(format!("0x{}", hex::encode(encoded)))
}

/// Encode Pool.borrow() calldata.
/// interestRateMode is always 2 (variable) — stable (1) is deprecated in V3.1+
pub fn encode_borrow(
    asset: &str,
    amount: u128,
    on_behalf_of: &str,
) -> anyhow::Result<String> {
    let call = borrowCall {
        asset: parse_address(asset)?,
        amount: U256::from(amount),
        interestRateMode: U256::from(crate::config::INTEREST_RATE_MODE_VARIABLE),
        referralCode: crate::config::REFERRAL_CODE,
        onBehalfOf: parse_address(on_behalf_of)?,
    };
    let encoded = call.abi_encode();
    Ok(format!("0x{}", hex::encode(encoded)))
}

/// Encode Pool.repay() calldata.
/// Pass u128::MAX for full repay (maps to type(uint256).max in Solidity).
pub fn encode_repay(
    asset: &str,
    amount: u128,
    on_behalf_of: &str,
) -> anyhow::Result<String> {
    let amount_u256 = if amount == u128::MAX {
        U256::MAX
    } else {
        U256::from(amount)
    };
    let call = repayCall {
        asset: parse_address(asset)?,
        amount: amount_u256,
        interestRateMode: U256::from(crate::config::INTEREST_RATE_MODE_VARIABLE),
        onBehalfOf: parse_address(on_behalf_of)?,
    };
    let encoded = call.abi_encode();
    Ok(format!("0x{}", hex::encode(encoded)))
}

/// Encode ERC-20 approve() calldata.
/// Pass u128::MAX for unlimited approval (type(uint256).max).
pub fn encode_erc20_approve(spender: &str, amount: u128) -> anyhow::Result<String> {
    let amount_u256 = if amount == u128::MAX {
        U256::MAX
    } else {
        U256::from(amount)
    };
    let call = approveCall {
        spender: parse_address(spender)?,
        amount: amount_u256,
    };
    let encoded = call.abi_encode();
    Ok(format!("0x{}", hex::encode(encoded)))
}
