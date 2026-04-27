/// `polymarket setup-proxy` — create a Polymarket proxy wallet and switch to POLY_PROXY mode.
///
/// Flow:
///   1. Check if proxy wallet already exists (via cached creds or on-chain)
///   2. If not: call PROXY_FACTORY.proxy([]) on-chain to deploy one (one-time POL gas cost)
///   3. Resolve the proxy address from the transaction trace
///   4. Persist proxy_wallet + mode=PolyProxy in creds.json
///   5. Set up one-time approvals on the proxy wallet so trading is gasless:
///
///      V1 (6 txs — USDC.e collateral):
///        USDC.e.approve(CTF_EXCHANGE, MAX_UINT)
///        CTF.setApprovalForAll(CTF_EXCHANGE, true)
///        USDC.e.approve(NEG_RISK_CTF_EXCHANGE, MAX_UINT)
///        CTF.setApprovalForAll(NEG_RISK_CTF_EXCHANGE, true)
///        USDC.e.approve(NEG_RISK_ADAPTER, MAX_UINT)
///        CTF.setApprovalForAll(NEG_RISK_ADAPTER, true)
///
///      V2 (4 txs — pUSD collateral, new exchange contracts post-2026-04-28):
///        pUSD.approve(CTF_EXCHANGE_V2, MAX_UINT)
///        pUSD.approve(NEG_RISK_CTF_EXCHANGE_V2, MAX_UINT)
///        pUSD.approve(NEG_RISK_ADAPTER, MAX_UINT)
///        USDC.e.approve(COLLATERAL_ONRAMP, MAX_UINT)  ← auto-wrap USDC.e → pUSD
///
/// After setup, all subsequent buy/sell commands use POLY_PROXY mode (no POL for trading).
/// Run `polymarket switch-mode --mode eoa` to revert to EOA mode at any time.

use anyhow::{Context as _, Result};
use reqwest::Client;

