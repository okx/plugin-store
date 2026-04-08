use crate::calldata;
use crate::config::{get_chain_config, get_ftoken_info};
use crate::onchainos;
use crate::rpc;

/// Supply underlying assets to a Fluid fToken (ERC-4626 deposit).
/// Steps: 1) approve fToken to spend underlying, 2) deposit to fToken
pub async fn run(
    ftoken: &str,
    amount: &str,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;

    // Resolve fToken address and underlying info
    let (ftoken_addr, underlying_addr, decimals) = if ftoken.starts_with("0x") && ftoken.len() == 42 {
        // Direct address — lookup decimals from chain
        let ftoken_lower = ftoken.to_lowercase();
        let decimals = rpc::erc20_decimals(&ftoken_lower, cfg.rpc_url).await.unwrap_or(18);
        // For direct address, we need to get the underlying from the fToken
        // Use asset() call: selector 0x38d52e0f
        let asset_hex = rpc::eth_call(&ftoken_lower, "0x38d52e0f", cfg.rpc_url).await?;
        let underlying = extract_address_from_hex(&asset_hex)?;
        (ftoken_lower, underlying, decimals)
    } else {
        let (fa, ua, dec) = get_ftoken_info(ftoken, chain_id)?;
        (fa.to_string(), ua.to_string(), dec)
    };

    let raw_amount = calldata::parse_amount(amount, decimals)?;

    // Resolve wallet address
    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, dry_run)?
    };

    let symbol = rpc::erc20_symbol(&underlying_addr, cfg.rpc_url)
        .await
        .unwrap_or_else(|_| "TOKEN".to_string());

    // Step 1: Approve fToken to spend underlying asset
    let approve_calldata = calldata::encode_approve(&ftoken_addr, raw_amount);
    eprintln!("[fluid] Step 1/2: Approving fToken {} to spend {} {}...", ftoken_addr, amount, symbol);
    if dry_run {
        eprintln!("[fluid] [dry-run] Would approve: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, underlying_addr, approve_calldata);
    }
    let approve_result = onchainos::wallet_contract_call(
        chain_id, &underlying_addr, &approve_calldata, from, None, dry_run
    ).await?;
    let approve_tx = onchainos::extract_tx_hash(&approve_result).to_string();

    // Wait for approve tx to land before deposit
    if !dry_run {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 2: Deposit to fToken
    let deposit_calldata = calldata::encode_ftoken_deposit(raw_amount, &wallet);
    eprintln!("[fluid] Step 2/2: Depositing {} {} into fToken {}...", amount, symbol, ftoken_addr);
    if dry_run {
        eprintln!("[fluid] [dry-run] Would deposit: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, ftoken_addr, deposit_calldata);
    }
    let deposit_result = onchainos::wallet_contract_call(
        chain_id, &ftoken_addr, &deposit_calldata, from, None, dry_run
    ).await?;
    let deposit_tx = onchainos::extract_tx_hash(&deposit_result).to_string();

    let output = serde_json::json!({
        "ok": true,
        "operation": "supply",
        "fToken": ftoken_addr,
        "underlying": symbol,
        "underlyingAddress": underlying_addr,
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

/// Extract a 20-byte address from a 32-byte ABI-encoded slot (right-aligned address).
fn extract_address_from_hex(hex: &str) -> anyhow::Result<String> {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 40 {
        anyhow::bail!("Hex too short to contain address");
    }
    // Last 40 hex chars = 20 bytes address
    let addr = &clean[clean.len() - 40..];
    Ok(format!("0x{}", addr))
}
