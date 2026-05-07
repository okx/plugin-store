use anyhow::{anyhow, Context as _, Result};
use reqwest::Client;

use crate::api::{get_clob_market, get_gamma_market_by_slug, get_positions};
// load_credentials_for used via crate::config path below
use crate::config::Contracts;
use crate::onchainos::{
    ctf_redeem_positions, ctf_redeem_via_proxy, decimal_str_to_hex64, get_ctf_balance,
    get_ctf_balance_hex, ctf_get_collection_id_hex, ctf_get_position_id_hex,
    get_existing_proxy, get_pol_balance, get_wallet_address, negrisk_redeem_positions,
    wait_for_tx_receipt_labeled,
};

/// Auto-detect which collateral (USDC.e or pUSD) the position uses.
///
/// Background: CTF.positionId = keccak256(collateralToken || collectionId).
/// V1 markets use USDC.e; V2 (post-2026-04-28 cutover) use pUSD. Calling
/// `redeemPositions` with the wrong collateral computes a positionId the
/// wallet doesn't hold → CTF silently no-ops (no revert, no PayoutRedemption
/// event, status=0x1, ~35k gas). This function probes both candidates by
/// querying actual on-chain balances and returns the collateral that matches
/// at least one wallet's holdings.
///
/// Probes USDC.e first (most pre-cutover positions), falls back to pUSD.
async fn detect_collateral_for_position(
    condition_id: &str,
    candidate_wallets: &[&str],
) -> Result<&'static str> {
    // Binary CTF: indexSet 1 (Yes) and 2 (No) — the user's winning shares are
    // in exactly one of them. parentCollectionId = bytes32(0) for top-level markets.
    let parent_zero = format!("0x{}", "0".repeat(64));
    let mut collection_ids = Vec::with_capacity(2);
    for &index_set in &[1u32, 2u32] {
        let cid_hex = ctf_get_collection_id_hex(&parent_zero, condition_id, index_set).await?;
        collection_ids.push(cid_hex);
    }

    for collateral in [Contracts::USDC_E, Contracts::PUSD] {
        for collection_id_hex in &collection_ids {
            let position_id_hex =
                ctf_get_position_id_hex(collateral, collection_id_hex).await?;
            for &wallet in candidate_wallets {
                let bal = get_ctf_balance_hex(wallet, &position_id_hex)
                    .await
                    .unwrap_or(0);
                if bal > 0 {
                    return Ok(collateral);
                }
            }
        }
    }

    anyhow::bail!(
        "Could not detect collateral for conditionId {} — no CTF balance found for either USDC.e or pUSD across {} wallet(s). \
         Possibilities: market not resolved, winning tokens already redeemed, or shares are in a wallet this plugin does not know about.",
        condition_id,
        candidate_wallets.len()
    );
}

/// Per-redeem timeout (Polygon block time ~2s; a healthy tx mines in <30s).
/// Kept short so batch redeem stays under typical subprocess timeouts.
const REDEEM_WAIT_SECS: u64 = 45;

/// Estimated POL gas cost per redeem call (conservative).
/// CTF.redeemPositions on Polygon typically costs ~0.008 POL; we budget 2×
/// to absorb gas price spikes.
const POL_PER_REDEEM: f64 = 0.015;

/// Fire `onchainos wallet report-plugin-info` with a REDEEM payload.
/// No-op when strategy_id is missing/empty or no tx hashes are available.
async fn report_redeem(
    strategy_id: Option<&str>,
    eoa: &str,
    proxy: Option<&str>,
    condition_id: &str,
    result: &serde_json::Value,
) {
    let sid = match strategy_id.filter(|s| !s.is_empty()) {
        Some(s) => s,
        None => return,
    };
    let mut tx_hashes: Vec<String> = Vec::new();
    if let Some(t) = result.get("eoa_tx").and_then(|v| v.as_str()) {
        tx_hashes.push(t.to_string());
    }
    if let Some(t) = result.get("proxy_tx").and_then(|v| v.as_str()) {
        tx_hashes.push(t.to_string());
    }
    if tx_hashes.is_empty() {
        return;
    }
    let ts_now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cid_display = format!("0x{}", condition_id.trim_start_matches("0x"));
    let payload = serde_json::json!({
        "wallet": eoa,
        "proxyAddress": proxy.unwrap_or(""),
        "order_id": tx_hashes[0],
        "tx_hashes": tx_hashes,
        "market_id": cid_display,
        "asset_id": "",
        "side": "REDEEM",
        "amount": "",
        "symbol": "USDC.e",
        "price": "",
        "timestamp": ts_now,
        "strategy_id": sid,
        "plugin_name": "polymarket-plugin",
    });
    if let Err(e) = crate::onchainos::report_plugin_info(&payload).await {
        eprintln!("[polymarket] Warning: report-plugin-info failed: {}", e);
    }
}


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
    deposit_wallet: bool,
}

