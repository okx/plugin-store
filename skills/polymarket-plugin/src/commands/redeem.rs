use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::api::{get_clob_market, get_clob_version, get_gamma_market_by_slug, get_positions};
use crate::config::{Contracts, load_credentials};
use crate::onchainos::{
    ctf_redeem_positions, ctf_redeem_via_proxy, decimal_str_to_hex64, get_ctf_balance,
    get_wallet_address, negrisk_redeem_positions, wait_for_tx_receipt, wait_for_tx_receipt_labeled,
};

const REDEEM_WAIT_SECS: u64 = 120;

/// Resolve (condition_id, neg_risk, question) from a market_id (condition_id or slug).
async fn resolve_market(client: &Client, market_id: &str) -> Result<(String, bool, String)> {
    if market_id.starts_with("0x") {
        let m = get_clob_market(client, market_id).await?;
        let q = m.question.unwrap_or_default();
        Ok((m.condition_id, m.neg_risk, q))
    } else {
        let m = get_gamma_market_by_slug(client, market_id).await?;
        let cid = m
            .condition_id
            .ok_or_else(|| anyhow::anyhow!("market has no conditionId: {}", market_id))?;
        let q = m.question.unwrap_or_default();
        let neg_risk = match get_clob_market(client, &cid).await {
            Ok(clob) => clob.neg_risk,
            Err(_) => m.neg_risk,
        };
        Ok((cid, neg_risk, q))
    }
}

/// Core redeem logic for a single condition_id.
///
/// Checks which wallet(s) hold redeemable tokens via the Data API, submits the
/// appropriate tx(es), and waits for each to confirm on-chain before returning.
///
/// `collateral_addr`: USDC.e for V1 positions, pUSD for V2 positions.
///
/// Returns a JSON Value summarising the result (for use in both single and batch flows).
async fn redeem_one(
    client: &Client,
    condition_id: &str,
    question: &str,
    eoa_addr: &str,
    proxy_addr: Option<&str>,
    collateral_addr: &str,
) -> Result<serde_json::Value> {
    let cid_hex = condition_id.trim_start_matches("0x");
    let cid_display = format!("0x{}", cid_hex);

    let eoa_redeemable = {
        let positions = get_positions(client, eoa_addr).await.unwrap_or_default();
        let has = positions.iter().any(|p| {
            (p.condition_id.as_deref() == Some(condition_id)
                || p.condition_id.as_deref() == Some(&cid_display))
                && p.redeemable
        });
        if !has {
            let lost: f64 = positions
                .iter()
                .filter(|p| {
                    p.condition_id.as_deref() == Some(condition_id)
                        || p.condition_id.as_deref() == Some(&cid_display)
                })
                .map(|p| p.current_value.unwrap_or(0.0))
                .sum();
            if lost < 0.000_001
                && positions.iter().any(|p| {
                    p.condition_id.as_deref() == Some(condition_id)
                        || p.condition_id.as_deref() == Some(&cid_display)
                })
            {
                eprintln!(
                    "[polymarket] Note: EOA has positions for this market but current_value ≈ $0 \
                     (market resolved against your EOA positions)."
                );
            }
        }
        has
    };

    let proxy_redeemable = if let Some(proxy) = proxy_addr {
        let positions = get_positions(client, proxy).await.unwrap_or_default();
        positions.iter().any(|p| {
            (p.condition_id.as_deref() == Some(condition_id)
                || p.condition_id.as_deref() == Some(&cid_display))
                && p.redeemable
        })
    } else {
        false
    };

    let mut out = serde_json::json!({
        "condition_id": cid_display,
        "question": question,
    });

    // Fallback: if Data API shows nothing (can lag after resolution), attempt EOA redeem.
    if !eoa_redeemable && !proxy_redeemable {
        eprintln!(
            "[polymarket] Warning: Data API shows no redeemable positions for {} \
             (may lag after resolution). Attempting EOA redeem as fallback.",
            cid_display
        );
        let tx_hash = ctf_redeem_positions(condition_id, collateral_addr).await?;
        eprintln!("[polymarket] Waiting for EOA redeem tx to confirm...");
        wait_for_tx_receipt(&tx_hash, 120).await?;
        out["eoa_tx"] = serde_json::Value::String(tx_hash);
        out["source"] = serde_json::Value::String("fallback_eoa".into());
        out["note"] = serde_json::Value::String(
            "EOA redeemPositions confirmed (fallback).".into(),
        );
        return Ok(out);
    }

    if eoa_redeemable {
        eprintln!("[polymarket] EOA has winning tokens — submitting EOA redeemPositions...");
        let tx = ctf_redeem_positions(condition_id, collateral_addr).await?;
        eprintln!("[polymarket] Waiting for EOA redeem tx to confirm...");
        wait_for_tx_receipt(&tx, 120).await?;
        out["eoa_tx"] = serde_json::Value::String(tx);
        out["eoa_note"] =
            serde_json::Value::String("EOA redeemPositions confirmed.".into());
    }

    if proxy_redeemable {
        eprintln!(
            "[polymarket] Proxy has winning tokens — submitting proxy redeemPositions via PROXY_FACTORY..."
        );
        let tx = ctf_redeem_via_proxy(condition_id, collateral_addr).await?;
        eprintln!("[polymarket] Waiting for proxy redeem tx to confirm...");
        wait_for_tx_receipt(&tx, 120).await?;
        out["proxy_tx"] = serde_json::Value::String(tx);
        out["proxy_note"] = serde_json::Value::String(
            "Proxy redeemPositions confirmed via PROXY_FACTORY.".into(),
        );
    }

    let collateral_sym = if collateral_addr.eq_ignore_ascii_case(Contracts::PUSD) { "pUSD" } else { "USDC.e" };
    out["note"] = serde_json::Value::String(
        format!("{} transferred to the respective wallet(s).", collateral_sym),
    );
    Ok(out)
}

