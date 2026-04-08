use crate::config::{get_chain_config, get_known_vault};
use crate::onchainos;
use crate::rpc;

/// Borrow assets from an Euler V2 EVault (DRY-RUN ONLY).
///
/// IMPORTANT: Borrowing requires EVC collateral setup (enableCollateral + enableController)
/// which is NOT handled here. This command only simulates the borrow calldata.
///
/// Real borrowing requires:
///   1. evc.enableCollateral(account, collateralVault)
///   2. evc.enableController(account, borrowVault)
///   3. EVault.borrow(amount, receiver)  selector: 0x4b3fd148
///
/// This command is dry-run only to prevent liquidation risk.
pub async fn run(
    vault_input: &str,
    amount: &str,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    if !dry_run {
        anyhow::bail!(
            "borrow is dry-run only. Re-run with --dry-run to see calldata. \
             Borrowing without proper EVC collateral setup will revert on-chain."
        );
    }

    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;

    let (vault_addr, underlying_addr, decimals) = resolve_vault(vault_input, chain_id, rpc).await?;
    let raw_amount = rpc::parse_amount(amount, decimals)?;

    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, dry_run)?
    };

    let asset_symbol = rpc::erc20_symbol(&underlying_addr, rpc).await.unwrap_or_else(|_| "TOKEN".to_string());
    let wallet_clean = wallet.trim_start_matches("0x").to_lowercase();

    // borrow(uint256 amount, address receiver)  selector: 0x4b3fd148
    let borrow_calldata = format!(
        "0x4b3fd148{:064x}{:0>64}",
        raw_amount, wallet_clean
    );

    // evc.enableCollateral(account, collateral) selector: 0xb9b2aa44
    let evc_enable_collateral = format!(
        "0xb9b2aa44{:0>64}{}",
        wallet_clean,
        vault_addr.trim_start_matches("0x").to_lowercase()
    );

    // evc.enableController(account, controller) selector: 0x04e5d38d
    let evc_enable_controller = format!(
        "0x04e5d38d{:0>64}{}",
        wallet_clean,
        vault_addr.trim_start_matches("0x").to_lowercase()
    );

    let output = serde_json::json!({
        "ok": true,
        "operation": "borrow",
        "dryRun": true,
        "warning": "BORROW IS DRY-RUN ONLY. Liquidation risk. Requires EVC collateral setup first.",
        "vault": vault_addr,
        "underlying": asset_symbol,
        "underlyingAddress": underlying_addr,
        "amount": amount,
        "rawAmount": raw_amount.to_string(),
        "receiver": wallet,
        "chain": cfg.name,
        "chainId": chain_id,
        "evc": cfg.evc,
        "simulatedCalldata": {
            "step1_enableCollateral": {
                "to": cfg.evc,
                "data": evc_enable_collateral,
                "description": "evc.enableCollateral(account, collateralVault)"
            },
            "step2_enableController": {
                "to": cfg.evc,
                "data": evc_enable_controller,
                "description": "evc.enableController(account, borrowVault)"
            },
            "step3_borrow": {
                "to": vault_addr,
                "data": borrow_calldata,
                "description": format!("EVault.borrow({}, {})", raw_amount, wallet)
            }
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
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
