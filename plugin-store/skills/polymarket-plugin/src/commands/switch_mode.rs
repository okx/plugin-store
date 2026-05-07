/// `polymarket switch-mode` — permanently change the default trading mode.
///
/// Modes:
///   eoa   — EOA wallet is the maker. Requires POL for each approve transaction.
///   proxy — Proxy wallet is the maker. No POL needed for trading; relayer pays gas.
///           Requires `polymarket setup-proxy` to have been run first.

use anyhow::{bail, Result};
use reqwest::Client;

pub async fn run(mode: &str) -> Result<()> {
    match run_inner(mode).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("switch-mode"), None)); Ok(()) }
    }
}

async fn run_inner(mode: &str) -> Result<()> {
    let client = Client::new();

    let signer_addr = crate::onchainos::get_wallet_address().await?;
    let mut creds = crate::auth::ensure_credentials(&client, &signer_addr).await?;

    let new_mode = match mode.to_lowercase().as_str() {
        "proxy" | "poly_proxy" | "polyproxy"         => crate::config::TradingMode::PolyProxy,
        "eoa"                                         => crate::config::TradingMode::Eoa,
        "deposit-wallet" | "deposit_wallet" | "dw"   => crate::config::TradingMode::DepositWallet,
        other => bail!(
            "Unknown mode '{}'. Valid values: eoa, proxy, deposit-wallet",
            other
        ),
    };

    // Mode-specific pre-flight: ensure the required wallet exists.
    match new_mode {
        crate::config::TradingMode::PolyProxy if creds.proxy_wallet.is_none() => {
            bail!(
                "Cannot switch to proxy mode: no proxy wallet configured.\n\
                 Run `polymarket setup-proxy` first to deploy a proxy wallet."
            );
        }
        crate::config::TradingMode::DepositWallet if creds.deposit_wallet.is_none() => {
            bail!(
                "Cannot switch to deposit-wallet mode: no deposit wallet configured.\n\
                 Run `polymarket setup-deposit-wallet` first to deploy a deposit wallet."
            );
        }
        _ => {}
    }

    let old_mode = creds.mode.clone();
    if old_mode == new_mode {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "data": {
                    "mode": mode,
                    "note": "Already in this mode — no change."
                }
            })
        );
        return Ok(());
    }

    creds.mode = new_mode;
    crate::config::save_credentials(&creds)?;

    let description = match &creds.mode {
        crate::config::TradingMode::PolyProxy => format!(
            "POLY_PROXY mode. Maker: {}. No POL needed for trading.",
            creds.proxy_wallet.as_deref().unwrap_or("(unknown)")
        ),
        crate::config::TradingMode::Eoa => format!(
            "EOA mode. Maker: {}. POL required for approve transactions.",
            signer_addr
        ),
        crate::config::TradingMode::DepositWallet => format!(
            "DEPOSIT_WALLET mode. Maker: {}. Gasless (relayer-paid). POLY_1271 signatures.",
            creds.deposit_wallet.as_deref().unwrap_or("(unknown)")
        ),
    };

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "data": {
                "mode": mode,
                "description": description,
                "proxy_wallet": creds.proxy_wallet,
            }
        })
    );
    Ok(())
}
