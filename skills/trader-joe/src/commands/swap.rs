use tokio::time::{sleep, Duration};

use crate::commands::quote::{find_best_path_in, version_name};
use crate::config::{
    apply_slippage, build_approve_calldata, deadline, is_native_eth, pad_address,
    pad_u256, resolve_token_address, encode_u256_array, encode_u8_array,
    encode_address_array, LB_ROUTER, RPC_URL, WETH,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::get_allowance;

/// Build swapExactTokensForTokens calldata for Trader Joe LB.
///
/// Function: swapExactTokensForTokens(uint256 amountIn, uint256 amountOutMin,
///             (uint256[] pairBinSteps, uint8[] versions, address[] tokenPath) path,
///             address to, uint256 deadline)
/// Selector: 0x2a443fae
///
/// ABI encoding for tuple:
/// The function args (amountIn, amountOutMin, path_tuple, to, deadline)
/// where path_tuple is itself a struct with 3 dynamic arrays.
/// ABI encoding rules:
/// - Static args go in-line
/// - Dynamic args (the path tuple containing arrays) get an offset pointer
///
/// Layout (each slot = 32 bytes = 64 hex chars):
///   [0x00] amountIn
///   [0x20] amountOutMin
///   [0x40] offset to path tuple = 0xa0 (5 slots before dynamic data)
///   [0x60] to (address)
///   [0x80] deadline
///   --- path tuple data starts here ---
///   [0xa0] offset to pairBinSteps[] within tuple = 0x60 (3 slots)
///   [0xc0] offset to versions[] within tuple = ?
///   [0xe0] offset to tokenPath[] within tuple = ?
///   [0x100] pairBinSteps[].length
///   [0x120+] pairBinSteps elements
///   [?] versions[].length
///   [?+] versions elements
///   [?] tokenPath[].length
///   [?+] tokenPath elements
pub fn build_swap_exact_tokens_for_tokens_calldata(
    amount_in: u128,
    amount_out_min: u128,
    pair_bin_steps: &[u128],
    versions: &[u8],
    token_path: &[&str],
    to: &str,
    deadline_ts: u64,
) -> String {
    // Build the path tuple's internal encoding first
    // The tuple has 3 dynamic arrays. Offsets are relative to start of tuple data.
    // Tuple data layout:
    //   slot 0: offset to pairBinSteps[]
    //   slot 1: offset to versions[]
    //   slot 2: offset to tokenPath[]
    //   then the arrays

    // Calculate sizes of each array encoding: 1 length word + N element words
    let n = pair_bin_steps.len();

    // pairBinSteps[]: length(1) + N words
    let bin_steps_size = 1 + n; // in 32-byte slots
    // versions[]: length(1) + N words
    let versions_size = 1 + n;
    // tokenPath[]: length(1) + (N+1) words (N+1 tokens for N hops)
    let _token_path_size = 1 + token_path.len();

    // Offsets relative to start of tuple (3 header slots = 96 bytes = 0x60)
    let offset_bin_steps: usize = 0x60; // 3 * 32 bytes
    let offset_versions: usize = offset_bin_steps + (bin_steps_size * 32);
    let offset_token_path: usize = offset_versions + (versions_size * 32);

    // Build tuple header (3 offset pointers)
    let tuple_header = format!(
        "{}{}{}",
        pad_u256(offset_bin_steps as u128),
        pad_u256(offset_versions as u128),
        pad_u256(offset_token_path as u128)
    );

    // Build the arrays
    let bin_steps_encoded = encode_u256_array(pair_bin_steps);
    let versions_encoded = encode_u8_array(versions);
    let token_path_strs: Vec<&str> = token_path.to_vec();
    let token_path_encoded = encode_address_array(&token_path_strs);

    let tuple_data = format!("{}{}{}{}", tuple_header, bin_steps_encoded, versions_encoded, token_path_encoded);

    // Now encode the outer function call
    // Args: amountIn (static), amountOutMin (static), path_tuple (dynamic → offset), to (static), deadline (static)
    // 5 static-or-pointer slots before dynamic data:
    // [0x00] amountIn
    // [0x20] amountOutMin
    // [0x40] offset to path tuple = 0xa0 (5 * 32 = 160 bytes)
    // [0x60] to
    // [0x80] deadline
    let outer = format!(
        "0x2a443fae{}{}{}{}{}{}",
        pad_u256(amount_in),
        pad_u256(amount_out_min),
        pad_u256(0xa0u128), // offset to tuple = 5 slots = 160 bytes = 0xa0
        pad_address(to),
        pad_u256(deadline_ts as u128),
        tuple_data
    );
    outer
}

/// Build swapExactNATIVEForTokens calldata (ETH → token).
/// Selector: 0xb066ea7c
/// swapExactNATIVEForTokens(uint256 amountOutMin,
///   (uint256[] pairBinSteps, uint8[] versions, address[] tokenPath) path,
///   address to, uint256 deadline)
/// The ETH value is sent as msg.value (--amt in onchainos)
pub fn build_swap_exact_native_for_tokens_calldata(
    amount_out_min: u128,
    pair_bin_steps: &[u128],
    versions: &[u8],
    token_path: &[&str],
    to: &str,
    deadline_ts: u64,
) -> String {
    let n = pair_bin_steps.len();
    let bin_steps_size = 1 + n;
    let versions_size = 1 + n;

    let offset_bin_steps: usize = 0x60;
    let offset_versions: usize = offset_bin_steps + (bin_steps_size * 32);
    let offset_token_path: usize = offset_versions + (versions_size * 32);

    let tuple_header = format!(
        "{}{}{}",
        pad_u256(offset_bin_steps as u128),
        pad_u256(offset_versions as u128),
        pad_u256(offset_token_path as u128)
    );

    let bin_steps_encoded = encode_u256_array(pair_bin_steps);
    let versions_encoded = encode_u8_array(versions);
    let token_path_strs: Vec<&str> = token_path.to_vec();
    let token_path_encoded = encode_address_array(&token_path_strs);

    let tuple_data = format!("{}{}{}{}", tuple_header, bin_steps_encoded, versions_encoded, token_path_encoded);

    // Args: amountOutMin (static), path_tuple (dynamic → offset), to (static), deadline (static)
    // 4 static-or-pointer slots before dynamic data:
    // [0x00] amountOutMin
    // [0x20] offset to path tuple = 0x80 (4 * 32 = 128 bytes)
    // [0x40] to
    // [0x60] deadline
    format!(
        "0xb066ea7c{}{}{}{}{}",
        pad_u256(amount_out_min),
        pad_u256(0x80u128), // offset to tuple
        pad_address(to),
        pad_u256(deadline_ts as u128),
        tuple_data
    )
}

/// swap command: execute token swap on Trader Joe Liquidity Book.
pub async fn run(
    token_in: &str,
    token_out: &str,
    amount: f64,
    decimals_in: u8,
    slippage_bps: u128, // e.g. 50 = 0.5%
    chain_id: u64,
    dry_run: bool,
) -> anyhow::Result<()> {
    let rpc_url = RPC_URL;
    let router = LB_ROUTER;

    let in_is_eth = is_native_eth(token_in);

    let addr_in = if in_is_eth {
        WETH.to_string()
    } else {
        resolve_token_address(token_in, chain_id)
    };
    let addr_out = resolve_token_address(token_out, chain_id);

    // Convert human amount to raw units
    let amount_in_raw = (amount * 10f64.powi(decimals_in as i32)) as u128;

    // Dry-run guard: resolve wallet only after dry-run check
    let recipient = if dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(chain_id)?
    };

    // Get quote from LBQuoter
    let q = find_best_path_in(&addr_in, &addr_out, amount_in_raw, rpc_url).await?;
    if q.amount_out() == 0 {
        anyhow::bail!("No route found or quote is zero for {}/{}", token_in, token_out);
    }

    let amount_out_min = apply_slippage(q.amount_out(), slippage_bps);
    let dl = deadline(300);

    eprintln!(
        "Quote: {} {} → ~{} {} (binStep={}, version={})",
        amount,
        token_in.to_uppercase(),
        q.amount_out(),
        token_out.to_uppercase(),
        q.bin_steps.first().copied().unwrap_or(0),
        q.versions.first().map(|v| version_name(*v)).unwrap_or("?")
    );
    eprintln!("  amountOutMin: {} (slippage {}bps)", amount_out_min, slippage_bps);
    eprintln!("  recipient: {}", recipient);

    if in_is_eth {
        // ETH → token: use swapExactNATIVEForTokens
        // token_path starts with WETH
        let token_path_strs: Vec<String> = {
            let mut p = vec![WETH.to_string()];
            for addr in &q.route[1..] {
                p.push(addr.clone());
            }
            p
        };
        let token_path_refs: Vec<&str> = token_path_strs.iter().map(|s| s.as_str()).collect();

        let calldata = build_swap_exact_native_for_tokens_calldata(
            amount_out_min,
            &q.bin_steps,
            &q.versions,
            &token_path_refs,
            &recipient,
            dl,
        );

        // Ask user to confirm before executing on-chain swap.
        let result = wallet_contract_call(
            chain_id, router, &calldata, None, Some(amount_in_raw), dry_run,
        )
        .await?;

        print_result(&result, token_in, token_out, amount, amount_out_min, dry_run)?;
    } else {
        // ERC-20 → token: use swapExactTokensForTokens
        // Check allowance and approve if needed
        if !dry_run {
            let allowance = get_allowance(&addr_in, &recipient, router, rpc_url).await?;
            if allowance < amount_in_raw {
                eprintln!("  Approving {} for LBRouter...", token_in.to_uppercase());
                // Ask user to confirm the approve transaction before submitting.
                let approve_calldata = build_approve_calldata(router, u128::MAX);
                let approve_result = wallet_contract_call(
                    chain_id, &addr_in, &approve_calldata, None, None, false,
                )
                .await?;
                eprintln!("  approve txHash: {}", extract_tx_hash(&approve_result));
                sleep(Duration::from_secs(3)).await;
            }
        }

        let token_path_strs: Vec<String> = q.route.clone();
        let token_path_refs: Vec<&str> = token_path_strs.iter().map(|s| s.as_str()).collect();

        let calldata = build_swap_exact_tokens_for_tokens_calldata(
            amount_in_raw,
            amount_out_min,
            &q.bin_steps,
            &q.versions,
            &token_path_refs,
            &recipient,
            dl,
        );

        // Ask user to confirm before executing on-chain swap.
        let result = wallet_contract_call(chain_id, router, &calldata, None, None, dry_run).await?;

        print_result(&result, token_in, token_out, amount, amount_out_min, dry_run)?;
    }

    Ok(())
}

fn print_result(
    result: &serde_json::Value,
    token_in: &str,
    token_out: &str,
    amount: f64,
    amount_out_min: u128,
    dry_run: bool,
) -> anyhow::Result<()> {
    let tx_hash = extract_tx_hash(result);
    let out = serde_json::json!({
        "ok": true,
        "dry_run": dry_run,
        "data": {
            "txHash": tx_hash,
            "tokenIn": token_in.to_uppercase(),
            "tokenOut": token_out.to_uppercase(),
            "amountIn": amount,
            "amountOutMin": amount_out_min.to_string(),
            "calldata": result.get("calldata")
        }
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