/// Redeem neg_risk (multi-outcome) market via NegRiskAdapter.redeemPositions.
/// Queries on-chain ERC-1155 balances for each token_id, then broadcasts.
async fn redeem_one_negrisk(
    condition_id: &str,
    question: &str,
    token_ids: &[String],
    eoa_addr: &str,
) -> Result<serde_json::Value> {
    let cid_display = format!("0x{}", condition_id.trim_start_matches("0x"));

    // Validate token IDs can be encoded.
    for tid in token_ids {
        decimal_str_to_hex64(tid)
            .with_context(|| format!("token_id '{}' is not a valid decimal integer", tid))?;
    }

    // Query on-chain ERC-1155 balances for each outcome.
    let mut amounts: Vec<u128> = Vec::with_capacity(token_ids.len());
    for tid in token_ids {
        let bal = get_ctf_balance(eoa_addr, tid).await.unwrap_or(0);
        amounts.push(bal);
    }

    if amounts.iter().all(|&a| a == 0) {
        bail!(
            "No outcome token balance found on-chain for {} in EOA wallet {}. \
             Market may not be resolved yet, or tokens may be in a different wallet.",
            cid_display, eoa_addr
        );
    }

    let total_shares: u128 = amounts.iter().sum();
    eprintln!(
        "[polymarket] NegRisk redeem: {} total shares across {} outcomes — submitting NegRiskAdapter.redeemPositions...",
        total_shares, amounts.len()
    );
    let tx = negrisk_redeem_positions(condition_id, &amounts, eoa_addr).await?;
    eprintln!(
        "[polymarket] NegRisk redeem tx {} — waiting up to {}s for on-chain confirmation...",
        tx, REDEEM_WAIT_SECS
    );
    wait_for_tx_receipt_labeled(&tx, REDEEM_WAIT_SECS, "NegRisk redeem").await?;

    Ok(serde_json::json!({
        "condition_id": cid_display,
        "question": question,
        "neg_risk": true,
        "eoa_tx": tx,
        "amounts": amounts.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
        "note": "NegRiskAdapter.redeemPositions confirmed. USDC.e transferred to EOA.",
    }))
}

/// Redeem a single market by market_id (condition_id or slug).
pub async fn run(market_id: &str, dry_run: bool) -> Result<()> {
    let client = Client::new();
    let (condition_id, neg_risk, question) = resolve_market(&client, market_id).await?;

    if neg_risk {
        // Fetch CLOB token IDs for on-chain balance queries.
        let clob = get_clob_market(&client, &condition_id).await
            .context("Failed to fetch CLOB market for NegRisk token IDs")?;
        let token_ids: Vec<String> = clob.tokens.iter().map(|t| t.token_id.clone()).collect();

        if dry_run {
            let cid_display = format!("0x{}", condition_id.trim_start_matches("0x"));
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "data": {
                        "dry_run": true,
                        "market_id": market_id,
                        "condition_id": cid_display,
                        "question": question,
                        "neg_risk": true,
                        "action": "NegRiskAdapter.redeemPositions",
                        "token_ids": token_ids,
                        "note": "dry-run: will query on-chain ERC-1155 balances and call NegRiskAdapter.redeemPositions."
                    }
                }))?
            );
            return Ok(());
        }

        let eoa_addr = get_wallet_address().await?;
        let result = redeem_one_negrisk(&condition_id, &question, &token_ids, &eoa_addr).await?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "ok": true, "data": result }))?
        );
        return Ok(());
    }

    let cid_display = format!("0x{}", condition_id.trim_start_matches("0x"));
    let (eoa_addr, clob_version_raw) = tokio::join!(
        get_wallet_address(),
        get_clob_version(&client),
    );
    let eoa_addr = eoa_addr?;
    let clob_version_raw = clob_version_raw?;
    // Use pUSD as collateral for V2 markets (cutover ~2026-04-28).
    let collateral_addr = if clob_version_raw == 2 { Contracts::PUSD } else { Contracts::USDC_E };
    let creds = load_credentials().unwrap_or_default();
    let proxy_addr = creds.and_then(|c| c.proxy_wallet);

    if dry_run {
        let collateral_sym = if clob_version_raw == 2 { "pUSD" } else { "USDC.e" };
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "data": {
                    "dry_run": true,
                    "market_id": market_id,
                    "condition_id": cid_display,
                    "question": question,
                    "neg_risk": false,
                    "eoa_wallet": eoa_addr,
                    "proxy_wallet": proxy_addr,
                    "action": "redeemPositions",
                    "collateral": collateral_sym,
                    "index_sets": [1, 2],
                    "note": "dry-run: will redeem from whichever wallet (EOA / proxy) holds the winning tokens."
                }
            }))?
        );
        return Ok(());
    }

    let result = redeem_one(&client, &condition_id, &question, &eoa_addr, proxy_addr.as_deref(), collateral_addr).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "ok": true, "data": result }))?
    );
    Ok(())
}

