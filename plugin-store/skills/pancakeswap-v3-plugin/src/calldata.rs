/// ABI calldata encoding for PancakeSwap V3 contract calls.
/// Uses alloy-sol-types for type-safe encoding.

use alloy_primitives::{Address, U256};
use alloy_sol_types::{sol, SolCall};
use anyhow::Result;

// ── Function signatures ───────────────────────────────────────────────────────

sol! {
    // ERC-20
    function approve(address spender, uint256 amount) external returns (bool);
    function allowance(address owner, address spender) external view returns (uint256);
    function decimals() external view returns (uint8);
    function symbol() external view returns (string);
    function balanceOf(address account) external view returns (uint256);

    // SmartRouter — exactInputSingle (7-field, NO deadline) — BSC / Base
    struct ExactInputSingleParams {
        address tokenIn;
        address tokenOut;
        uint24  fee;
        address recipient;
        uint256 amountIn;
        uint256 amountOutMinimum;
        uint160 sqrtPriceLimitX96;
    }
    function exactInputSingle(ExactInputSingleParams params) external payable returns (uint256 amountOut);

    // SmartRouter — exactInputSingle (8-field, WITH deadline) — Arbitrum
    struct ExactInputSingleParamsWithDeadline {
        address tokenIn;
        address tokenOut;
        uint24  fee;
        address recipient;
        uint256 deadline;
        uint256 amountIn;
        uint256 amountOutMinimum;
        uint160 sqrtPriceLimitX96;
    }
    function exactInputSingleWithDeadline(ExactInputSingleParamsWithDeadline params) external payable returns (uint256 amountOut);

    // QuoterV2 — quoteExactInputSingle
    struct QuoteExactInputSingleParams {
        address tokenIn;
        address tokenOut;
        uint256 amountIn;
        uint24  fee;
        uint160 sqrtPriceLimitX96;
    }
    function quoteExactInputSingle(QuoteExactInputSingleParams params) external returns (uint256 amountOut, uint160 sqrtPriceX96After, uint32 initializedTicksCrossed, uint256 gasEstimate);

    // NonfungiblePositionManager — mint
    struct MintParams {
        address token0;
        address token1;
        uint24  fee;
        int24   tickLower;
        int24   tickUpper;
        uint256 amount0Desired;
        uint256 amount1Desired;
        uint256 amount0Min;
        uint256 amount1Min;
        address recipient;
        uint256 deadline;
    }
    function mint(MintParams params) external payable returns (uint256 tokenId, uint128 liquidity, uint256 amount0, uint256 amount1);

    // NonfungiblePositionManager — decreaseLiquidity
    struct DecreaseLiquidityParams {
        uint256 tokenId;
        uint128 liquidity;
        uint256 amount0Min;
        uint256 amount1Min;
        uint256 deadline;
    }
    function decreaseLiquidity(DecreaseLiquidityParams params) external payable returns (uint256 amount0, uint256 amount1);

    // NonfungiblePositionManager — collect
    struct CollectParams {
        uint256 tokenId;
        address recipient;
        uint128 amount0Max;
        uint128 amount1Max;
    }
    function collect(CollectParams params) external payable returns (uint256 amount0, uint256 amount1);
}

// ── ERC-20 ────────────────────────────────────────────────────────────────────

pub fn encode_approve(spender: &str, amount: u128) -> Result<String> {
    let call = approveCall {
        spender: spender.parse::<Address>()?,
        amount: U256::from(amount),
    };
    Ok(format!("0x{}", hex::encode(call.abi_encode())))
}

// ── SmartRouter ───────────────────────────────────────────────────────────────

