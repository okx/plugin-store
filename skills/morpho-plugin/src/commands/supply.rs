use anyhow::Context;
use crate::api;
use crate::calldata;
use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Supply assets to a MetaMorpho vault (ERC-4626 deposit).
pub async fn run(
    vault: &str,
    asset: &str,
    amount: &str,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
    confirm: bool,
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;

    // Resolve vault asset address and decimals
    let asset_addr = resolve_asset_address(asset, chain_id)?;
    let decimals = rpc::erc20_decimals(&asset_addr, cfg.rpc_url).await.unwrap_or(18);
    let symbol = rpc::erc20_symbol(&asset_addr, cfg.rpc_url).await.unwrap_or_else(|_| "TOKEN".to_string());

    let raw_amount = calldata::parse_amount(amount, decimals)
        .context("Failed to parse amount")?;

    // Resolve the caller's wallet address (used as receiver in deposit)
    let wallet_addr = onchainos::resolve_wallet(from, chain_id).await?;

    // Pre-flight: balance check and auto-wrap ETH→WETH if needed
    let is_weth = weth_address(chain_id)
        .map_or(false, |w| w.eq_ignore_ascii_case(&asset_addr));
    let mut wrap_tx: Option<String> = None;

    if !dry_run {
        if is_weth {
            let weth_balance = rpc::erc20_balance_of(&asset_addr, &wallet_addr, cfg.rpc_url)
                .await
                .context("Failed to fetch WETH balance")?;
            if weth_balance < raw_amount {
                let needed = raw_amount - weth_balance;
                let eth_bal = rpc::eth_balance(&wallet_addr, cfg.rpc_url)
                    .await
                    .context("Failed to fetch ETH balance")?;
                if eth_bal < needed {
                    anyhow::bail!(
                        "Insufficient balance: need {:.6} WETH to deposit, \
                         have {:.6} WETH and {:.6} ETH. \
                         Add more ETH or WETH to your wallet.",
                        raw_amount as f64 / 1e18,
                        weth_balance as f64 / 1e18,
                        eth_bal as f64 / 1e18,
                    );
                }
                // Auto-wrap ETH → WETH using WETH.deposit()
                eprintln!("[morpho] Wrapping {:.6} ETH → WETH (WETH balance insufficient)...",
                    needed as f64 / 1e18);
                let wrap_result = onchainos::wallet_contract_call(
                    chain_id,
                    &asset_addr,
                    "0xd0e30db0", // WETH.deposit() selector
                    Some(wallet_addr.as_str()),
                    Some(needed),
                    false,
                    false,
                ).await?;
                let wrap_hash = onchainos::extract_tx_hash_or_err(&wrap_result)?;
                eprintln!("[morpho] Wrap tx: {} — waiting for confirmation...", wrap_hash);
                onchainos::wait_for_tx(&wrap_hash, cfg.rpc_url, chain_id).await
                    .context("WETH wrap tx did not confirm in time")?;
                wrap_tx = Some(wrap_hash);
            }
        } else {
            // Non-WETH: check ERC-20 balance before proceeding
            let token_balance = rpc::erc20_balance_of(&asset_addr, &wallet_addr, cfg.rpc_url)
                .await
                .context("Failed to fetch token balance")?;
            if token_balance < raw_amount {
                anyhow::bail!(
                    "Insufficient {} balance: need {}, have {}. Add funds to your wallet.",
                    symbol,
                    calldata::format_amount(raw_amount, decimals),
                    calldata::format_amount(token_balance, decimals),
                );
            }
        }
    }

    // Build calldatas (needed for both preview and execution)
    let approve_calldata = calldata::encode_approve(vault, raw_amount);
    let deposit_calldata = calldata::encode_vault_deposit(raw_amount, &wallet_addr);

    // Confirm gate: show preview and exit if --confirm not given
    if !dry_run && !confirm {
        let preview = serde_json::json!({
            "ok": true,
            "preview": true,
            "operation": "supply",
            "vault": vault,
            "asset": symbol,
            "assetAddress": asset_addr,
            "amount": amount,
            "rawAmount": raw_amount.to_string(),
            "chainId": chain_id,
            "pendingTransactions": 2,
            "transactions": [
                {"step": 1, "description": format!("Approve {} to spend {} {}", vault, amount, symbol), "to": asset_addr},
                {"step": 2, "description": format!("Deposit {} {} into vault {}", amount, symbol, vault), "to": vault},
            ],
            "note": "Re-run with --confirm to execute these transactions on-chain. If depositing WETH and wallet only holds ETH, a wrap step will be added automatically."
        });
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    // Step 1: Approve vault to spend asset
    let step = if wrap_tx.is_some() { "2/3" } else { "1/2" };
    eprintln!("[morpho] Step {}: Approving {} to spend {} {}...", step, vault, amount, symbol);
    if dry_run {
        eprintln!("[morpho] [dry-run] Would approve: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, asset_addr, approve_calldata);
    }
    let approve_result = onchainos::wallet_contract_call(chain_id, &asset_addr, &approve_calldata, Some(wallet_addr.as_str()), None, dry_run, true).await?;
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)?;
    onchainos::wait_for_tx(&approve_tx, cfg.rpc_url, chain_id).await?;

    // Step 2: Deposit to vault
    let step = if wrap_tx.is_some() { "3/3" } else { "2/2" };
    eprintln!("[morpho] Step {}: Depositing {} {} into vault {}...", step, amount, symbol, vault);
    if dry_run {
        eprintln!("[morpho] [dry-run] Would deposit: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, vault, deposit_calldata);
    }
    let deposit_result = onchainos::wallet_contract_call(chain_id, vault, &deposit_calldata, Some(wallet_addr.as_str()), None, dry_run, false).await?;
    let deposit_tx = onchainos::extract_tx_hash_or_err(&deposit_result)?;

    let output = serde_json::json!({
        "ok": true,
        "operation": "supply",
        "vault": vault,
        "asset": symbol,
        "assetAddress": asset_addr,
        "amount": amount,
        "rawAmount": raw_amount.to_string(),
        "chainId": chain_id,
        "dryRun": dry_run,
        "wrapTxHash": wrap_tx,
        "approveTxHash": approve_tx,
        "supplyTxHash": deposit_tx,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Return the WETH contract address for known chains, or None.
fn weth_address(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        1     => Some("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"),
        8453  => Some("0x4200000000000000000000000000000000000006"),
        42161 => Some("0x82af49447d8a07e3bd95bd0d56f35241523fbab1"), // Arbitrum
        10    => Some("0x4200000000000000000000000000000000000006"), // Optimism
        _ => None,
    }
}

/// Resolve asset symbol or address to a checksummed address.
fn resolve_asset_address(asset: &str, chain_id: u64) -> anyhow::Result<String> {
    if asset.starts_with("0x") && asset.len() == 42 {
        return Ok(asset.to_lowercase());
    }
    // Well-known token symbols
    let addr = match (chain_id, asset.to_uppercase().as_str()) {
        (1, "WETH") => "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
        (1, "USDC") => "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        (1, "USDT") => "0xdac17f958d2ee523a2206206994597c13d831ec7",
        (1, "DAI") => "0x6b175474e89094c44da98b954eedeac495271d0f",
        (1, "WSTETH") => "0x7f39c581f595b53c5cb19bd0b3f8da6c935e2ca0",
        (8453, "WETH") => "0x4200000000000000000000000000000000000006",
        (8453, "USDC") => "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        (8453, "CBETH") => "0x2ae3f1ec7f1f5012cfeab0185bfc7aa3cf0dec22",
        (8453, "CBBTC") => "0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf",
        _ => anyhow::bail!("Unknown asset symbol '{}' on chain {}. Please provide the token address.", asset, chain_id),
    };
    Ok(addr.to_string())
}
