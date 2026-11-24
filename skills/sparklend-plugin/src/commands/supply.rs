use anyhow::Context;
use serde_json::{json, Value};

use crate::calldata;
use crate::config;
use crate::onchainos;
use crate::rpc;

/// Supply assets to SparkLend Pool via direct contract-call.
///
/// Flow:
/// 1. Resolve token contract address (symbol → address via onchainos token search)
/// 2. Resolve Pool address via PoolAddressesProvider
/// 3. If supplying WETH and wallet has insufficient WETH, auto-wrap ETH
/// 4. Approve token to Pool (ERC-20 approve)
/// 5. Call Pool.supply(asset, amount, onBehalfOf, 0)
pub async fn run(
    chain_id: u64,
    asset: &str,
    amount: f64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if amount <= 0.0 {
        anyhow::bail!("--amount must be greater than 0");
    }

    let from_addr = resolve_from(from, chain_id)?;

    // Resolve token address and decimals
    let (token_addr, decimals) = onchainos::resolve_token(asset, chain_id)
        .with_context(|| format!("Could not resolve token address for '{}'", asset))?;

    let amount_minimal = human_to_minimal(amount, decimals as u64);
    let amount_display = format!("{:.6}", amount);

    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(config::POOL_ADDRESSES_PROVIDER, config::RPC_URL)
        .await
        .context("Failed to resolve SparkLend Pool address")?;

    // Pre-flight: if supplying WETH and wallet has insufficient WETH, auto-wrap ETH.
    // WETH.deposit{value: needed}() selector: 0xd0e30db0
    let is_weth = config::WETH_ADDRESS.to_lowercase() == token_addr.to_lowercase();
    let mut wrap_tx: Option<String> = None;

    if is_weth {
        let weth_balance = rpc::get_erc20_balance(&token_addr, &from_addr, config::RPC_URL)
            .await
            .context("Failed to fetch WETH balance")?;
        if weth_balance < amount_minimal {
            let needed = amount_minimal - weth_balance;
            let eth_balance = rpc::get_eth_balance(&from_addr, config::RPC_URL)
                .await
                .context("Failed to fetch ETH balance")?;
            if eth_balance < needed {
                anyhow::bail!(
                    "Insufficient balance: need {:.6} WETH to supply, have {:.6} WETH and {:.6} ETH. \
                     Add more ETH or WETH to your wallet.",
                    amount_minimal as f64 / 1e18,
                    weth_balance as f64 / 1e18,
                    eth_balance as f64 / 1e18,
                );
            }
            if dry_run {
                let wrap_cmd = format!(
                    "onchainos wallet contract-call --chain {} --to {} --input-data 0xd0e30db0 --amt {} --from {}",
                    chain_id, token_addr, needed, from_addr
                );
                eprintln!("[dry-run] step 0 wrap ETH→WETH: {}", wrap_cmd);
            } else {
                let wrap_result = onchainos::wallet_contract_call_with_value(
                    chain_id,
                    &token_addr,
                    "0xd0e30db0",
                    Some(&from_addr),
                    needed,
                    false,
                )
                .context("WETH.deposit() (ETH→WETH wrap) failed")?;
                let tx = wrap_result["data"]["txHash"]
                    .as_str()
                    .or_else(|| wrap_result["txHash"].as_str())
                    .or_else(|| wrap_result["hash"].as_str())
                    .unwrap_or("pending")
                    .to_string();
                if tx == "pending" || !tx.starts_with("0x") {
                    anyhow::bail!(
                        "WETH wrap tx was not broadcast (tx hash: '{}'). Check wallet connection and retry.",
                        tx
                    );
                }
                rpc::wait_for_tx(config::RPC_URL, &tx)
                    .await
                    .context("WETH wrap tx did not confirm in time")?;
                wrap_tx = Some(tx);
            }
        }
    } else {
        // Non-WETH: check ERC-20 balance before attempting supply
        let token_balance = rpc::get_erc20_balance(&token_addr, &from_addr, config::RPC_URL)
            .await
            .context("Failed to fetch token balance")?;
        if token_balance < amount_minimal && !dry_run {
            anyhow::bail!(
                "Insufficient {} balance: need {:.6}, have {:.6}. Add funds to your wallet before supplying.",
                asset,
                amount_minimal as f64 / 10f64.powi(decimals as i32),
                token_balance as f64 / 10f64.powi(decimals as i32),
            );
        }
    }

    if dry_run {
        let approve_calldata = calldata::encode_erc20_approve(&pool_addr, amount_minimal)
            .context("Failed to encode approve calldata")?;
        let supply_calldata = calldata::encode_supply(&token_addr, amount_minimal, &from_addr)
            .context("Failed to encode supply calldata")?;
        let approve_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, token_addr, approve_calldata, from_addr
        );
        let supply_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, pool_addr, supply_calldata, from_addr
        );
        eprintln!("[dry-run] step 1 approve: {}", approve_cmd);
        eprintln!("[dry-run] step 2 supply: {}", supply_cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "asset": asset,
            "tokenAddress": token_addr,
            "amount": amount,
            "amountDisplay": amount_display,
            "amountMinimal": amount_minimal.to_string(),
            "poolAddress": pool_addr,
            "chain": config::CHAIN_NAME,
            "steps": [
                {"step": 1, "action": "approve", "simulatedCommand": approve_cmd},
                {"step": 2, "action": "supply",  "simulatedCommand": supply_cmd}
            ]
        }));
    }

    // Step 1: approve
    let approve_calldata = calldata::encode_erc20_approve(&pool_addr, amount_minimal)
        .context("Failed to encode approve calldata")?;
    let approve_result = onchainos::wallet_contract_call(
        chain_id,
        &token_addr,
        &approve_calldata,
        Some(&from_addr),
        false,
    )
    .context("ERC-20 approve failed")?;
    let approve_tx = approve_result["data"]["txHash"]
        .as_str()
        .or_else(|| approve_result["txHash"].as_str())
        .or_else(|| approve_result["hash"].as_str())
        .unwrap_or("pending")
        .to_string();

    if approve_tx == "pending" || !approve_tx.starts_with("0x") {
        anyhow::bail!(
            "Approve tx was not broadcast (tx hash: '{}'). Check wallet connection and retry.",
            approve_tx
        );
    }
    rpc::wait_for_tx(config::RPC_URL, &approve_tx)
        .await
        .context("Approve tx did not confirm in time")?;

    // Step 2: supply
    let supply_calldata = calldata::encode_supply(&token_addr, amount_minimal, &from_addr)
        .context("Failed to encode supply calldata")?;
    let supply_result = onchainos::wallet_contract_call(
        chain_id,
        &pool_addr,
        &supply_calldata,
        Some(&from_addr),
        false,
    )
    .context("Pool.supply() failed")?;
    let supply_tx = supply_result["data"]["txHash"]
        .as_str()
        .or_else(|| supply_result["txHash"].as_str())
        .or_else(|| supply_result["hash"].as_str())
        .unwrap_or("pending");

    Ok(json!({
        "ok": true,
        "asset": asset,
        "tokenAddress": token_addr,
        "amount": amount,
        "amountDisplay": amount_display,
        "amountMinimal": amount_minimal.to_string(),
        "poolAddress": pool_addr,
        "chain": config::CHAIN_NAME,
        "wrapTxHash": wrap_tx,
        "approveTxHash": approve_tx,
        "supplyTxHash": supply_tx.to_string(),
        "explorer": format!("https://etherscan.io/tx/{}", supply_tx),
        "dryRun": false
    }))
}

fn resolve_from(from: Option<&str>, chain_id: u64) -> anyhow::Result<String> {
    if let Some(addr) = from {
        return Ok(addr.to_string());
    }
    onchainos::wallet_address(chain_id).context("No --from address and could not resolve active wallet.")
}

pub fn human_to_minimal(amount: f64, decimals: u64) -> u128 {
    let factor = 10u128.pow(decimals as u32);
    (amount * factor as f64) as u128
}
