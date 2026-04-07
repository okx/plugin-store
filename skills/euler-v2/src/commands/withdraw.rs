use crate::config::{get_chain_config, get_known_vault};
use crate::onchainos;
use crate::rpc;

/// Withdraw underlying assets from an Euler V2 EVault.
///
/// If `--all` is set: EVault.redeem(shares, receiver, owner)  selector: 0xba087652
/// Otherwise:         EVault.withdraw(assets, receiver, owner) selector: 0xb460af94
///
/// CONFIRM: This is an on-chain write operation. Review the amounts before submitting.
pub async fn run(
    vault_input: &str,
    amount: Option<&str>,
    all: bool,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;

    // Resolve vault
    let (vault_addr, underlying_addr, decimals) = resolve_vault(vault_input, chain_id, rpc).await?;

    // Resolve wallet
    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, dry_run)?
    };

    let asset_symbol = rpc::erc20_symbol(&underlying_addr, rpc).await.unwrap_or_else(|_| "TOKEN".to_string());
    let wallet_clean = wallet.trim_start_matches("0x").to_lowercase();

    if all {
        // redeem all shares
        let shares = rpc::vault_balance_of(&vault_addr, &wallet, rpc).await.unwrap_or(0);
        if shares == 0 && !dry_run {
            anyhow::bail!("No shares to redeem in vault {}", vault_addr);
        }

        let supplied_assets = rpc::vault_convert_to_assets(&vault_addr, shares, rpc).await.unwrap_or(shares);
        let display_amount = rpc::format_amount(supplied_assets, decimals);

        eprintln!(
            "[euler-v2] Withdrawing all ({} shares ≈ {} {}) from vault {} on {}",
            shares, display_amount, asset_symbol, vault_addr, cfg.name
        );

        // redeem(uint256 shares, address receiver, address owner)  selector: 0xba087652
        let calldata = format!(
            "0xba087652{:064x}{:0>64}{:0>64}",
            shares, wallet_clean, wallet_clean
        );
        let result = onchainos::wallet_contract_call(
            chain_id, &vault_addr, &calldata, from, None, dry_run
        ).await?;
        let tx = onchainos::extract_tx_hash(&result).to_string();

        let output = serde_json::json!({
            "ok": true,
            "operation": "withdraw",
            "mode": "redeem_all",
            "vault": vault_addr,
            "underlying": asset_symbol,
            "underlyingAddress": underlying_addr,
            "shares": shares.to_string(),
            "estimatedAssets": display_amount,
            "receiver": wallet,
            "chain": cfg.name,
            "chainId": chain_id,
            "dryRun": dry_run,
            "txHash": tx,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let amt_str = amount.ok_or_else(|| anyhow::anyhow!("Provide --amount <n> or use --all"))?;
        let raw_amount = rpc::parse_amount(amt_str, decimals)?;

        eprintln!(
            "[euler-v2] Withdrawing {} {} from vault {} on {}",
            amt_str, asset_symbol, vault_addr, cfg.name
        );

        // withdraw(uint256 assets, address receiver, address owner)  selector: 0xb460af94
        let calldata = format!(
            "0xb460af94{:064x}{:0>64}{:0>64}",
            raw_amount, wallet_clean, wallet_clean
        );
        let result = onchainos::wallet_contract_call(
            chain_id, &vault_addr, &calldata, from, None, dry_run
        ).await?;
        let tx = onchainos::extract_tx_hash(&result).to_string();

        let output = serde_json::json!({
            "ok": true,
            "operation": "withdraw",
            "mode": "withdraw_exact",
            "vault": vault_addr,
            "underlying": asset_symbol,
            "underlyingAddress": underlying_addr,
            "amount": amt_str,
            "rawAmount": raw_amount.to_string(),
            "receiver": wallet,
            "chain": cfg.name,
            "chainId": chain_id,
            "dryRun": dry_run,
            "txHash": tx,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    }

    Ok(())
}

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
            "Unknown vault '{}'. Use a vault address (0x...) or symbol (USDC, WETH). \
             Run 'euler-v2 --chain {} markets' to list available vaults.",
            input, chain_id
        )
    }
}
