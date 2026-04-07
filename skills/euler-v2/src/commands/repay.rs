use crate::config::{get_chain_config, get_known_vault};
use crate::onchainos;
use crate::rpc;

/// Repay borrowed assets in an Euler V2 EVault (DRY-RUN ONLY).
///
/// Steps (simulated):
///   1. ERC-20 approve(vault, amount) on underlying token
///   2. EVault.repay(amount, receiver)  selector: 0xacb70815
///
/// This command is dry-run only to prevent accidental on-chain execution.
pub async fn run(
    vault_input: &str,
    amount: Option<&str>,
    all: bool,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    if !dry_run {
        anyhow::bail!(
            "repay is dry-run only. Re-run with --dry-run to see calldata. \
             Use carefully to avoid over-repayment."
        );
    }

    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;

    let (vault_addr, underlying_addr, decimals) = resolve_vault(vault_input, chain_id, rpc).await?;

    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, dry_run)?
    };

    let asset_symbol = rpc::erc20_symbol(&underlying_addr, rpc).await.unwrap_or_else(|_| "TOKEN".to_string());
    let wallet_clean = wallet.trim_start_matches("0x").to_lowercase();

    // Fetch current debt
    let current_debt = rpc::vault_debt_of(&vault_addr, &wallet, rpc).await.unwrap_or(0);

    let (raw_amount, amount_display) = if all {
        // repay type(uint256).max to repay all debt
        let max_u128 = u128::MAX;
        (max_u128, "ALL (type(uint256).max)".to_string())
    } else {
        let amt_str = amount.ok_or_else(|| anyhow::anyhow!("Provide --amount <n> or use --all"))?;
        let raw = rpc::parse_amount(amt_str, decimals)?;
        (raw, amt_str.to_string())
    };

    let vault_clean = vault_addr.trim_start_matches("0x").to_lowercase();

    // approve(vault, raw_amount)  selector: 0x095ea7b3
    let approve_calldata = format!(
        "0x095ea7b3{:0>64}{:064x}",
        vault_clean, raw_amount
    );

    // repay(uint256 amount, address receiver)  selector: 0xacb70815
    let repay_calldata = format!(
        "0xacb70815{:064x}{:0>64}",
        raw_amount, wallet_clean
    );

    let output = serde_json::json!({
        "ok": true,
        "operation": "repay",
        "dryRun": true,
        "warning": "REPAY IS DRY-RUN ONLY.",
        "vault": vault_addr,
        "underlying": asset_symbol,
        "underlyingAddress": underlying_addr,
        "amount": amount_display,
        "rawAmount": raw_amount.to_string(),
        "currentDebt": current_debt.to_string(),
        "currentDebtFormatted": rpc::format_amount(current_debt, decimals),
        "wallet": wallet,
        "chain": cfg.name,
        "chainId": chain_id,
        "simulatedCalldata": {
            "step1_approve": {
                "to": underlying_addr,
                "data": approve_calldata,
                "description": format!("ERC20.approve({}, {})", vault_addr, raw_amount)
            },
            "step2_repay": {
                "to": vault_addr,
                "data": repay_calldata,
                "description": format!("EVault.repay({}, {})", raw_amount, wallet)
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
