use anyhow::{anyhow, Result};
use reqwest::Client;

use crate::api::{get_clob_market, get_gamma_market_by_slug, get_positions};
use crate::config::load_credentials;
use crate::onchainos::{
    ctf_redeem_positions, ctf_redeem_via_proxy, get_existing_proxy, get_pol_balance,
    get_wallet_address, wait_for_tx_receipt_labeled,
};

/// Per-redeem timeout (Polygon block time ~2s; a healthy tx mines in <30s).
/// Kept short so batch redeem stays under typical subprocess timeouts.
const REDEEM_WAIT_SECS: u64 = 45;

/// Estimated POL gas cost per redeem call (conservative).
/// CTF.redeemPositions on Polygon typically costs ~0.008 POL; we budget 2×
/// to absorb gas price spikes.
const POL_PER_REDEEM: f64 = 0.015;

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
            .ok_or_else(|| anyhow!("market has no conditionId: {}", market_id))?;
        let q = m.question.unwrap_or_default();
        let neg_risk = match get_clob_market(client, &cid).await {
            Ok(clob) => clob.neg_risk,
            Err(_) => m.neg_risk,
        };
        Ok((cid, neg_risk, q))
    }
}

/// Summary of which wallet(s) hold redeemable tokens for a given condition_id.
#[derive(Default)]
struct Redeemability {
    eoa: bool,
    proxy: bool,
}

async fn check_redeemability(
    client: &Client,
    condition_id: &str,
    eoa_addr: &str,
    proxy_addr: Option<&str>,
) -> Redeemability {
    let cid_hex = condition_id.trim_start_matches("0x");
    let cid_display = format!("0x{}", cid_hex);
    let matches = |cid_opt: Option<&str>| -> bool {
        cid_opt == Some(condition_id) || cid_opt == Some(&cid_display)
    };

    let eoa_positions = get_positions(client, eoa_addr).await.unwrap_or_default();
    let eoa = eoa_positions
        .iter()
        .any(|p| matches(p.condition_id.as_deref()) && p.redeemable);

    let proxy = if let Some(proxy) = proxy_addr {
        let positions = get_positions(client, proxy).await.unwrap_or_default();
        positions
            .iter()
            .any(|p| matches(p.condition_id.as_deref()) && p.redeemable)
    } else {
        false
    };

    Redeemability { eoa, proxy }
}

/// Core redeem logic for a single condition_id.
///
/// Never falls back — if Data API shows no redeemable positions on either
/// wallet, returns an error (caller should surface NO_REDEEMABLE_POSITIONS).
async fn redeem_one(
    client: &Client,
    condition_id: &str,
    question: &str,
    eoa_addr: &str,
    proxy_addr: Option<&str>,
) -> Result<serde_json::Value> {
    let cid_hex = condition_id.trim_start_matches("0x");
    let cid_display = format!("0x{}", cid_hex);

    let r = check_redeemability(client, condition_id, eoa_addr, proxy_addr).await;

    if !r.eoa && !r.proxy {
        return Err(anyhow!(
            "No redeemable positions found for {} on EOA ({}) {}. \
             Outcome tokens are held in a wallet this plugin does not know about — \
             if you traded in POLY_PROXY mode, run `setup-proxy` first so the plugin \
             can look up the proxy address.",
            cid_display,
            eoa_addr,
            proxy_addr
                .map(|p| format!("or proxy ({})", p))
                .unwrap_or_else(|| "(no proxy configured)".into())
        ));
    }

    let mut out = serde_json::json!({
        "condition_id": cid_display,
        "question": question,
    });

    if r.eoa {
        eprintln!("[polymarket] EOA holds winning tokens — submitting EOA redeemPositions...");
        let tx = ctf_redeem_positions(condition_id, eoa_addr).await?;
        eprintln!(
            "[polymarket] EOA redeem tx {} — waiting up to {}s for on-chain confirmation...",
            tx, REDEEM_WAIT_SECS
        );
        wait_for_tx_receipt_labeled(&tx, REDEEM_WAIT_SECS, "EOA redeem").await?;
        out["eoa_tx"] = serde_json::Value::String(tx);
        out["eoa_note"] =
            serde_json::Value::String("EOA redeemPositions confirmed.".into());
    }

    if r.proxy {
        eprintln!(
            "[polymarket] Proxy holds winning tokens — submitting proxy redeemPositions via PROXY_FACTORY..."
        );
        let tx = ctf_redeem_via_proxy(condition_id, eoa_addr).await?;
        eprintln!(
            "[polymarket] Proxy redeem tx {} — waiting up to {}s for on-chain confirmation...",
            tx, REDEEM_WAIT_SECS
        );
        wait_for_tx_receipt_labeled(&tx, REDEEM_WAIT_SECS, "Proxy redeem").await?;
        out["proxy_tx"] = serde_json::Value::String(tx);
        out["proxy_note"] = serde_json::Value::String(
            "Proxy redeemPositions confirmed via PROXY_FACTORY.".into(),
        );
    }

    out["note"] = serde_json::Value::String(
        "USDC.e transferred to the respective wallet(s).".into(),
    );
    Ok(out)
}

