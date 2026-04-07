use anyhow::Context;
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
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;

    // Resolve vault asset address and decimals
    let asset_addr = resolve_asset_address(asset)?;
    let decimals = rpc::erc20_decimals(&asset_addr, cfg.rpc_url).await.unwrap_or(18);
    let symbol = rpc::erc20_symbol(&asset_addr, cfg.rpc_url).await.unwrap_or_else(|_| "TOKEN".to_string());

    let raw_amount = calldata::parse_amount(amount, decimals)
        .context("Failed to parse amount")?;

    // Resolve the caller's wallet address (used as receiver in deposit)
    let wallet_addr = onchainos::resolve_wallet(from, chain_id).await?;

    // Step 1: Approve vault to spend asset (ask user to confirm before executing)
    let approve_calldata = calldata::encode_approve(vault, raw_amount);
    eprintln!("[morpho] Step 1/2: Approving {} to spend {} {}...", vault, amount, symbol);
    if dry_run {
        eprintln!("[morpho] [dry-run] Would approve: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, asset_addr, approve_calldata);
    }
    let approve_result = onchainos::wallet_contract_call(chain_id, &asset_addr, &approve_calldata, from, None, dry_run).await?;
    let approve_tx = onchainos::extract_tx_hash(&approve_result);

    // Wait for approve tx to be picked up before sending deposit, to avoid nonce conflicts.
    if !dry_run {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 2: Deposit to vault (ask user to confirm before executing)
    let deposit_calldata = calldata::encode_vault_deposit(raw_amount, &wallet_addr);
    eprintln!("[morpho] Step 2/2: Depositing {} {} into vault {}...", amount, symbol, vault);
    if dry_run {
        eprintln!("[morpho] [dry-run] Would deposit: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, vault, deposit_calldata);
    }
    let deposit_result = onchainos::wallet_contract_call(chain_id, vault, &deposit_calldata, from, None, dry_run).await?;
    let deposit_tx = onchainos::extract_tx_hash(&deposit_result);

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

/// Resolve asset symbol or address to a checksummed address (Base only).
fn resolve_asset_address(asset: &str) -> anyhow::Result<String> {
    if asset.starts_with("0x") && asset.len() == 42 {
        return Ok(asset.to_lowercase());
    }
    // Well-known token symbols on Base (8453)
    let addr = match asset.to_uppercase().as_str() {
        "WETH" => "0x4200000000000000000000000000000000000006",
        "USDC" => "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        "CBETH" => "0x2ae3f1ec7f1f5012cfeab0185bfc7aa3cf0dec22",
        "CBBTC" => "0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf",
        _ => anyhow::bail!("Unknown asset symbol '{}' on Base. Please provide the ERC-20 token address.", asset),
    };
    Ok(addr.to_string())
}
