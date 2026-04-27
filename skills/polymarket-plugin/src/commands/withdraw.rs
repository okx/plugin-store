/// `polymarket withdraw` — transfer collateral from proxy wallet back to EOA wallet.
///
/// Uses PROXY_FACTORY.proxy([op]) to execute an ERC-20 transfer from the proxy's context.
/// The token depends on the CLOB version:
///   V1 → USDC.e  (legacy exchange)
///   V2 → pUSD    (Polymarket USD, from ~2026-04-28)
///
/// The command auto-detects which token the proxy holds (pUSD balance checked first,
/// fallback to USDC.e) and withdraws whichever has the requested amount.

use anyhow::{bail, Result};
use crate::onchainos::{get_pusd_balance, get_usdc_balance, get_wallet_address};

pub async fn run(amount: &str, dry_run: bool) -> Result<()> {
    use crate::config::Contracts;

    let eoa = get_wallet_address().await?;
    let creds = crate::config::load_credentials()
        .ok()
        .flatten()
        .ok_or_else(|| anyhow::anyhow!("No credentials found. Run `polymarket setup-proxy` first."))?;
    let proxy = creds.proxy_wallet
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No proxy wallet configured. Run `polymarket setup-proxy` first."))?
        .clone();

    let amount_f: f64 = amount.parse().map_err(|_| anyhow::anyhow!("invalid amount"))?;
    if amount_f <= 0.0 {
        bail!("amount must be positive");
    }
    let amount_raw = (amount_f * 1_000_000.0).round() as u128;

    // Auto-detect which token the proxy holds: pUSD (V2) or USDC.e (V1).
    // Check both in parallel and pick whichever has enough balance.
    let (pusd_bal_r, usdc_bal_r) = tokio::join!(
        get_pusd_balance(&proxy),
        get_usdc_balance(&proxy),
    );
    let pusd_bal  = pusd_bal_r.unwrap_or(0.0);
    let usdc_e_bal = usdc_bal_r.unwrap_or(0.0);
    let pusd_raw  = (pusd_bal * 1_000_000.0).floor() as u128;
    let usdc_e_raw = (usdc_e_bal * 1_000_000.0).floor() as u128;

    // Prefer pUSD (V2 collateral) if it covers the requested amount.
    let (token_name, token_addr, proxy_bal) = if pusd_raw >= amount_raw {
        ("pUSD", Contracts::PUSD, pusd_bal)
    } else if usdc_e_raw >= amount_raw {
        ("USDC.e", Contracts::USDC_E, usdc_e_bal)
    } else {
        bail!(
            "Insufficient proxy wallet balance: have ${:.2} pUSD + ${:.2} USDC.e, need ${:.2}. \
             Check `polymarket balance` for details.",
            pusd_bal, usdc_e_bal, amount_f
        );
    };

    if dry_run {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "from": proxy,
                "to": eoa,
                "token": token_name,
                "token_contract": token_addr,
                "amount": amount_f,
                "amount_raw": amount_raw,
                "proxy_balance": proxy_bal,
                "note": "dry-run: no transaction submitted"
            }
        }))?);
        return Ok(());
    }

    eprintln!("[polymarket] Withdrawing ${:.2} {} from proxy {} to EOA {}...", amount_f, token_name, proxy, eoa);
    // Route to the correct onchainos withdraw helper based on token.
    let tx_hash = if token_addr == Contracts::PUSD {
        crate::onchainos::withdraw_pusd_from_proxy(&eoa, amount_raw).await?
    } else {
        crate::onchainos::withdraw_usdc_from_proxy(&eoa, amount_raw).await?
    };
    eprintln!("[polymarket] Withdraw tx: {}", tx_hash);
    eprintln!("[polymarket] Waiting for confirmation...");
    crate::onchainos::wait_for_tx_receipt(&tx_hash, 30).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "tx_hash": tx_hash,
            "from": proxy,
            "to": eoa,
            "token": token_name,
            "amount": amount_f,
        }
    }))?);
    Ok(())
}