pub async fn run(dry_run: bool) -> Result<()> {
    let client = Client::new();

    // Geo check — WARNING only, do not abort. Users in restricted regions can still
    // set up a proxy wallet; trading commands (buy/sell) will hard-fail separately.
    if let Some(geo_msg) = crate::api::check_clob_access(&client).await {
        eprintln!("[polymarket] WARNING: {}", geo_msg);
        eprintln!("[polymarket] Continuing setup — proxy wallet creation does not require trading access.");
    }

    let signer_addr = crate::onchainos::get_wallet_address().await?;
    let mut creds = crate::auth::ensure_credentials(&client, &signer_addr).await?;

    // Step 1: check if proxy wallet already exists in cached creds.
    if let Some(ref proxy) = creds.proxy_wallet {
        if creds.mode == crate::config::TradingMode::PolyProxy {
            let proxy = proxy.clone();
            eprintln!("[polymarket] Proxy wallet already configured. Checking approvals...");
            ensure_proxy_approvals(&proxy, dry_run).await?;
            println!(
                "{}",
                serde_json::json!({
                    "ok": true,
                    "data": {
                        "status": "already_configured",
                        "proxy_wallet": proxy,
                        "mode": "poly_proxy",
                        "note": "Proxy wallet set up and approvals confirmed. Use `polymarket switch-mode --mode eoa` to revert."
                    }
                })
            );
            return Ok(());
        }
        // Has proxy but mode is EOA — switch mode and ensure approvals.
        let proxy = proxy.clone();
        if !dry_run {
            creds.mode = crate::config::TradingMode::PolyProxy;
            crate::config::save_credentials(&creds)?;
        }
        ensure_proxy_approvals(&proxy, dry_run).await?;
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": dry_run,
                "data": {
                    "status": "mode_switched",
                    "proxy_wallet": proxy,
                    "mode": "poly_proxy",
                    "note": if dry_run { "dry-run: would switch to POLY_PROXY mode (no state written)" } else { "Switched to POLY_PROXY mode. Deposit USDC.e with `polymarket deposit --amount <N>`." }
                }
            })
        );
        return Ok(());
    }

    // Step 2: mandatory on-chain check before any deployment.
    // If the RPC call fails we MUST abort — we cannot distinguish "no proxy exists"
    // from "RPC error", and deploying a duplicate wastes gas and risks proxy confusion.
    eprintln!("[polymarket] Checking on-chain for existing proxy wallet...");
    let existing_proxy = crate::onchainos::get_existing_proxy(&signer_addr).await
        .map_err(|e| anyhow::anyhow!(
            "On-chain proxy check failed: {}. \
             Aborting to prevent duplicate deployment. Retry when the RPC is available.",
            e
        ))?;

    if let Some(existing) = existing_proxy {
        eprintln!("[polymarket] Found existing proxy on-chain: {}", existing);
        if !dry_run {
            creds.proxy_wallet = Some(existing.clone());
            creds.mode = crate::config::TradingMode::PolyProxy;
            crate::config::save_credentials(&creds)?;
        }
        ensure_proxy_approvals(&existing, dry_run).await?;
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": dry_run,
                "data": {
                    "status": "recovered",
                    "proxy_wallet": existing,
                    "mode": "poly_proxy",
                    "note": if dry_run { "dry-run: found proxy on-chain; would save to creds (no state written)" } else { "Existing proxy wallet found on-chain and saved to creds. No new deployment needed." }
                }
            })
        );
        return Ok(());
    }

    // Step 3: confirmed no proxy on-chain — deploy one via PROXY_FACTORY.
    if dry_run {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "signer": signer_addr,
                    "action": "would call PROXY_FACTORY.proxy([]) to deploy proxy wallet, then set 10 USDC.e/pUSD/CTF approvals",
                    "note": "dry-run: no transaction submitted"
                }
            })
        );
        return Ok(());
    }

    eprintln!("[polymarket] Deploying proxy wallet via PROXY_FACTORY (one-time gas cost)...");
    let tx_hash = crate::onchainos::create_proxy_wallet().await?;
    eprintln!("[polymarket] Proxy wallet deploy tx: {}", tx_hash);

    // Resolve the proxy address from the transaction trace.
    eprintln!("[polymarket] Resolving proxy wallet address from transaction trace...");
    let proxy_addr = crate::onchainos::get_proxy_address_from_tx(&tx_hash)
        .await
        .with_context(|| format!(
            "Proxy deployed (tx {}) but address could not be resolved. \
             Check: https://polygonscan.com/tx/{}",
            tx_hash, tx_hash
        ))?;

    // Step 4: persist.
    creds.proxy_wallet = Some(proxy_addr.clone());
    creds.mode = crate::config::TradingMode::PolyProxy;
    crate::config::save_credentials(&creds)?;

    // Step 5: set up the one-time approvals so trading is gasless.
    ensure_proxy_approvals(&proxy_addr, dry_run).await?;

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "data": {
                "status": "created",
                "proxy_wallet": proxy_addr,
                "deploy_tx": tx_hash,
                "mode": "poly_proxy",
                "next_step": "Deposit USDC.e with: polymarket deposit --amount <N>"
            }
        })
    );
    Ok(())
}

