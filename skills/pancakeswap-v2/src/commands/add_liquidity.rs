use tokio::time::{sleep, Duration};

use crate::config::{
    resolve_token_address, is_native_bnb, apply_slippage, deadline, pad_address, pad_u256,
    build_approve_calldata, ROUTER_V2, BSC_RPC, CHAIN_ID,
};
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};
use crate::rpc::get_allowance;

/// Add liquidity to a PancakeSwap V2 pool.
///
/// Handles two variants:
///   1. token + token  → addLiquidity
///   2. token + BNB    → addLiquidityETH (payable)
pub async fn run(
    token_a: &str,
    token_b: &str,
    amount_a: u128,
    amount_b: u128,
    dry_run: bool,
) -> anyhow::Result<()> {
    let chain_id = CHAIN_ID;
    let rpc = BSC_RPC;
    let router = ROUTER_V2;

    let b_is_bnb = is_native_bnb(token_b);
    let a_is_bnb = is_native_bnb(token_a);

    if a_is_bnb && b_is_bnb {
        anyhow::bail!("Cannot add BNB + BNB liquidity");
    }

    let addr_a = resolve_token_address(token_a, chain_id);
    let addr_b = resolve_token_address(token_b, chain_id);

    let recipient = if dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(chain_id)?
    };
    let dl = deadline();

    if a_is_bnb || b_is_bnb {
        // --- addLiquidityETH: token + BNB ---
        // Determine which is the token and which is BNB
        let (token_addr, token_sym, token_amount, bnb_amount) = if b_is_bnb {
            (addr_a.as_str(), token_a, amount_a, amount_b)
        } else {
            (addr_b.as_str(), token_b, amount_b, amount_a)
        };

        let amount_token_min = apply_slippage(token_amount);
        let amount_eth_min = apply_slippage(bnb_amount);

        // Step 1: approve token (if needed)
        if !dry_run {
            let allowance = get_allowance(token_addr, &recipient, router, rpc).await?;
            if allowance < token_amount {
                println!("  Approving {} for Router...", token_sym.to_uppercase());
                // User confirmation is required before submitting approve. Ask the user to confirm.
                let approve_cd = build_approve_calldata(router, u128::MAX);
                let ar = wallet_contract_call(chain_id, token_addr, &approve_cd, None, None, false).await?;
                println!("  approve txHash: {}", extract_tx_hash(&ar));
                sleep(Duration::from_secs(5)).await;
            }
        }

        // Step 2: addLiquidityETH
        // addLiquidityETH(address token, uint256 amountTokenDesired, uint256 amountTokenMin,
        //                 uint256 amountETHMin, address to, uint256 deadline)
        // Selector: 0xf305d719
        // All fixed params (no dynamic arrays):
        //   word 0: token
        //   word 1: amountTokenDesired
        //   word 2: amountTokenMin
        //   word 3: amountETHMin
        //   word 4: to
        //   word 5: deadline
        let calldata = format!(
            "0xf305d719{}{}{}{}{}{}",
            pad_address(token_addr),
            pad_u256(token_amount),
            pad_u256(amount_token_min),
            pad_u256(amount_eth_min),
            pad_address(&recipient),
            pad_u256(dl as u128)
        );

        println!("Add Liquidity ETH: {} + {} BNB", token_sym.to_uppercase(), bnb_amount);
        println!("  amountTokenDesired: {}", token_amount);
        println!("  amountTokenMin: {}", amount_token_min);
        println!("  amountETHMin: {}", amount_eth_min);
        println!("  to: {}", recipient);

        // User confirmation is required before submitting. Ask the user to confirm before proceeding.
        let result = wallet_contract_call(
            chain_id, router, &calldata, None, Some(bnb_amount), dry_run,
        ).await?;
        println!("  txHash: {}", extract_tx_hash(&result));

    } else {
        // --- addLiquidity: token + token ---
        // Step 1: approve tokenA (if needed)
        if !dry_run {
            let allowance_a = get_allowance(&addr_a, &recipient, router, rpc).await?;
            if allowance_a < amount_a {
                println!("  Approving {} for Router...", token_a.to_uppercase());
                // User confirmation is required before submitting approve. Ask the user to confirm.
                let approve_cd = build_approve_calldata(router, u128::MAX);
                let ar = wallet_contract_call(chain_id, &addr_a, &approve_cd, None, None, false).await?;
                println!("  approve tokenA txHash: {}", extract_tx_hash(&ar));
                sleep(Duration::from_secs(5)).await;
            }
        }

        // Step 2: approve tokenB (if needed)
        if !dry_run {
            let allowance_b = get_allowance(&addr_b, &recipient, router, rpc).await?;
            if allowance_b < amount_b {
                println!("  Approving {} for Router...", token_b.to_uppercase());
                // User confirmation is required before submitting approve. Ask the user to confirm.
                let approve_cd = build_approve_calldata(router, u128::MAX);
                let ar = wallet_contract_call(chain_id, &addr_b, &approve_cd, None, None, false).await?;
                println!("  approve tokenB txHash: {}", extract_tx_hash(&ar));
                sleep(Duration::from_secs(5)).await;
            }
        }

        let amount_a_min = apply_slippage(amount_a);
        let amount_b_min = apply_slippage(amount_b);

        // addLiquidity(address tokenA, address tokenB, uint256 amountADesired, uint256 amountBDesired,
        //              uint256 amountAMin, uint256 amountBMin, address to, uint256 deadline)
        // Selector: 0xe8e33700
        // All fixed params:
        //   word 0: tokenA
        //   word 1: tokenB
        //   word 2: amountADesired
        //   word 3: amountBDesired
        //   word 4: amountAMin
        //   word 5: amountBMin
        //   word 6: to
        //   word 7: deadline
        let calldata = format!(
            "0xe8e33700{}{}{}{}{}{}{}{}",
            pad_address(&addr_a),
            pad_address(&addr_b),
            pad_u256(amount_a),
            pad_u256(amount_b),
            pad_u256(amount_a_min),
            pad_u256(amount_b_min),
            pad_address(&recipient),
            pad_u256(dl as u128)
        );

        println!("Add Liquidity: {} + {}", token_a.to_uppercase(), token_b.to_uppercase());
        println!("  amountADesired: {}", amount_a);
        println!("  amountBDesired: {}", amount_b);
        println!("  amountAMin: {}", amount_a_min);
        println!("  amountBMin: {}", amount_b_min);
        println!("  to: {}", recipient);

        // User confirmation is required before submitting. Ask the user to confirm before proceeding.
        let result = wallet_contract_call(
            chain_id, router, &calldata, None, None, dry_run,
        ).await?;
        println!("  txHash: {}", extract_tx_hash(&result));
    }

    Ok(())
}