/// Look up an on-chain proxy wallet that is not yet recorded in credentials.
///
/// Safe to call freely: uses `debug_traceCall` (read-only, no gas, no tx). If the
/// RPC doesn't support `debug_traceCall` or anything else fails, returns None and
/// callers should fall through silently — this is purely a UX hint.
async fn discover_uncached_proxy(eoa: &str, creds_proxy: Option<&str>) -> Option<String> {
    if creds_proxy.is_some() {
        return None;
    }
    get_existing_proxy(eoa).await.ok().flatten()
}

/// Build a human-readable hint pointing at a proxy wallet discovered on-chain,
/// to be appended to an error's `suggestion` field. Empty string if no proxy found.
fn proxy_hint(discovered: Option<&str>) -> String {
    match discovered {
        Some(addr) => format!(
            "Detected existing proxy wallet on-chain for this EOA: {}. \
             Run `polymarket-plugin setup-proxy` to save it to credentials — \
             once saved, redeem will route through the proxy automatically.",
            addr
        ),
        None => String::new(),
    }
}

/// Fail-fast POL balance check: EOA pays gas for both EOA and proxy redeem paths.
async fn check_pol_budget(eoa_addr: &str, tx_count: usize) -> Result<f64> {
    let pol = get_pol_balance(eoa_addr).await?;
    let needed = tx_count as f64 * POL_PER_REDEEM;
    if pol < needed {
        return Err(anyhow!(
            "Insufficient POL for gas: EOA {} has {:.4} POL but redeeming {} market(s) \
             needs ~{:.4} POL (budgeting {} POL per market). \
             Top up {:.4} more POL.",
            eoa_addr,
            pol,
            tx_count,
            needed,
            POL_PER_REDEEM,
            needed - pol
        ));
    }
    Ok(pol)
}

/// Redeem a single market by market_id (condition_id or slug).
pub async fn run(market_id: &str, dry_run: bool) -> Result<()> {
    let client = Client::new();

    let (condition_id, neg_risk, question) = match resolve_market(&client, market_id).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&e, Some("redeem"), None));
            return Ok(());
        }
    };

    if neg_risk {
        let e = anyhow!(
            "redeem is not supported for neg_risk (multi-outcome) markets — \
             use the Polymarket web UI to redeem positions in this market"
        );
        println!("{}", super::error_response(&e, Some("redeem"), None));
        return Ok(());
    }

    let cid_display = format!("0x{}", condition_id.trim_start_matches("0x"));
    let eoa_addr = match get_wallet_address().await {
        Ok(a) => a,
        Err(e) => {
            println!("{}", super::error_response(&e, Some("redeem"), None));
            return Ok(());
        }
    };
    let creds = load_credentials().unwrap_or_default();
    let proxy_addr = creds.and_then(|c| c.proxy_wallet);

    // Best-effort: if no proxy in creds, check on-chain so error hints can cite the address.
    let discovered_proxy = discover_uncached_proxy(&eoa_addr, proxy_addr.as_deref()).await;
    let hint = proxy_hint(discovered_proxy.as_deref());
    let hint_opt = if hint.is_empty() { None } else { Some(hint.as_str()) };

    if dry_run {
        let r = check_redeemability(&client, &condition_id, &eoa_addr, proxy_addr.as_deref()).await;
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
                    "discovered_proxy": discovered_proxy,
                    "eoa_redeemable": r.eoa,
                    "proxy_redeemable": r.proxy,
                    "action": "redeemPositions",
                    "index_sets": [1, 2],
                    "note": "dry-run: will redeem from whichever wallet holds the winning tokens. \
                             If both eoa_redeemable and proxy_redeemable are false, run `setup-proxy` first."
                }
            }))?
        );
        return Ok(());
    }

    if let Err(e) = check_pol_budget(&eoa_addr, 1).await {
        println!("{}", super::error_response(&e, Some("redeem"), hint_opt));
        return Ok(());
    }

    match redeem_one(&client, &condition_id, &question, &eoa_addr, proxy_addr.as_deref()).await {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "data": result
                }))?
            );
        }
        Err(e) => {
            println!("{}", super::error_response(&e, Some("redeem"), hint_opt));
        }
    }
    Ok(())
}

