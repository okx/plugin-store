use anyhow::{bail, Result};
use reqwest::Client;

use crate::api::{get_clob_market, get_gamma_market_by_slug, get_positions};
use crate::config::load_credentials;
use crate::onchainos::{ctf_redeem_positions, ctf_redeem_via_proxy, get_wallet_address};

/// Run the redeem command.
///
/// Automatically determines which wallet (EOA / proxy) holds the winning outcome tokens
/// by querying the Data API, then submits the correct redeemPositions path for each.
///
/// This handles all four cases correctly regardless of the current trading mode setting:
///   - Tokens in EOA only          → EOA direct redeem
///   - Tokens in proxy only        → proxy redeem via PROXY_FACTORY
///   - Tokens in both              → both redeems submitted
///   - Data API lag / no positions → fallback to EOA redeem with a warning
pub async fn run(market_id: &str, dry_run: bool) -> Result<()> {
    let client = Client::new();

    // Resolve condition_id and check neg_risk
    let (condition_id, neg_risk, question) = if market_id.starts_with("0x") {
        let m = get_clob_market(&client, market_id).await?;
        let q = m.question.unwrap_or_default();
        (m.condition_id, m.neg_risk, q)
    } else {
        let m = get_gamma_market_by_slug(&client, market_id).await?;
        let cid = m
            .condition_id
            .ok_or_else(|| anyhow::anyhow!("market has no conditionId: {}", market_id))?;
        let q = m.question.unwrap_or_default();
        // Get authoritative neg_risk from CLOB (same fix as buy/sell)
        let neg_risk = match get_clob_market(&client, &cid).await {
            Ok(clob) => clob.neg_risk,
            Err(_) => m.neg_risk,
        };
        (cid, neg_risk, q)
    };

    if neg_risk {
        bail!(
            "redeem is not supported for neg_risk (multi-outcome) markets — use the Polymarket web UI to redeem positions in this market"
        );
    }

    let cid_hex = condition_id.trim_start_matches("0x");
    let cid_display = format!("0x{}", cid_hex);

    // Resolve EOA and proxy wallet addresses
    let eoa_addr  = get_wallet_address().await?;
    let creds     = load_credentials().unwrap_or_default();
    let proxy_addr = creds.and_then(|c| c.proxy_wallet);

    if dry_run {
        let out = serde_json::json!({
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
                "index_sets": [1, 2],
                "note": "dry-run: will redeem from whichever wallet (EOA / proxy) holds the winning tokens, regardless of current trading mode."
            }
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // Determine which wallet(s) have redeemable positions for this condition_id.
    // We check the Data API for each address independently — this is correct regardless
    // of the current trading mode setting, since the mode may have changed after the order.
    let eoa_redeemable = {
        let positions = get_positions(&client, &eoa_addr).await.unwrap_or_default();
        let has = positions.iter().any(|p| {
            (p.condition_id.as_deref() == Some(&condition_id)
                || p.condition_id.as_deref() == Some(&cid_display))
                && p.redeemable
        });
        // Also warn if EOA positions exist but have no value (resolved against us)
        if !has {
            let lost: f64 = positions.iter()
                .filter(|p| p.condition_id.as_deref() == Some(&condition_id)
                    || p.condition_id.as_deref() == Some(&cid_display))
                .map(|p| p.current_value.unwrap_or(0.0))
                .sum();
            if lost < 0.000_001 && positions.iter().any(|p|
                p.condition_id.as_deref() == Some(&condition_id)
                || p.condition_id.as_deref() == Some(&cid_display))
            {
                eprintln!(
                    "[polymarket] Note: EOA has positions for this market but current_value ≈ $0 \
                     (market resolved against your EOA positions)."
                );
            }
        }
        has
    };

    let proxy_redeemable = if let Some(ref proxy) = proxy_addr {
        let positions = get_positions(&client, proxy).await.unwrap_or_default();
        positions.iter().any(|p| {
            (p.condition_id.as_deref() == Some(&condition_id)
                || p.condition_id.as_deref() == Some(&cid_display))
                && p.redeemable
        })
    } else {
        false
    };

    // If Data API shows nothing redeemable in either wallet, fall back to EOA redeem.
    // The API can lag after resolution — we still attempt rather than blocking the user.
    if !eoa_redeemable && !proxy_redeemable {
        eprintln!(
            "[polymarket] Warning: Data API shows no redeemable positions for {} in EOA or proxy wallet \
             (API may lag after market resolution). Attempting EOA redeem as fallback.",
            cid_display
        );
        let tx_hash = ctf_redeem_positions(&condition_id).await?;
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "condition_id": cid_display,
                "question": question,
                "eoa_tx": tx_hash,
                "source": "fallback_eoa",
                "note": "EOA redeemPositions submitted (fallback — Data API showed no positions). USDC.e arrives once tx confirms."
            }
        }))?);
        return Ok(());
    }

    // Execute the correct path(s) based on actual token ownership
    let mut out = serde_json::json!({
        "ok": true,
        "data": {
            "condition_id": cid_display,
            "question": question,
        }
    });

    if eoa_redeemable {
        eprintln!("[polymarket] EOA has winning tokens — submitting EOA redeemPositions...");
        let tx = ctf_redeem_positions(&condition_id).await?;
        out["data"]["eoa_tx"]   = serde_json::Value::String(tx);
        out["data"]["eoa_note"] = serde_json::Value::String(
            "EOA redeemPositions submitted.".into()
        );
    }

    if proxy_redeemable {
        eprintln!("[polymarket] Proxy has winning tokens — submitting proxy redeemPositions via PROXY_FACTORY...");
        let tx = ctf_redeem_via_proxy(&condition_id).await?;
        out["data"]["proxy_tx"]   = serde_json::Value::String(tx);
        out["data"]["proxy_note"] = serde_json::Value::String(
            "Proxy redeemPositions submitted via PROXY_FACTORY.".into()
        );
    }

    out["data"]["note"] = serde_json::Value::String(
        "USDC.e will be transferred to the respective wallet(s) once tx(s) confirm on Polygon.".into()
    );

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
