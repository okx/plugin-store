use anyhow::Context;
use crate::calldata;
use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Withdraw from a MetaMorpho vault (ERC-4626).
/// If `amount` is Some, does a partial withdraw by assets.
/// If `all` is true, redeems all shares.
pub async fn run(
    vault: &str,
    asset: &str,
    amount: Option<&str>,
    all: bool,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    // Resolve the active wallet address (used as owner/receiver)
    let owner_string = onchainos::resolve_wallet(from, chain_id).await?;
    let owner = owner_string.as_str();

    // Resolve asset address and decimals for display
    let asset_addr = resolve_asset_address(asset, chain_id)?;
    let decimals = rpc::erc20_decimals(&asset_addr, cfg.rpc_url).await.unwrap_or(18);
    let symbol = rpc::erc20_symbol(&asset_addr, cfg.rpc_url).await.unwrap_or_else(|_| "TOKEN".to_string());

    let calldata_hex;
    let display_amount;

    if all {
        // Use redeem(shares, receiver, owner) — fetch share balance first
        let shares = rpc::vault_share_balance(vault, owner, cfg.rpc_url).await?;
        let assets = rpc::vault_convert_to_assets(vault, shares, cfg.rpc_url).await?;
        display_amount = calldata::format_amount(assets, decimals);
        calldata_hex = calldata::encode_vault_redeem(shares, owner, owner);
        eprintln!("[morpho] Redeeming all shares ({}) from vault {}...", shares, vault);
    } else {
        let amt_str = amount.context("Must provide --amount or --all")?;
        let raw_amount = calldata::parse_amount(amt_str, decimals)?;
        display_amount = amt_str.to_string();
        calldata_hex = calldata::encode_vault_withdraw(raw_amount, owner, owner);
        eprintln!("[morpho] Withdrawing {} {} from vault {}...", amt_str, symbol, vault);
    }

    if dry_run {
        eprintln!("[morpho] [dry-run] Would call: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, vault, calldata_hex);
    }

    // Ask user to confirm before executing on-chain
    let result = onchainos::wallet_contract_call(chain_id, vault, &calldata_hex, from, None, dry_run, false).await?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    let output = serde_json::json!({
        "ok": true,
        "operation": "withdraw",
        "vault": vault,
        "asset": symbol,
        "assetAddress": asset_addr,
        "amount": display_amount,
        "chainId": chain_id,
        "dryRun": dry_run,
        "txHash": tx_hash,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn resolve_asset_address(asset: &str, chain_id: u64) -> anyhow::Result<String> {
    if asset.starts_with("0x") && asset.len() == 42 {
        return Ok(asset.to_lowercase());
    }
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
