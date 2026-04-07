// commands/swap.rs — Execute a swap via Curve pool exchange()
use crate::{api, config, curve_abi, onchainos, rpc};
use anyhow::Result;
use tokio::time::{sleep, Duration};

/// Determine whether a pool uses uint256 or int128 indices.
/// Factory v2 (CryptoSwap, tricrypto) pools use uint256; classic StableSwap pools use int128.
/// NOTE: Some old-style CryptoSwap pools are registered in the main registry with numeric IDs
/// (e.g. id="38"). We therefore try uint256 first and fall back to int128 when the
/// uint256 call returns empty data.
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

    // Resolve wallet address
    let wallet_addr = if dry_run {
        wallet.clone().unwrap_or_else(|| curve_abi::ZERO_ADDR.to_string())
    } else {
        match wallet.clone() {
            Some(w) => w,
            None => {
                let w = onchainos::resolve_wallet(chain_id)?;
                if w.is_empty() {
                    anyhow::bail!("Cannot determine wallet address. Pass --wallet or ensure onchainos is logged in.");
                }
                w
            }
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
    let hint_uint256 = uses_uint256_indices(pool);

    // Get a quote to determine expected output.
    // Try uint256 selector first; if it returns 0 (unknown selector) fall back to int128.
    // This handles old-style CryptoSwap pools registered with numeric IDs in main registry.
    let (amount_out, use_uint256) = {
        let cd = curve_abi::encode_get_dy_uint256(in_idx as u64, out_idx as u64, amount_in);
        let hex = rpc::eth_call(&pool.address, &cd, rpc_url).await.unwrap_or_default();
        let val = rpc::decode_uint128(&hex);
        if val > 0 {
            (val, true)
        } else if hint_uint256 {
            // Pool is classified as crypto but uint256 also returned 0 — might be low liquidity
            (val, true)
        } else {
            let cd_i128 = curve_abi::encode_get_dy(in_idx as i64, out_idx as i64, amount_in);
            let hex_i128 = rpc::eth_call(&pool.address, &cd_i128, rpc_url).await?;
            (rpc::decode_uint128(&hex_i128), false)
        }
    };

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
                u128::MAX,
                Some(&wallet_addr),
                false,
            )
            .await?;
            let approve_hash = onchainos::extract_tx_hash(&approve_result)?;
            eprintln!("Approve tx: {}", approve_hash);
            sleep(Duration::from_secs(3)).await;
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

    let tx_hash = onchainos::extract_tx_hash(&result)?;
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