async fn check_redeemability(
    client: &Client,
    condition_id: &str,
    eoa_addr: &str,
    proxy_addr: Option<&str>,
    deposit_wallet_addr: Option<&str>,
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

    let deposit_wallet = if let Some(dw) = deposit_wallet_addr {
        let positions = get_positions(client, dw).await.unwrap_or_default();
        positions
            .iter()
            .any(|p| matches(p.condition_id.as_deref()) && p.redeemable)
    } else {
        false
    };

    Redeemability { eoa, proxy, deposit_wallet }
}

/// Core redeem logic for a single condition_id.
///
/// Never falls back — if Data API shows no redeemable positions on either
/// wallet, returns an error (caller should surface NO_REDEEMABLE_POSITIONS).
///
/// `collateral_addr`: USDC.e for V1 positions, pUSD for V2 positions.
///
/// Returns a JSON Value summarising the result (for use in both single and batch flows).
async fn redeem_one(
    client: &Client,
    condition_id: &str,
    question: &str,
    neg_risk: bool,
    token_ids: &[String],
    eoa_addr: &str,
    proxy_addr: Option<&str>,
    deposit_wallet_addr: Option<&str>,
    collateral_addr: &str,
) -> Result<serde_json::Value> {
    let cid_hex = condition_id.trim_start_matches("0x");
    let cid_display = format!("0x{}", cid_hex);

    let r = check_redeemability(client, condition_id, eoa_addr, proxy_addr, deposit_wallet_addr).await;

    if !r.eoa && !r.proxy && !r.deposit_wallet {
        return Err(anyhow!(
            "No redeemable positions found for {} on EOA ({}) {}{}. \
             Outcome tokens are held in a wallet this plugin does not know about — \
             if you traded in POLY_PROXY mode, run `setup-proxy` first so the plugin \
             can look up the proxy address.",
            cid_display,
            eoa_addr,
            proxy_addr
                .map(|p| format!("or proxy ({})", p))
                .unwrap_or_else(|| "(no proxy configured)".into()),
            deposit_wallet_addr
                .map(|d| format!(" or deposit wallet ({})", d))
                .unwrap_or_default()
        ));
    }

    let mut out = serde_json::json!({
        "condition_id": cid_display,
        "question": question,
        "neg_risk": neg_risk,
    });

    if neg_risk {
        // NegRisk markets: call NegRiskAdapter.redeemPositions(conditionId, [yes_bal, no_bal]).
        // Proxy-via-PROXY_FACTORY routing for neg_risk is not yet implemented; EOA only.
        if r.proxy && !r.eoa {
            return Err(anyhow!(
                "Neg_risk redeem from proxy wallet is not yet supported by this plugin. \
                 If your winning tokens are in the proxy wallet, use the Polymarket web UI \
                 to redeem. EOA redeem via NegRiskAdapter is fully supported."
            ));
        }

        // Query on-chain ERC-1155 balances for each outcome token.
        // Propagate RPC errors (don't unwrap_or(0)) — silently treating an RPC failure as
        // "no balance" would tell users their winning tokens don't exist when really the
        // node is just unavailable.
        let wallet = if r.proxy && proxy_addr.is_some() { proxy_addr.unwrap() } else { eoa_addr };
        let mut amounts: Vec<u128> = Vec::with_capacity(token_ids.len());
        for tid in token_ids {
            let bal = get_ctf_balance(wallet, tid).await
                .with_context(|| format!(
                    "Failed to query CTF balance for token_id {} in wallet {}. \
                     Polygon RPC may be unavailable — retry in a few seconds.",
                    tid, wallet
                ))?;
            amounts.push(bal);
        }

        // Validate we can encode the token IDs (catches malformed API data early).
        for tid in token_ids {
            decimal_str_to_hex64(tid)
                .with_context(|| format!("token_id '{}' is not a valid decimal integer", tid))?;
        }

        if amounts.iter().all(|&a| a == 0) {
            return Err(anyhow!(
                "No outcome token balance found on-chain for {} in wallet {}. \
                 The market may not be resolved yet, or winning tokens may be in a \
                 different wallet.",
                cid_display,
                wallet
            ));
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
        out["eoa_tx"] = serde_json::Value::String(tx);
        out["amounts"] = serde_json::Value::Array(
            amounts.iter().map(|a| serde_json::Value::String(a.to_string())).collect()
        );

        out["note"] = serde_json::Value::String(
            "NegRiskAdapter.redeemPositions confirmed. USDC.e transferred to EOA.".into(),
        );
    } else {
        // Standard binary market: call CTF.redeemPositions.
        if r.eoa {
            eprintln!("[polymarket] EOA holds winning tokens — submitting EOA redeemPositions...");
            let tx = ctf_redeem_positions(condition_id, collateral_addr).await?;
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
            let tx = ctf_redeem_via_proxy(condition_id, collateral_addr).await?;
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

        if r.deposit_wallet {
            let dw = deposit_wallet_addr.unwrap(); // safe: r.deposit_wallet implies Some
            eprintln!(
                "[polymarket] Deposit wallet holds winning tokens — submitting redeemPositions via relayer WALLET batch..."
            );
            // Fetch builder credentials for relayer auth.
            let clob_creds = crate::auth::ensure_credentials(client, eoa_addr).await
                .map_err(|e| anyhow::anyhow!("Could not load CLOB credentials for deposit wallet redeem: {}", e))?;
            let builder = crate::api::get_builder_api_key(client, &clob_creds, eoa_addr).await
                .map_err(|e| anyhow::anyhow!("Could not derive builder credentials for relayer: {}", e))?;
            let tx = crate::onchainos::ctf_redeem_via_deposit_wallet(
                condition_id, collateral_addr, dw, eoa_addr, &builder,
            ).await?;
            eprintln!(
                "[polymarket] Deposit wallet redeem tx {} — waiting up to {}s for confirmation...",
                tx, REDEEM_WAIT_SECS
            );
            wait_for_tx_receipt_labeled(&tx, REDEEM_WAIT_SECS, "Deposit wallet redeem").await?;
            out["deposit_wallet_tx"] = serde_json::Value::String(tx);
            out["deposit_wallet_note"] = serde_json::Value::String(
                "redeemPositions confirmed via deposit wallet. pUSD transferred to deposit wallet.".into(),
            );
        }

        out["note"] = serde_json::Value::String(
            "Collateral transferred to the respective wallet(s).".into(),
        );
    }

    Ok(out)
}

/// Look up an on-chain proxy wallet that is not yet recorded in credentials.
///
/// Safe to call freely: uses `debug_traceCall` (read-only, no gas, no tx). If the
/// RPC doesn't support `debug_traceCall` or anything else fails, returns None and
/// callers should fall through silently — this is purely a UX hint.
///
/// We only return a proxy if its bytecode is present on-chain. The trace can produce
/// a deterministic CREATE2 address even for un-deployed proxies; surfacing that in a
/// redeem hint would mislead the user into thinking they have a usable proxy.
async fn discover_uncached_proxy(eoa: &str, creds_proxy: Option<&str>) -> Option<String> {
    if creds_proxy.is_some() {
        return None;
    }
    get_existing_proxy(eoa).await.ok().flatten()
        .filter(|(_, exists)| *exists)
        .map(|(addr, _)| addr)
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
pub async fn run(market_id: &str, dry_run: bool, strategy_id: Option<&str>) -> Result<()> {
    let client = Client::new();

    let (condition_id, neg_risk, question) = match resolve_market(&client, market_id).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&e, Some("redeem"), None));
            return Ok(());
        }
    };

    // Fetch CLOB token IDs (needed for neg_risk on-chain balance queries).
    // For standard markets, tokens are also available but unused in the redeem path.
    let token_ids: Vec<String> = match get_clob_market(&client, &condition_id).await {
        Ok(m) => m.tokens.into_iter().map(|t| t.token_id).collect(),
        Err(_) => vec![],
    };

    let cid_display = format!("0x{}", condition_id.trim_start_matches("0x"));
    let eoa_addr = match get_wallet_address().await {
        Ok(a) => a,
        Err(e) => {
            println!("{}", super::error_response(&e, Some("redeem"), None));
            return Ok(());
        }
    };

    let creds = crate::config::load_credentials_for(&eoa_addr).ok().flatten();
    let proxy_addr = creds.as_ref().and_then(|c| c.proxy_wallet.clone());
    let deposit_wallet_addr = creds.and_then(|c| c.deposit_wallet);

    // Best-effort: if no proxy in creds, check on-chain so error hints can cite the address.
    let discovered_proxy = discover_uncached_proxy(&eoa_addr, proxy_addr.as_deref()).await;
    let hint = proxy_hint(discovered_proxy.as_deref());
    let hint_opt = if hint.is_empty() { None } else { Some(hint.as_str()) };

    // Deposit wallet mode is gasless (relayer-paid). EOA/proxy need POL.
    let needs_pol = deposit_wallet_addr.is_none();

    if dry_run {
        let r = check_redeemability(&client, &condition_id, &eoa_addr, proxy_addr.as_deref(), deposit_wallet_addr.as_deref()).await;
        let action = if neg_risk {
            "NegRiskAdapter.redeemPositions"
        } else {
            "CTF.redeemPositions"
        };

        let note = if neg_risk && r.proxy && !r.eoa {
            "dry-run WARNING: neg_risk redeem from proxy wallet is not yet supported by this plugin. \
             Use the Polymarket web UI to redeem proxy-held neg_risk positions."
        } else {
            "dry-run: will redeem from whichever wallet holds the winning tokens."
        };

        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "data": {
                    "dry_run": true,
                    "market_id": market_id,
                    "condition_id": cid_display,
                    "question": question,
                    "neg_risk": neg_risk,
                    "eoa_wallet": eoa_addr,
                    "proxy_wallet": proxy_addr,
                    "deposit_wallet": deposit_wallet_addr,
                    "discovered_proxy": discovered_proxy,
                    "eoa_redeemable": r.eoa,
                    "proxy_redeemable": r.proxy,
                    "deposit_wallet_redeemable": r.deposit_wallet,
                    "action": action,
                    "token_ids": token_ids,
                    "note": note,
                }
            }))?
        );
        return Ok(());
    }

    if needs_pol {
        if let Err(e) = check_pol_budget(&eoa_addr, 1).await {
            println!("{}", super::error_response(&e, Some("redeem"), hint_opt));
            return Ok(());
        }
    }

    // NegRisk markets route through NegRiskAdapter.redeemPositions — the CTF collateral
    // parameter is irrelevant and the standard positionId probe would fail anyway because
    // neg_risk markets use a non-zero parentCollectionId inside the NegRiskAdapter.
    // For standard binary markets, probe USDC.e vs pUSD to handle the V1/V2 cutover.
    let collateral_addr = if neg_risk {
        Contracts::USDC_E
    } else {
        let mut wallets: Vec<&str> = vec![&eoa_addr];
        if let Some(p) = proxy_addr.as_deref() { wallets.push(p); }
        if let Some(d) = deposit_wallet_addr.as_deref() { wallets.push(d); }
        match detect_collateral_for_position(&condition_id, &wallets).await {
            Ok(c) => c,
            Err(e) => {
                println!("{}", super::error_response(&e, Some("redeem"), hint_opt));
                return Ok(());
            }
        }
    };
    eprintln!(
        "[polymarket]   Detected CTF collateral: {} ({})",
        if collateral_addr == Contracts::PUSD { "pUSD" } else { "USDC.e" },
        collateral_addr
    );

    match redeem_one(&client, &condition_id, &question, neg_risk, &token_ids, &eoa_addr, proxy_addr.as_deref(), deposit_wallet_addr.as_deref(), collateral_addr).await {
        Ok(result) => {
            report_redeem(strategy_id, &eoa_addr, proxy_addr.as_deref(), &condition_id, &result).await;
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
pub async fn run_all(dry_run: bool, strategy_id: Option<&str>) -> Result<()> {
    let client = Client::new();
    let eoa_addr = match get_wallet_address().await {
        Ok(a) => a,
        Err(e) => {
            println!("{}", super::error_response(&e, Some("redeem"), None));
            return Ok(());
        }
    };

    let creds = crate::config::load_credentials_for(&eoa_addr).ok().flatten();
    let proxy_addr = creds.as_ref().and_then(|c| c.proxy_wallet.clone());
    let deposit_wallet_addr = creds.and_then(|c| c.deposit_wallet);

    // Best-effort discovery: if creds has no proxy but one exists on-chain,
    // surface it in error hints so the user knows `setup-proxy` is the fix.
    let discovered_proxy = discover_uncached_proxy(&eoa_addr, proxy_addr.as_deref()).await;
    let hint = proxy_hint(discovered_proxy.as_deref());
    let hint_opt = if hint.is_empty() { None } else { Some(hint.as_str()) };

    // Collect all unique redeemable condition_ids from all wallets.
    let mut redeemable: Vec<(String, String)> = Vec::new(); // (condition_id, title)

    let eoa_positions = get_positions(&client, &eoa_addr).await.unwrap_or_default();
    for p in &eoa_positions {
        if p.redeemable && p.current_value.unwrap_or(0.0) > 0.000_001 {
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
            if p.redeemable && p.current_value.unwrap_or(0.0) > 0.000_001 {
                if let Some(cid) = &p.condition_id {
                    let title = p.title.clone().unwrap_or_default();
                    if !redeemable.iter().any(|(c, _)| c == cid) {
                        redeemable.push((cid.clone(), title));
                    }
                }
            }
        }
    }

    if let Some(ref dw) = deposit_wallet_addr {
        let dw_positions = get_positions(&client, dw).await.unwrap_or_default();
        for p in &dw_positions {
            if p.redeemable && p.current_value.unwrap_or(0.0) > 0.000_001 {
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
            "No redeemable positions found on EOA ({}) {}{}. \
             If you traded in POLY_PROXY mode, run `setup-proxy` first so the plugin \
             can look up the proxy address.",
            eoa_addr,
            proxy_addr
                .as_ref()
                .map(|p| format!("or proxy ({})", p))
                .unwrap_or_else(|| "(no proxy configured)".into()),
            deposit_wallet_addr
                .as_ref()
                .map(|d| format!(" or deposit wallet ({})", d))
                .unwrap_or_default()
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
                    "estimated_pol_needed": if deposit_wallet_addr.is_none() { n as f64 * POL_PER_REDEEM } else { 0.0 },
                    "discovered_proxy": discovered_proxy,
                    "positions": items,
                    "note": "dry-run: would redeem each position sequentially, waiting for on-chain confirmation between each."
                }
            }))?
        );
        return Ok(());
    }

    // Deposit wallet redeems go through the relayer (no EOA gas required).
    // Only enforce the POL budget when the EOA/proxy path is active.
    let needs_pol = deposit_wallet_addr.is_none();
    if needs_pol {
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
    }

    let mut results = Vec::new();
    let mut errors = Vec::new();

    for (i, (cid, title)) in redeemable.iter().enumerate() {
        eprintln!(
            "[polymarket] [{}/{}] Redeeming: {}",
            i + 1,
            n,
            title
        );
        // Fetch neg_risk flag and token_ids for each market (needed for NegRisk redeem path).
        let (market_neg_risk, market_token_ids) = match get_clob_market(&client, cid).await {
            Ok(m) => (m.neg_risk, m.tokens.into_iter().map(|t| t.token_id).collect()),
            Err(_) => (false, vec![]),
        };

        // NegRisk markets route through NegRiskAdapter — collateral detection is irrelevant
        // and the standard positionId probe would fail anyway (non-zero parentCollectionId).
        // For standard binary markets, probe USDC.e vs pUSD to handle the V1/V2 cutover.
        let collateral_addr = if market_neg_risk {
            Contracts::USDC_E
        } else {
            let mut wallets: Vec<&str> = vec![&eoa_addr];
            if let Some(p) = proxy_addr.as_deref() { wallets.push(p); }
            if let Some(d) = deposit_wallet_addr.as_deref() { wallets.push(d); }
            match detect_collateral_for_position(cid, &wallets).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[polymarket] Collateral detection failed for {}: {:#}", cid, e);
                    let classified: serde_json::Value = serde_json::from_str(
                        &super::error_response(&e, Some("redeem"), hint_opt),
                    ).unwrap_or_else(|_| serde_json::json!({ "error": e.to_string() }));
                    errors.push(serde_json::json!({
                        "condition_id": cid,
                        "title": title,
                        "error": classified.get("error"),
                        "error_code": "COLLATERAL_NOT_DETECTED",
                        "suggestion": classified.get("suggestion"),
                    }));
                    continue;
                }
            }
        };
        eprintln!(
            "[polymarket]   Detected collateral: {} ({})",
            if collateral_addr == Contracts::PUSD { "pUSD (V2)" } else { "USDC.e (V1)" },
            collateral_addr
        );

        match redeem_one(&client, cid, title, market_neg_risk, &market_token_ids, &eoa_addr, proxy_addr.as_deref(), deposit_wallet_addr.as_deref(), collateral_addr).await {
            Ok(r) => {
                report_redeem(strategy_id, &eoa_addr, proxy_addr.as_deref(), cid, &r).await;
                results.push(r);
            }

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
                "note": "Collateral (pUSD/USDC.e) transferred to respective wallet(s) for all confirmed redemptions."
            }
        }))?
    );
    Ok(())
}
