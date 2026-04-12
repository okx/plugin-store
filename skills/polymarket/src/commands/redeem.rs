use anyhow::{bail, Result};
use reqwest::Client;

use crate::api::{get_clob_market, get_gamma_market_by_slug, get_positions};
use crate::onchainos::{ctf_redeem_positions, get_wallet_address};

/// Run the redeem command.
///
/// market_id: condition_id (0x-prefixed) or slug
/// dry_run: if true, print preview and exit without submitting the tx
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

    if dry_run {
        let out = serde_json::json!({
            "ok": true,
            "data": {
                "dry_run": true,
                "market_id": market_id,
                "condition_id": cid_display,
                "question": question,
                "neg_risk": false,
                "action": "redeemPositions",
                "index_sets": [1, 2],
                "note": "dry-run: CTF redeemPositions tx not submitted. index_sets [1,2] covers YES and NO outcomes — the CTF contract pays out winning tokens and silently no-ops for losing ones."
            }
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // Pre-flight: check if this market has any redeemable value.
    // If all positions for this condition_id show current_value ≈ 0, the user
    // would waste gas on a no-op redemption — warn and require explicit confirmation
    // (agent should surface this; binary just logs a clear warning).
    let wallet_addr = get_wallet_address().await?;
    let positions = get_positions(&client, &wallet_addr).await.unwrap_or_default();
    let market_positions: Vec<_> = positions
        .iter()
        .filter(|p| p.condition_id.as_deref() == Some(&condition_id)
            || p.condition_id.as_deref() == Some(&cid_display))
        .collect();

    if !market_positions.is_empty() {
        let total_value: f64 = market_positions
            .iter()
            .filter(|p| p.redeemable)
            .map(|p| p.current_value.unwrap_or(0.0))
            .sum();

        if total_value < 0.000_001 {
            eprintln!(
                "[polymarket] Warning: all redeemable positions for this market have current_value ≈ $0. \
                 This market resolved against your positions — redeeming will cost gas and receive nothing. \
                 Use --dry-run to preview, or proceed only if you are certain."
            );
        }
    }

    let tx_hash = ctf_redeem_positions(&condition_id).await?;

    let out = serde_json::json!({
        "ok": true,
        "data": {
            "condition_id": cid_display,
            "question": question,
            "tx_hash": tx_hash,
            "note": "redeemPositions submitted. USDC.e will be transferred to your wallet once the tx confirms on Polygon."
        }
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
