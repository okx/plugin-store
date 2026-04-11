// commands/add_liquidity.rs — Add liquidity to a Curve pool
use crate::{api, config, curve_abi, onchainos, rpc};
use anyhow::{Context, Result};

pub async fn run(
    chain_id: u64,
    pool_address: String,
    amounts: Vec<u128>,
    min_mint: u128,
    wallet: Option<String>,
    dry_run: bool,
) -> Result<()> {
    let chain_name = config::chain_name(chain_id);
    let rpc_url = config::rpc_url(chain_id);

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

    // Fetch pool info to get coin list
    let pools = api::get_all_pools(chain_name).await?;
    let pool = api::find_pool_by_address(&pools, &pool_address);

    let n_coins = match pool {
        Some(p) => p.coins.len(),
        None => amounts.len(), // fallback: infer from amounts length
    };

    if amounts.len() != n_coins {
        anyhow::bail!(
            "Pool has {} coins but {} amounts were provided",
            n_coins,
            amounts.len()
        );
    }

    // Build add_liquidity calldata based on coin count
    let calldata = match n_coins {
        2 => curve_abi::encode_add_liquidity_2([amounts[0], amounts[1]], min_mint),
        3 => curve_abi::encode_add_liquidity_3([amounts[0], amounts[1], amounts[2]], min_mint),
        4 => curve_abi::encode_add_liquidity_4(
            [amounts[0], amounts[1], amounts[2], amounts[3]],
            min_mint,
        ),
        _ => anyhow::bail!("Unsupported pool size: {} coins", n_coins),
    };

    if dry_run {
        let pool_name = pool.map(|p| p.name.as_str()).unwrap_or("unknown");
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain": chain_name,
                "pool_address": pool_address,
                "pool_name": pool_name,
                "amounts_raw": amounts.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
                "min_mint_raw": min_mint.to_string(),
                "calldata": calldata
            })
        );
        return Ok(());
    }

    // Approve each token with a non-zero amount; wait for each approve to confirm before next
    if let Some(p) = pool {
        for (i, coin) in p.coins.iter().enumerate() {
            let amount = amounts[i];
            if amount == 0 {
                continue;
            }
            let allowance = rpc::get_allowance(&coin.address, &wallet_addr, &pool_address, rpc_url)
                .await
                .unwrap_or(0);
            if allowance < amount {
                eprintln!("Approving {} ({}) for pool...", coin.symbol, coin.address);
                let approve_result = onchainos::erc20_approve(
                    chain_id,
                    &coin.address,
                    &pool_address,
                    amount,
                    Some(&wallet_addr),
                    false,
                )
                .await?;
                let ah = onchainos::extract_tx_hash_or_err(&approve_result)?;
                eprintln!("Approve {} tx: {} — waiting for confirmation...", coin.symbol, ah);
                onchainos::wait_for_tx(chain_id, ah.clone(), wallet_addr.clone())
                    .await
                    .with_context(|| format!("Approve {} tx did not confirm in time", coin.symbol))?;
                eprintln!("Approve {} confirmed.", coin.symbol);
            }
        }
    }

    // Execute add_liquidity — requires --force
    let result = onchainos::wallet_contract_call(
        chain_id,
        &pool_address,
        &calldata,
        Some(&wallet_addr),
        None,
        true,  // --force required
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;
    let explorer = config::explorer_url(chain_id, &tx_hash);
    let pool_name = pool.map(|p| p.name.as_str()).unwrap_or("unknown");

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "chain": chain_name,
            "pool_address": pool_address,
            "pool_name": pool_name,
            "amounts_raw": amounts.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
            "min_mint_raw": min_mint.to_string(),
            "tx_hash": tx_hash,
            "explorer": explorer
        })
    );
    Ok(())
}