pub fn encode_exact_input_single(
    token_in: &str,
    token_out: &str,
    fee: u32,
    recipient: &str,
    amount_in: u128,
    amount_out_minimum: u128,
    with_deadline: bool,
) -> Result<String> {
    use alloy_primitives::Uint;
    if with_deadline {
        // Arbitrum SmartRouter: selector 0x414bf389, 8-field struct with deadline
        let deadline = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() + 1800; // now + 30 min
        // alloy sol! generates the function as exactInputSingleWithDeadlineCall,
        // but the on-chain selector must be 0x414bf389 (exactInputSingle with deadline struct).
        // We manually encode: selector + ABI-encoded params.
        let params = ExactInputSingleParamsWithDeadline {
            tokenIn: token_in.parse::<Address>()?,
            tokenOut: token_out.parse::<Address>()?,
            fee: Uint::<24, 1>::from(fee),
            recipient: recipient.parse::<Address>()?,
            deadline: U256::from(deadline),
            amountIn: U256::from(amount_in),
            amountOutMinimum: U256::from(amount_out_minimum),
            sqrtPriceLimitX96: alloy_primitives::U160::ZERO,
        };
        use alloy_sol_types::SolValue;
        let encoded = params.abi_encode();
        // Selector for exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))
        let selector = [0x41u8, 0x4b, 0xf3, 0x89];
        let mut calldata = selector.to_vec();
        calldata.extend_from_slice(&encoded);
        Ok(format!("0x{}", hex::encode(calldata)))
    } else {
        let call = exactInputSingleCall {
            params: ExactInputSingleParams {
                tokenIn: token_in.parse::<Address>()?,
                tokenOut: token_out.parse::<Address>()?,
                fee: Uint::<24, 1>::from(fee),
                recipient: recipient.parse::<Address>()?,
                amountIn: U256::from(amount_in),
                amountOutMinimum: U256::from(amount_out_minimum),
                sqrtPriceLimitX96: alloy_primitives::U160::ZERO,
            },
        };
        Ok(format!("0x{}", hex::encode(call.abi_encode())))
    }
}

// ── QuoterV2 ──────────────────────────────────────────────────────────────────

pub fn encode_quote_exact_input_single(
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    fee: u32,
) -> Result<String> {
    use alloy_primitives::Uint;
    let call = quoteExactInputSingleCall {
        params: QuoteExactInputSingleParams {
            tokenIn: token_in.parse::<Address>()?,
            tokenOut: token_out.parse::<Address>()?,
            amountIn: U256::from(amount_in),
            fee: Uint::<24, 1>::from(fee),
            sqrtPriceLimitX96: alloy_primitives::U160::ZERO,
        },
    };
    Ok(format!("0x{}", hex::encode(call.abi_encode())))
}

// ── NonfungiblePositionManager ────────────────────────────────────────────────

pub fn encode_mint(
    token0: &str,
    token1: &str,
    fee: u32,
    tick_lower: i32,
    tick_upper: i32,
    amount0_desired: u128,
    amount1_desired: u128,
    amount0_min: u128,
    amount1_min: u128,
    recipient: &str,
    deadline: u64,
) -> Result<String> {
    use alloy_primitives::{Uint, Signed};
    let call = mintCall {
        params: MintParams {
            token0: token0.parse::<Address>()?,
            token1: token1.parse::<Address>()?,
            fee: Uint::<24, 1>::from(fee),
            tickLower: Signed::<24, 1>::try_from(tick_lower as i64)
                .map_err(|_| anyhow::anyhow!("tickLower out of int24 range: {}", tick_lower))?,
            tickUpper: Signed::<24, 1>::try_from(tick_upper as i64)
                .map_err(|_| anyhow::anyhow!("tickUpper out of int24 range: {}", tick_upper))?,
            amount0Desired: U256::from(amount0_desired),
            amount1Desired: U256::from(amount1_desired),
            amount0Min: U256::from(amount0_min),
            amount1Min: U256::from(amount1_min),
            recipient: recipient.parse::<Address>()?,
            deadline: U256::from(deadline),
        },
    };
    Ok(format!("0x{}", hex::encode(call.abi_encode())))
}

pub fn encode_decrease_liquidity(
    token_id: u128,
    liquidity: u128,
    amount0_min: u128,
    amount1_min: u128,
    deadline: u64,
) -> Result<String> {
    let call = decreaseLiquidityCall {
        params: DecreaseLiquidityParams {
            tokenId: U256::from(token_id),
            liquidity: liquidity as u128,
            amount0Min: U256::from(amount0_min),
            amount1Min: U256::from(amount1_min),
            deadline: U256::from(deadline),
        },
    };
    Ok(format!("0x{}", hex::encode(call.abi_encode())))
}

pub fn encode_collect(
    token_id: u128,
    recipient: &str,
) -> Result<String> {
    let call = collectCall {
        params: CollectParams {
            tokenId: U256::from(token_id),
            recipient: recipient.parse::<Address>()?,
            amount0Max: u128::MAX,
            amount1Max: u128::MAX,
        },
    };
    Ok(format!("0x{}", hex::encode(call.abi_encode())))
}

// ── Helper: sort token addresses (token0 < token1) ────────────────────────────

/// Returns (token0, token1) sorted such that token0 < token1 numerically.
pub fn sort_tokens<'a>(a: &'a str, b: &'a str) -> Result<(&'a str, &'a str)> {
    let addr_a: Address = a.parse()?;
    let addr_b: Address = b.parse()?;
    if addr_a < addr_b {
        Ok((a, b))
    } else {
        Ok((b, a))
    }
}
