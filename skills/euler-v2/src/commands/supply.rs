use crate::config::{get_chain_config, get_known_vault};
use crate::onchainos;
use crate::rpc;

/// Supply (deposit) underlying assets into an Euler V2 EVault (ERC-4626).
///
/// Steps:
///   1. ERC-20 approve(vault, amount) on underlying token
///   2. EVault.deposit(amount, receiver)  selector: 0x6e553f65
///
/// CONFIRM: This is an on-chain write operation. Review the amounts before submitting.
pub async fn run(
    vault_input: &str,
    amount: &str,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;

    // Resolve vault address and underlying asset
    let (vault_addr, underlying_addr, decimals) = resolve_vault(vault_input, chain_id, rpc).await?;

    let raw_amount = rpc::parse_amount(amount, decimals)?;

    // Resolve wallet
    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, dry_run)?
    };

    let asset_symbol = rpc::erc20_symbol(&underlying_addr, rpc).await.unwrap_or_else(|_| "TOKEN".to_string());

    // Check user balance
    let user_balance = rpc::erc20_balance_of(&underlying_addr, &wallet, rpc).await.unwrap_or(0);
    if !dry_run && user_balance < raw_amount {
        anyhow::bail!(
            "Insufficient {} balance. Have: {}, Need: {}",
            asset_symbol,
            rpc::format_amount(user_balance, decimals),
            amount
        );
    }

    eprintln!(
        "[euler-v2] Supplying {} {} to vault {} on {}",
        amount, asset_symbol, vault_addr, cfg.name
    );

    // Step 1: approve vault to spend underlying
    let approve_calldata = format!(
        "0x095ea7b3{:0>64}{:064x}",
        vault_addr.trim_start_matches("0x").to_lowercase(),
        raw_amount
    );
    eprintln!("[euler-v2] Step 1/2: Approving vault to spend {} {}...", amount, asset_symbol);
    let approve_result = onchainos::wallet_contract_call(
        chain_id, &underlying_addr, &approve_calldata, from, None, dry_run
    ).await?;
    let approve_tx = onchainos::extract_tx_hash(&approve_result).to_string();

    // Wait for approve to confirm
    if !dry_run {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    // Step 2: deposit(uint256 assets, address receiver)  selector: 0x6e553f65
    let wallet_clean = wallet.trim_start_matches("0x").to_lowercase();
    let deposit_calldata = format!(
        "0x6e553f65{:064x}{:0>64}",
        raw_amount, wallet_clean
    );
    eprintln!("[euler-v2] Step 2/2: Depositing {} {} into EVault {}...", amount, asset_symbol, vault_addr);
    let deposit_result = onchainos::wallet_contract_call(
        chain_id, &vault_addr, &deposit_calldata, from, None, dry_run
    ).await?;
    let deposit_tx = onchainos::extract_tx_hash(&deposit_result).to_string();

    let output = serde_json::json!({
        "ok": true,
        "operation": "supply",
        "vault": vault_addr,
        "underlying": asset_symbol,
        "underlyingAddress": underlying_addr,
        "amount": amount,
        "rawAmount": raw_amount.to_string(),
        "receiver": wallet,
        "chain": cfg.name,
        "chainId": chain_id,
        "dryRun": dry_run,
        "approveTxHash": approve_tx,
        "supplyTxHash": deposit_tx,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Resolve vault address, underlying address, and decimals from user input.
/// Accepts vault address (0x...) or known asset symbol (USDC, WETH, ...).
async fn resolve_vault(input: &str, chain_id: u64, rpc: &str) -> anyhow::Result<(String, String, u8)> {
    if input.starts_with("0x") && input.len() == 42 {
        let vault = input.to_lowercase();
        let underlying = rpc::vault_asset(&vault, rpc).await?;
        let decimals = rpc::erc20_decimals(&underlying, rpc).await.unwrap_or(18);
        Ok((vault, underlying, decimals))
    } else if let Some((vault, underlying, decimals)) = get_known_vault(input, chain_id) {
        Ok((vault.to_string(), underlying.to_string(), decimals))
    } else {
        anyhow::bail!(
            "Unknown vault '{}'. Use a vault address (0x...) or symbol (USDC, WETH, CBBTC). \
             Run 'euler-v2 --chain {} markets' to list available vaults.",
            input, chain_id
        )
    }
}
