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
            "note": "Re-run with --confirm to execute these transactions on-chain."
        });
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    // Step 1: Approve vault to spend asset
    eprintln!("[morpho] Step 1/2: Approving {} to spend {} {}...", vault, amount, symbol);
    if dry_run {
        eprintln!("[morpho] [dry-run] Would approve: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, asset_addr, approve_calldata);
    }
    let approve_result = onchainos::wallet_contract_call(chain_id, &asset_addr, &approve_calldata, from, None, dry_run, true).await?;
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)?;
    onchainos::wait_for_tx(&approve_tx, cfg.rpc_url, chain_id).await?;

    // Step 2: Deposit to vault
    eprintln!("[morpho] Step 2/2: Depositing {} {} into vault {}...", amount, symbol, vault);
    if dry_run {
        eprintln!("[morpho] [dry-run] Would deposit: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, vault, deposit_calldata);
    }
    let deposit_result = onchainos::wallet_contract_call(chain_id, vault, &deposit_calldata, from, None, dry_run, false).await?;
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
        "approveTxHash": approve_tx,
        "supplyTxHash": deposit_tx,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
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
