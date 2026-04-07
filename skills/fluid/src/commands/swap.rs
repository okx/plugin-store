use crate::calldata;
use crate::config::{get_chain_config, get_dex_pool};
use crate::onchainos;

/// Swap tokens via Fluid DEX.
/// Routes through the appropriate Fluid DEX pool contract.
/// For ERC-20 tokens: Step 1 approve pool, Step 2 swapIn.
/// For ETH input: swapIn with msg.value (payable).
pub async fn run(
    token_in: &str,
    token_out: &str,
    amount_in: &str,
    _slippage_bps: u32,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let _cfg = get_chain_config(chain_id)?;
    let (pool, swap0to1) = get_dex_pool(token_in, token_out, chain_id)?;

    let in_decimals = if swap0to1 { pool.token0_decimals } else { pool.token1_decimals };
    let in_is_eth = if swap0to1 { pool.token0_is_eth } else { pool.token1_is_eth };
    let in_token_addr = if swap0to1 { pool.token0 } else { pool.token1 };

    let raw_in = calldata::parse_amount(amount_in, in_decimals)?;
    // Compute minimum out: 0 slippage protection by default, caller can specify
    let amount_out_min = 0u128; // simplified: no min output (user should use --dry-run to check)

    // Resolve wallet
    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, dry_run)?
    };

    let swap_calldata = calldata::encode_swap_in(swap0to1, raw_in, amount_out_min, &wallet);

    let mut approve_tx = "N/A".to_string();

    if !in_is_eth {
        // Step 1: Approve pool to spend token_in
        let approve_calldata = calldata::encode_approve(pool.address, raw_in);
        eprintln!("[fluid] Step 1/2: Approving DEX pool {} to spend {} {}...", pool.address, amount_in, token_in);
        if dry_run {
            eprintln!("[fluid] [dry-run] Would approve: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, in_token_addr, approve_calldata);
        }
        let approve_result = onchainos::wallet_contract_call(
            chain_id, in_token_addr, &approve_calldata, from, None, dry_run
        ).await?;
        approve_tx = onchainos::extract_tx_hash(&approve_result).to_string();

        if !dry_run {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
        eprintln!("[fluid] Step 2/2: Swapping {} {} -> {}...", amount_in, token_in, token_out);
    } else {
        // ETH input — send as msg.value
        eprintln!("[fluid] Swapping {} ETH -> {}...", amount_in, token_out);
    }

    if dry_run {
        let value_note = if in_is_eth { format!(" --amt {}", raw_in) } else { String::new() };
        eprintln!(
            "[fluid] [dry-run] Would swap: onchainos wallet contract-call --chain {} --to {}{} --input-data {}",
            chain_id, pool.address, value_note, swap_calldata
        );
    }

    let eth_value = if in_is_eth { Some(raw_in) } else { None };
    let swap_result = onchainos::wallet_contract_call(
        chain_id, pool.address, &swap_calldata, from, eth_value, dry_run
    ).await?;
    let swap_tx = onchainos::extract_tx_hash(&swap_result).to_string();

    let output = serde_json::json!({
        "ok": true,
        "operation": "swap",
        "pool": pool.address,
        "tokenIn": token_in,
        "tokenOut": token_out,
        "amountIn": amount_in,
        "amountInRaw": raw_in.to_string(),
        "swap0to1": swap0to1,
        "chainId": chain_id,
        "dryRun": dry_run,
        "approveTxHash": approve_tx,
        "swapTxHash": swap_tx,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