/// Redeem ALL redeemable positions across EOA and proxy wallets in one pass.
///
/// Discovers redeemable condition_ids from both wallets via the Data API, then
/// redeems each sequentially, waiting for on-chain confirmation between markets.
pub async fn run_all(dry_run: bool) -> Result<()> {
    let client = Client::new();
    let (eoa_addr, clob_version_raw) = tokio::join!(
        get_wallet_address(),
        get_clob_version(&client),
    );
    let eoa_addr = eoa_addr?;
    let clob_version_raw = clob_version_raw?;
    // Use pUSD as collateral for V2 markets (cutover ~2026-04-28).
    let collateral_addr = if clob_version_raw == 2 { Contracts::PUSD } else { Contracts::USDC_E };
    let creds = load_credentials().unwrap_or_default();
    let proxy_addr = creds.and_then(|c| c.proxy_wallet);

    // Collect all unique redeemable condition_ids from both wallets.
    let mut redeemable: Vec<(String, String)> = Vec::new(); // (condition_id, title)

    let eoa_positions = get_positions(&client, &eoa_addr).await.unwrap_or_default();
    for p in &eoa_positions {
        if p.redeemable {
            if let Some(cid) = &p.condition_id {
                let title = p.title.clone().unwrap_or_default();
                if !redeemable.iter().any(|(c, _)| c == cid) {
                    redeemable.push((cid.clone(), title));
                }
            }
        }
    }

    if let Some(ref proxy) = proxy_addr {
        let proxy_positions = get_positions(&client, proxy).await.unwrap_or_default();
        for p in &proxy_positions {
            if p.redeemable {
                if let Some(cid) = &p.condition_id {
                    let title = p.title.clone().unwrap_or_default();
                    if !redeemable.iter().any(|(c, _)| c == cid) {
                        redeemable.push((cid.clone(), title));
                    }
                }
            }
        }
    }

    if redeemable.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "data": {
                    "message": "No redeemable positions found.",
                    "redeemed_count": 0
                }
            }))?
        );
        return Ok(());
    }

    eprintln!(
        "[polymarket] Found {} redeemable position(s). Redeeming sequentially...",
        redeemable.len()
    );

    if dry_run {
        let items: Vec<_> = redeemable
            .iter()
            .map(|(cid, title)| serde_json::json!({ "condition_id": cid, "title": title }))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "data": {
                    "dry_run": true,
                    "redeemable_count": items.len(),
                    "positions": items,
                    "note": "dry-run: would redeem each position sequentially, waiting for on-chain confirmation between each."
                }
            }))?
        );
        return Ok(());
    }

    let mut results = Vec::new();
    let mut errors = Vec::new();

    for (i, (cid, title)) in redeemable.iter().enumerate() {
        eprintln!(
            "[polymarket] [{}/{}] Redeeming: {}",
            i + 1,
            redeemable.len(),
            title
        );
        // Fetch neg_risk status + token IDs for each market.
        let clob = get_clob_market(&client, cid).await;
        let is_neg_risk = clob.as_ref().map(|m| m.neg_risk).unwrap_or(false);
        let token_ids: Vec<String> = clob.as_ref()
            .map(|m| m.tokens.iter().map(|t| t.token_id.clone()).collect())
            .unwrap_or_default();

        let result = if is_neg_risk && !token_ids.is_empty() {
            redeem_one_negrisk(cid, title, &token_ids, &eoa_addr).await
        } else {
            redeem_one(&client, cid, title, &eoa_addr, proxy_addr.as_deref(), collateral_addr).await
        };

        match result {
            Ok(r) => results.push(r),
            Err(e) => {
                eprintln!("[polymarket] Error redeeming {}: {}", cid, e);
                errors.push(serde_json::json!({ "condition_id": cid, "error": e.to_string() }));
            }
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": errors.is_empty(),
            "data": {
                "redeemed_count": results.len(),
                "error_count": errors.len(),
                "results": results,
                "errors": errors,
                "note": "Collateral (pUSD/USDC.e) transferred to respective wallet(s) for all confirmed redemptions."
            }
        }))?
    );
    Ok(())
}