/// Set up the one-time on-chain approvals required for gasless trading in POLY_PROXY mode.
///
/// V1 block (6 txs): USDC.e + CTF approved to V1 exchange contracts.
/// Idempotent: skipped if USDC.e→CTF_EXCHANGE allowance is already non-zero.
///
/// V2 block (4 txs): pUSD approved to V2 exchange contracts + USDC.e approved to
/// COLLATERAL_ONRAMP for auto-wrap. Idempotent: skipped if pUSD→CTF_EXCHANGE_V2
/// allowance is already non-zero.
async fn ensure_proxy_approvals(proxy_addr: &str, dry_run: bool) -> Result<()> {
    use crate::config::Contracts;

    // ── V1 approvals ─────────────────────────────────────────────────────────
    let v1_existing = crate::onchainos::get_usdc_allowance(proxy_addr, Contracts::CTF_EXCHANGE)
        .await
        .unwrap_or(0);
    if v1_existing > 0 {
        eprintln!("[polymarket] USDC.e approvals already set (allowance: {}).", v1_existing);
    } else if dry_run {
        eprintln!("[polymarket] dry-run: would set 6 V1 approvals (USDC.e + CTF × 3 contracts).");
    } else {
        eprintln!("[polymarket] Setting up V1 USDC.e / CTF approvals for gasless trading...");
        let v1_approvals: &[(&str, bool, &str)] = &[
            (Contracts::CTF_EXCHANGE,          false, "CTF Exchange / USDC.e"),
            (Contracts::CTF_EXCHANGE,          true,  "CTF Exchange / CTF"),
            (Contracts::NEG_RISK_CTF_EXCHANGE, false, "Neg Risk CTF Exchange / USDC.e"),
            (Contracts::NEG_RISK_CTF_EXCHANGE, true,  "Neg Risk CTF Exchange / CTF"),
            (Contracts::NEG_RISK_ADAPTER,      false, "Neg Risk Adapter / USDC.e"),
            (Contracts::NEG_RISK_ADAPTER,      true,  "Neg Risk Adapter / CTF"),
        ];
        for (spender, is_ctf, label) in v1_approvals {
            eprintln!("[polymarket] Approving {} ...", label);
            let tx = if *is_ctf {
                crate::onchainos::proxy_ctf_set_approval_for_all(spender).await?
            } else {
                crate::onchainos::proxy_usdc_approve(spender).await?
            };
            eprintln!("[polymarket] tx: {}", tx);
            crate::onchainos::wait_for_tx_receipt(&tx, 30).await?;
        }
        eprintln!("[polymarket] V1 approvals confirmed.");
    }

    // ── V2 approvals ─────────────────────────────────────────────────────────
    // pUSD approvals to V2 exchange contracts + USDC.e to COLLATERAL_ONRAMP.
    let v2_existing = crate::onchainos::get_pusd_allowance(proxy_addr, Contracts::CTF_EXCHANGE_V2)
        .await
        .unwrap_or(0);
    if v2_existing > 0 {
        eprintln!("[polymarket] pUSD V2 approvals already set (allowance: {}).", v2_existing);
    } else if dry_run {
        eprintln!("[polymarket] dry-run: would set 4 V2 approvals (pUSD × 3 contracts + USDC.e → COLLATERAL_ONRAMP).");
    } else {
        eprintln!("[polymarket] Setting up V2 pUSD approvals for gasless V2 trading...");

        let v2_pusd_spenders: &[(&str, &str)] = &[
            (Contracts::CTF_EXCHANGE_V2,          "V2 CTF Exchange / pUSD"),
            (Contracts::NEG_RISK_CTF_EXCHANGE_V2, "V2 Neg Risk CTF Exchange / pUSD"),
            (Contracts::NEG_RISK_ADAPTER,         "Neg Risk Adapter / pUSD"),
        ];
        for (spender, label) in v2_pusd_spenders {
            eprintln!("[polymarket] Approving {} ...", label);
            let tx = crate::onchainos::proxy_pusd_approve(spender).await?;
            eprintln!("[polymarket] tx: {}", tx);
            crate::onchainos::wait_for_tx_receipt(&tx, 30).await?;
        }

        // USDC.e → COLLATERAL_ONRAMP: allows the proxy to auto-wrap USDC.e → pUSD on V2 buys.
        eprintln!("[polymarket] Approving COLLATERAL_ONRAMP / USDC.e ...");
        let onramp_tx = crate::onchainos::proxy_usdc_approve(Contracts::COLLATERAL_ONRAMP).await?;
        eprintln!("[polymarket] tx: {}", onramp_tx);
        crate::onchainos::wait_for_tx_receipt(&onramp_tx, 30).await?;

        eprintln!("[polymarket] V2 approvals confirmed. Proxy wallet fully ready for V1 and V2 gasless trading.");
    }

    Ok(())
}
