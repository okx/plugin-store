use crate::calldata;
use crate::config::{get_chain_config, get_ftoken_info};
use crate::onchainos;
use crate::rpc;

/// Withdraw assets from a Fluid fToken.
/// Partial: withdraw(assets, receiver, owner) — ERC-4626
/// Full (--all): redeem(shares, receiver, owner) — burns all fToken shares
pub async fn run(
    ftoken: &str,
    amount: Option<&str>,
    all: bool,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    if amount.is_none() && !all {
        anyhow::bail!("Must specify --amount <n> or --all");
    }

    let cfg = get_chain_config(chain_id)?;

    // Resolve fToken address and decimals
    let (ftoken_addr, _underlying_addr, decimals) = if ftoken.starts_with("0x") && ftoken.len() == 42 {
        let ftoken_lower = ftoken.to_lowercase();
        let decimals = rpc::erc20_decimals(&ftoken_lower, cfg.rpc_url).await.unwrap_or(18);
        (ftoken_lower, String::new(), decimals)
    } else {
        let (fa, ua, dec) = get_ftoken_info(ftoken, chain_id)?;
        (fa.to_string(), ua.to_string(), dec)
    };

    // Resolve wallet
    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, dry_run)?
    };

    let (calldata_hex, op_label, display_amount) = if all {
        // Full withdrawal: redeem all shares
        let shares = rpc::ftoken_share_balance(&ftoken_addr, &wallet, cfg.rpc_url).await?;
        if shares == 0 {
            anyhow::bail!("No fToken shares found for address {} in {}", wallet, ftoken_addr);
        }
        let underlying = rpc::ftoken_convert_to_assets(&ftoken_addr, shares, cfg.rpc_url).await?;
        let display = calldata::format_amount(underlying, decimals);
        eprintln!("[fluid] Redeeming {} shares (~{} underlying) from {}...", shares, display, ftoken_addr);
        let cd = calldata::encode_ftoken_redeem(shares, &wallet, &wallet);
        (cd, "redeem", display)
    } else {
        let amt_str = amount.unwrap();
        let raw_amount = calldata::parse_amount(amt_str, decimals)?;
        eprintln!("[fluid] Withdrawing {} from {}...", amt_str, ftoken_addr);
        let cd = calldata::encode_ftoken_withdraw(raw_amount, &wallet, &wallet);
        (cd, "withdraw", amt_str.to_string())
    };

    if dry_run {
        eprintln!("[fluid] [dry-run] Would {}: onchainos wallet contract-call --chain {} --to {} --input-data {}", op_label, chain_id, ftoken_addr, calldata_hex);
    }

    let result = onchainos::wallet_contract_call(
        chain_id, &ftoken_addr, &calldata_hex, from, None, dry_run
    ).await?;
    let tx_hash = onchainos::extract_tx_hash(&result).to_string();

    let output = serde_json::json!({
        "ok": true,
        "operation": op_label,
        "fToken": ftoken_addr,
        "amount": display_amount,
        "chainId": chain_id,
        "dryRun": dry_run,
        "txHash": tx_hash,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
