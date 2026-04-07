use tokio::time::{sleep, Duration};

use crate::config::{
    resolve_token_address, is_native_bnb, apply_slippage, deadline, pad_address, pad_u256,
    build_approve_calldata, ROUTER_V2, BSC_RPC, CHAIN_ID, FACTORY_V2,
};
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};
use crate::rpc::{get_allowance, get_balance, factory_get_pair, get_reserves, get_token0, get_total_supply};

/// Remove liquidity from a PancakeSwap V2 pool.
///
/// Handles two variants:
///   1. token + token  → removeLiquidity
///   2. token + BNB    → removeLiquidityETH
///
/// If liquidity is 0 (all), uses the full LP token balance.
pub async fn run(
    token_a: &str,
    token_b: &str,
    liquidity: Option<u128>, // None = use full balance
    dry_run: bool,
) -> anyhow::Result<()> {
    let chain_id = CHAIN_ID;
    let rpc = BSC_RPC;
    let router = ROUTER_V2;
    let factory = FACTORY_V2;

    let b_is_bnb = is_native_bnb(token_b);
    let a_is_bnb = is_native_bnb(token_a);

    if a_is_bnb && b_is_bnb {
        anyhow::bail!("Cannot remove BNB + BNB liquidity");
    }

    let addr_a = resolve_token_address(token_a, chain_id);
    let addr_b = resolve_token_address(token_b, chain_id);

    let wallet = if dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(chain_id)?
    };
    let dl = deadline();

    // Step 1: Get pair address (= LP token address)
    let pair_addr = if !dry_run {
        factory_get_pair(factory, &addr_a, &addr_b, rpc).await?
    } else {
        "0x0000000000000000000000000000000000000000".to_string()
    };

    if !dry_run && pair_addr == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!("Pair does not exist for {} / {}", token_a, token_b);
    }

    // Step 2: Get LP balance
    let lp_balance = if dry_run {
        liquidity.unwrap_or(1_000_000_000_000_000_000u128) // 1e18 placeholder
    } else {
        let bal = get_balance(&pair_addr, &wallet, rpc).await?;
        if bal == 0 {
            anyhow::bail!("No LP balance found for {} / {} pool", token_a, token_b);
        }
        liquidity.unwrap_or(bal)
    };

    // Step 3: Estimate expected amounts from reserves
    let (amount_a_min, amount_b_min) = if !dry_run {
        let (r0, r1) = get_reserves(&pair_addr, rpc).await?;
        let total_supply = get_total_supply(&pair_addr, rpc).await?;
        let token0 = get_token0(&pair_addr, rpc).await?;

        // Determine which reserve corresponds to tokenA and tokenB
        let (reserve_a, reserve_b) = if token0.to_lowercase() == addr_a.to_lowercase() {
            (r0, r1)
        } else {
            (r1, r0)
        };

        let expected_a = if total_supply > 0 {
            (lp_balance as u128).saturating_mul(reserve_a) / total_supply
        } else {
            0
        };
        let expected_b = if total_supply > 0 {
            (lp_balance as u128).saturating_mul(reserve_b) / total_supply
        } else {
            0
        };

        (apply_slippage(expected_a), apply_slippage(expected_b))
    } else {
        (0u128, 0u128)
    };

    // Step 4: Approve LP token to Router (if needed)
    if !dry_run {
        let allowance = get_allowance(&pair_addr, &wallet, router, rpc).await?;
        if allowance < lp_balance {
            println!("  Approving LP token for Router...");
            // User confirmation is required before submitting approve. Ask the user to confirm.
            let approve_cd = build_approve_calldata(router, lp_balance);
            let ar = wallet_contract_call(chain_id, &pair_addr, &approve_cd, None, None, false).await?;
            println!("  approve txHash: {}", extract_tx_hash(&ar));
            sleep(Duration::from_secs(5)).await;
        }
    }

    if a_is_bnb || b_is_bnb {
        // --- removeLiquidityETH ---
        // Determine which is the ERC-20 token
        let (token_addr, token_sym, amount_token_min, amount_eth_min) = if b_is_bnb {
            (addr_a.as_str(), token_a, amount_a_min, amount_b_min)
        } else {
            (addr_b.as_str(), token_b, amount_b_min, amount_a_min)
        };

        // removeLiquidityETH(address token, uint256 liquidity, uint256 amountTokenMin,
        //                    uint256 amountETHMin, address to, uint256 deadline)
        // Selector: 0x02751cec
        // All fixed params:
        //   word 0: token
        //   word 1: liquidity
        //   word 2: amountTokenMin
        //   word 3: amountETHMin
        //   word 4: to
        //   word 5: deadline
        let calldata = format!(
            "0x02751cec{}{}{}{}{}{}",
            pad_address(token_addr),
            pad_u256(lp_balance),
            pad_u256(amount_token_min),
            pad_u256(amount_eth_min),
            pad_address(&wallet),
            pad_u256(dl as u128)
        );

        println!("Remove Liquidity ETH: {} / BNB", token_sym.to_uppercase());
        println!("  liquidity (LP): {}", lp_balance);
        println!("  amountTokenMin: {}", amount_token_min);
        println!("  amountETHMin: {}", amount_eth_min);
        println!("  to: {}", wallet);

        // User confirmation is required before submitting. Ask the user to confirm before proceeding.
        let result = wallet_contract_call(chain_id, router, &calldata, None, None, dry_run).await?;
        println!("  txHash: {}", extract_tx_hash(&result));

    } else {
        // --- removeLiquidity: token + token ---
        // removeLiquidity(address tokenA, address tokenB, uint256 liquidity,
        //                 uint256 amountAMin, uint256 amountBMin, address to, uint256 deadline)
        // Selector: 0xbaa2abde
        // All fixed params:
        //   word 0: tokenA
        //   word 1: tokenB
        //   word 2: liquidity
        //   word 3: amountAMin
        //   word 4: amountBMin
        //   word 5: to
        //   word 6: deadline
        let calldata = format!(
            "0xbaa2abde{}{}{}{}{}{}{}",
            pad_address(&addr_a),
            pad_address(&addr_b),
            pad_u256(lp_balance),
            pad_u256(amount_a_min),
            pad_u256(amount_b_min),
            pad_address(&wallet),
            pad_u256(dl as u128)
        );

        println!("Remove Liquidity: {} / {}", token_a.to_uppercase(), token_b.to_uppercase());
        println!("  liquidity (LP): {}", lp_balance);
        println!("  amountAMin: {}", amount_a_min);
        println!("  amountBMin: {}", amount_b_min);
        println!("  to: {}", wallet);

        // User confirmation is required before submitting. Ask the user to confirm before proceeding.
        let result = wallet_contract_call(chain_id, router, &calldata, None, None, dry_run).await?;
        println!("  txHash: {}", extract_tx_hash(&result));
    }

    Ok(())
}