/// Redeem ALL redeemable positions across EOA and proxy wallets in one pass.
pub async fn run_all(dry_run: bool) -> Result<()> {
    let client = Client::new();
    let eoa_addr = match get_wallet_address().await {
        Ok(a) => a,
        Err(e) => {
            println!("{}", super::error_response(&e, Some("redeem"), None));
            return Ok(());
        }
    };
    let creds = load_credentials().unwrap_or_default();
    let proxy_addr = creds.and_then(|c| c.proxy_wallet);

    // Best-effort discovery: if creds has no proxy but one exists on-chain,
    // surface it in error hints so the user knows `setup-proxy` is the fix.
    let discovered_proxy = discover_uncached_proxy(&eoa_addr, proxy_addr.as_deref()).await;
    let hint = proxy_hint(discovered_proxy.as_deref());
    let hint_opt = if hint.is_empty() { None } else { Some(hint.as_str()) };

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
        let e = anyhow!(
            "No redeemable positions found on EOA ({}) {}. \
             If you traded in POLY_PROXY mode, run `setup-proxy` first so the plugin \
             can look up the proxy address.",
            eoa_addr,
            proxy_addr
                .as_ref()
                .map(|p| format!("or proxy ({})", p))
                .unwrap_or_else(|| "(no proxy configured)".into())
        );
        println!("{}", super::error_response(&e, Some("redeem"), hint_opt));
        return Ok(());
    }

    let n = redeemable.len();
    eprintln!(
        "[polymarket] Found {} redeemable position(s). Redeeming sequentially...",
        n
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
                    "redeemable_count": n,
                    "estimated_pol_needed": n as f64 * POL_PER_REDEEM,
                    "discovered_proxy": discovered_proxy,
                    "positions": items,
                    "note": "dry-run: would redeem each position sequentially, waiting for on-chain confirmation between each."
                }
            }))?
        );
        return Ok(());
    }

    // Fail fast if EOA does not have enough POL to cover all redeems.
    let pol_balance = match check_pol_budget(&eoa_addr, n).await {
        Ok(b) => b,
        Err(e) => {
            println!("{}", super::error_response(&e, Some("redeem"), hint_opt));
            return Ok(());
        }
    };
    eprintln!(
        "[polymarket] POL budget OK: {:.4} POL available, ~{:.4} POL needed for {} redeem(s).",
        pol_balance,
        n as f64 * POL_PER_REDEEM,
        n
    );

    let mut results = Vec::new();
    let mut errors = Vec::new();

    for (i, (cid, title)) in redeemable.iter().enumerate() {
        eprintln!(
            "[polymarket] [{}/{}] Redeeming: {}",
            i + 1,
            n,
            title
        );
        match redeem_one(&client, cid, title, &eoa_addr, proxy_addr.as_deref()).await {
            Ok(r) => results.push(r),
            Err(e) => {
                eprintln!("[polymarket] Error redeeming {}: {:#}", cid, e);
                let classified: serde_json::Value = serde_json::from_str(
                    &super::error_response(&e, Some("redeem"), hint_opt),
                )
                .unwrap_or_else(|_| serde_json::json!({ "error": e.to_string() }));
                errors.push(serde_json::json!({
                    "condition_id": cid,
                    "title": title,
                    "error": classified.get("error"),
                    "error_code": classified.get("error_code"),
                    "suggestion": classified.get("suggestion"),
                }));
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
                "note": "USDC.e transferred to respective wallet(s) for all confirmed redemptions."
            }
        }))?
    );
    Ok(())
}
