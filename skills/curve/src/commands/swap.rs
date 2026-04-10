// commands/swap.rs — Execute a swap via Curve pool exchange()
use crate::{api, config, curve_abi, onchainos, rpc};
use anyhow::Result;

/// Determine whether a pool uses uint256 or int128 indices.
/// Factory v2 (CryptoSwap, tricrypto) pools use uint256; classic StableSwap pools use int128.
fn uses_uint256_indices(pool: &api::PoolData) -> bool {
    let id = pool.id.to_lowercase();
    id.contains("factory-crypto") || id.contains("tricrypto") || id.contains("crypto")
}

pub async fn run(
    chain_id: u64,
    token_in: String,
    token_out: String,
    amount_in: u128,
    slippage: f64,
    wallet: Option<String>,
    dry_run: bool,
) -> Result<()> {
    let chain_name = config::chain_name(chain_id);
    let rpc_url = config::rpc_url(chain_id);

    let token_in_addr = config::resolve_token_address(&token_in, chain_id);
    let token_out_addr = config::resolve_token_address(&token_out, chain_id);
    let is_native = config::is_native_eth(&token_in_addr);

    // Resolve wallet address — always fetch real address even in dry_run for accurate preview
    let wallet_addr = match wallet.clone() {
        Some(w) => w,
        None => {
            let w = onchainos::resolve_wallet(chain_id)?;
            if w.is_empty() {
                anyhow::bail!("Cannot determine wallet address. Pass --wallet or ensure onchainos is logged in.");
            }
            w
        }
    };

    // Fetch pools and find matching pool
    let pools = api::get_all_pools(chain_name).await?;
    let matching_pools = api::find_pools_for_pair(&pools, &token_in_addr, &token_out_addr);

    if matching_pools.is_empty() {
        anyhow::bail!(
            "No Curve pool found on {} containing both {} and {}",
            chain_name,
            token_in,
            token_out
        );
    }

    let pool = matching_pools[0];
    let in_idx = api::coin_index(pool, &token_in_addr).unwrap_or(0);
    let out_idx = api::coin_index(pool, &token_out_addr).unwrap_or(1);
    let use_uint256 = uses_uint256_indices(pool);

    // Get a quote to determine expected output
    let get_dy_calldata = if use_uint256 {
        curve_abi::encode_get_dy_uint256(in_idx as u64, out_idx as u64, amount_in)
    } else {
        curve_abi::encode_get_dy(in_idx as i64, out_idx as i64, amount_in)
    };

    let result_hex = rpc::eth_call(&pool.address, &get_dy_calldata, rpc_url).await?;
    let amount_out = rpc::decode_uint128(&result_hex);

    if amount_out == 0 {
        anyhow::bail!("Quote returned 0 — pool may have insufficient liquidity");
    }

    let min_expected = (amount_out as f64 * (1.0 - slippage)) as u128;

    // Build exchange calldata
    // Selector: 0x3df02124 = exchange(int128,int128,uint256,uint256) for StableSwap pools
    // Selector: 0x5b41b908 = exchange(uint256,uint256,uint256,uint256) for CryptoSwap/factory-v2 pools
    let calldata = if use_uint256 {
        curve_abi::encode_exchange_uint256(in_idx as u64, out_idx as u64, amount_in, min_expected)
    } else {
        curve_abi::encode_exchange(in_idx as i64, out_idx as i64, amount_in, min_expected)
    };

    if dry_run {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain": chain_name,
                "pool": { "id": pool.id, "name": pool.name, "address": pool.address },
                "token_in": { "symbol": token_in, "address": token_in_addr, "index": in_idx },
                "token_out": { "symbol": token_out, "address": token_out_addr, "index": out_idx },
                "amount_in_raw": amount_in.to_string(),
                "expected_out_raw": amount_out.to_string(),
                "min_expected_raw": min_expected.to_string(),
                "slippage_pct": slippage * 100.0,
                "calldata": calldata,
                "target_contract": pool.address
            })
        );
        return Ok(());
    }

    // ERC-20 approve if not native ETH
    if !is_native {
        let allowance = rpc::get_allowance(&token_in_addr, &wallet_addr, &pool.address, rpc_url).await?;
        if allowance < amount_in {
            eprintln!("Approving {} for Curve pool...", token_in);
            let approve_result = onchainos::erc20_approve(
                chain_id,
                &token_in_addr,
                &pool.address,
                amount_in,
                Some(&wallet_addr),
                false,
            )
            .await?;
            let approve_hash = onchainos::extract_tx_hash_or_err(&approve_result)?;
            eprintln!("Approve tx: {}", approve_hash);
            onchainos::wait_for_tx(&approve_hash, rpc_url).await?;
        }
    }

    // Execute swap — requires --force for DEX operations
    let amt = if is_native { Some(amount_in as u64) } else { None };
    let result = onchainos::wallet_contract_call(
        chain_id,
        &pool.address,
        &calldata,
        Some(&wallet_addr),
        amt,
        true,  // --force required for DEX swap
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;
    let explorer = config::explorer_url(chain_id, &tx_hash);

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "chain": chain_name,
            "pool": { "id": pool.id, "name": pool.name, "address": pool.address },
            "token_in": { "symbol": token_in, "address": token_in_addr },
            "token_out": { "symbol": token_out, "address": token_out_addr },
            "amount_in_raw": amount_in.to_string(),
            "expected_out_raw": amount_out.to_string(),
            "min_expected_raw": min_expected.to_string(),
            "tx_hash": tx_hash,
            "explorer": explorer
        })
    );
    Ok(())
}
