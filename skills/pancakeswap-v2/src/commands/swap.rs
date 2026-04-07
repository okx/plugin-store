use tokio::time::{sleep, Duration};

use crate::config::{
    resolve_token_address, is_native_bnb, apply_slippage, deadline, pad_address, pad_u256,
    encode_address_array, build_approve_calldata, ROUTER_V2, WBNB, BSC_RPC, CHAIN_ID,
};
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};
use crate::rpc::{get_amounts_out, get_allowance};

/// Swap tokens on PancakeSwap V2.
///
/// Handles three variants:
///   1. BNB → token  (swapExactETHForTokens, payable, no approve needed)
///   2. token → BNB  (swapExactTokensForETH)
///   3. token → token (swapExactTokensForTokens, routes via WBNB)
pub async fn run(
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    dry_run: bool,
) -> anyhow::Result<()> {
    let chain_id = CHAIN_ID;
    let rpc = BSC_RPC;
    let router = ROUTER_V2;

    let in_is_bnb = is_native_bnb(token_in);
    let out_is_bnb = is_native_bnb(token_out);

    let addr_in = resolve_token_address(token_in, chain_id);
    let addr_out = resolve_token_address(token_out, chain_id);

    let recipient = if dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(chain_id)?
    };
    let dl = deadline();

    if in_is_bnb {
        // --- Variant 1: BNB → token (swapExactETHForTokens) ---
        // path starts with WBNB
        let path = [WBNB, addr_out.as_str()];
        let amounts = get_amounts_out(router, amount_in, &path, rpc).await?;
        let amount_out_min = apply_slippage(*amounts.last().unwrap_or(&0));

        // swapExactETHForTokens(uint256 amountOutMin, address[] path, address to, uint256 deadline)
        // Selector: 0x7ff36ab5
        // ABI layout:
        //   [0] amountOutMin
        //   [1] offset to path = 0x80 (4 fixed words before dynamic: amountOutMin, offset, to, deadline... wait)
        // Actually: (amountOutMin, path[], to, deadline)
        //   word 0: amountOutMin
        //   word 1: offset to path = 0x80 (4 words = 128 bytes before array data start)
        //   word 2: to
        //   word 3: deadline
        //   word 4: path.length
        //   word 5+: path elements
        let amount_out_min_hex = pad_u256(amount_out_min);
        let path_offset_hex = pad_u256(0x80); // 4 fixed words * 32 bytes = 128 = 0x80
        let to_hex = pad_address(&recipient);
        let dl_hex = pad_u256(dl as u128);
        let path_encoded = encode_address_array(&path);
        let calldata = format!(
            "0x7ff36ab5{}{}{}{}{}",
            amount_out_min_hex,
            path_offset_hex,
            to_hex,
            dl_hex,
            path_encoded
        );

        println!("Swap: {} wei BNB → {} (swapExactETHForTokens)", amount_in, token_out.to_uppercase());
        println!("  amountOutMin: {}", amount_out_min);
        println!("  to: {}", recipient);
        println!("  deadline: {}", dl);

        // User confirmation is required before submitting. Ask the user to confirm before proceeding.
        let result = wallet_contract_call(
            chain_id, router, &calldata, None, Some(amount_in), dry_run,
        ).await?;
        println!("  txHash: {}", extract_tx_hash(&result));

    } else if out_is_bnb {
        // --- Variant 2: token → BNB (swapExactTokensForETH) ---
        // Selector: 0x18cbafe5
        // path ends with WBNB
        let path = [addr_in.as_str(), WBNB];
        let amounts = get_amounts_out(router, amount_in, &path, rpc).await?;
        let amount_out_min = apply_slippage(*amounts.last().unwrap_or(&0));

        // Check and do approve if needed
        if !dry_run {
            let allowance = get_allowance(&addr_in, &recipient, router, rpc).await?;
            if allowance < amount_in {
                println!("  Approving {} for Router...", token_in.to_uppercase());
                // User confirmation is required before submitting approve. Ask the user to confirm.
                let approve_calldata = build_approve_calldata(router, u128::MAX);
                let approve_result = wallet_contract_call(
                    chain_id, &addr_in, &approve_calldata, None, None, false,
                ).await?;
                println!("  approve txHash: {}", extract_tx_hash(&approve_result));
                sleep(Duration::from_secs(3)).await;
            }
        }

        // swapExactTokensForETH(uint256 amountIn, uint256 amountOutMin, address[] path, address to, uint256 deadline)
        // ABI layout:
        //   word 0: amountIn
        //   word 1: amountOutMin
        //   word 2: offset to path = 0xa0 (5 fixed words = 160 bytes)
        //   word 3: to
        //   word 4: deadline
        //   word 5: path.length
        //   word 6+: path elements
        let calldata = format!(
            "0x18cbafe5{}{}{}{}{}{}",
            pad_u256(amount_in),
            pad_u256(amount_out_min),
            pad_u256(0xa0),
            pad_address(&recipient),
            pad_u256(dl as u128),
            encode_address_array(&path)
        );

        println!("Swap: {} (token → BNB, swapExactTokensForETH)", token_in.to_uppercase());
        println!("  amountIn: {}", amount_in);
        println!("  amountOutMin: {} wei BNB", amount_out_min);
        println!("  to: {}", recipient);

        // User confirmation is required before submitting. Ask the user to confirm before proceeding.
        let result = wallet_contract_call(
            chain_id, router, &calldata, None, None, dry_run,
        ).await?;
        println!("  txHash: {}", extract_tx_hash(&result));

    } else {
        // --- Variant 3: token → token (swapExactTokensForTokens, route via WBNB) ---
        // Selector: 0x38ed1739

        // Use WBNB as intermediate hop for maximum liquidity
        let wbnb_lower = WBNB.to_lowercase();
        let ai_lower = addr_in.to_lowercase();
        let ao_lower = addr_out.to_lowercase();

        let (path_vec, path_desc): (Vec<String>, String) =
            if ai_lower == wbnb_lower || ao_lower == wbnb_lower {
                // direct path
                (
                    vec![addr_in.clone(), addr_out.clone()],
                    format!("{} → {}", token_in.to_uppercase(), token_out.to_uppercase()),
                )
            } else {
                // route via WBNB
                (
                    vec![addr_in.clone(), WBNB.to_string(), addr_out.clone()],
                    format!("{} → WBNB → {}", token_in.to_uppercase(), token_out.to_uppercase()),
                )
            };

        let path: Vec<&str> = path_vec.iter().map(|s| s.as_str()).collect();
        let amounts = get_amounts_out(router, amount_in, &path, rpc).await?;
        let amount_out_min = apply_slippage(*amounts.last().unwrap_or(&0));

        // Check and do approve if needed
        if !dry_run {
            let allowance = get_allowance(&addr_in, &recipient, router, rpc).await?;
            if allowance < amount_in {
                println!("  Approving {} for Router...", token_in.to_uppercase());
                // User confirmation is required before submitting approve. Ask the user to confirm.
                let approve_calldata = build_approve_calldata(router, u128::MAX);
                let approve_result = wallet_contract_call(
                    chain_id, &addr_in, &approve_calldata, None, None, false,
                ).await?;
                println!("  approve txHash: {}", extract_tx_hash(&approve_result));
                sleep(Duration::from_secs(3)).await;
            }
        }

        // swapExactTokensForTokens(uint256 amountIn, uint256 amountOutMin, address[] path, address to, uint256 deadline)
        // ABI layout:
        //   word 0: amountIn
        //   word 1: amountOutMin
        //   word 2: offset to path = 0xa0 (5 fixed words)
        //   word 3: to
        //   word 4: deadline
        //   word 5: path.length
        //   word 6+: path elements
        let calldata = format!(
            "0x38ed1739{}{}{}{}{}{}",
            pad_u256(amount_in),
            pad_u256(amount_out_min),
            pad_u256(0xa0),
            pad_address(&recipient),
            pad_u256(dl as u128),
            encode_address_array(&path)
        );

        println!("Swap: {} (token → token, swapExactTokensForTokens)", path_desc);
        println!("  amountIn: {}", amount_in);
        println!("  amountOutMin: {}", amount_out_min);
        println!("  to: {}", recipient);

        // User confirmation is required before submitting. Ask the user to confirm before proceeding.
        let result = wallet_contract_call(
            chain_id, router, &calldata, None, None, dry_run,
        ).await?;
        println!("  txHash: {}", extract_tx_hash(&result));
    }

    Ok(())
}
