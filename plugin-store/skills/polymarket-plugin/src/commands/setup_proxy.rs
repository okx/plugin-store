/// `polymarket setup-proxy` — create a Polymarket proxy wallet and switch to POLY_PROXY mode.
///
/// Flow:
///   1. Resolve the deterministic proxy address via `PROXY_FACTORY.proxy([])` debug_traceCall.
///      Also probes whether the address has bytecode → distinguishes "recovered" vs
///      "deployed_inline" (CREATE2 destination computed but not yet deployed).
///   2. Persist the proxy_wallet in creds.json (mode is set ONLY after approvals succeed).
///   3. Set up the 10 one-time approvals so trading is gasless:
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
/// For a fresh wallet (no proxy on-chain), the FIRST approve tx is also the deployment tx —
/// the factory deploys via CREATE2 and forwards the approve op atomically. The tx hash of
/// that first approve is surfaced as `deploy_tx` in the response.
///
/// Each approval is checked individually before submission — if a previous run partially
/// succeeded, only the missing approvals are sent. This is safe because all approvals
/// are independent and idempotent (re-approving with MAX_UINT is a no-op).
///
/// After setup, all subsequent buy/sell commands use POLY_PROXY mode (no POL for trading).
/// Run `polymarket switch-mode --mode eoa` to revert to EOA mode at any time.

use anyhow::{Context as _, Result};
use reqwest::Client;

/// Token type the proxy is approving from.
#[derive(Clone, Copy)]
enum ApproveToken {
    UsdcE,
    Pusd,
    CtfErc1155,
}

/// One element in the canonical approval list. Each is checked + sent independently.
struct Approval {
    token: ApproveToken,
    spender: &'static str,
    label: &'static str,
}

fn full_approval_list() -> Vec<Approval> {
    use crate::config::Contracts;
    vec![
        // V1 — USDC.e + CTF setApprovalForAll across the 3 exchange contracts
        Approval { token: ApproveToken::UsdcE,      spender: Contracts::CTF_EXCHANGE,          label: "V1 / USDC.e → CTF Exchange" },
        Approval { token: ApproveToken::CtfErc1155, spender: Contracts::CTF_EXCHANGE,          label: "V1 / CTF → CTF Exchange" },
        Approval { token: ApproveToken::UsdcE,      spender: Contracts::NEG_RISK_CTF_EXCHANGE, label: "V1 / USDC.e → Neg Risk CTF Exchange" },
        Approval { token: ApproveToken::CtfErc1155, spender: Contracts::NEG_RISK_CTF_EXCHANGE, label: "V1 / CTF → Neg Risk CTF Exchange" },
        Approval { token: ApproveToken::UsdcE,      spender: Contracts::NEG_RISK_ADAPTER,      label: "V1 / USDC.e → Neg Risk Adapter" },
        Approval { token: ApproveToken::CtfErc1155, spender: Contracts::NEG_RISK_ADAPTER,      label: "V1 / CTF → Neg Risk Adapter" },
        // V2 — pUSD spending allowance + USDC.e onramp wrapper
        Approval { token: ApproveToken::Pusd,       spender: Contracts::CTF_EXCHANGE_V2,          label: "V2 / pUSD → CTF Exchange V2" },
        Approval { token: ApproveToken::Pusd,       spender: Contracts::NEG_RISK_CTF_EXCHANGE_V2, label: "V2 / pUSD → Neg Risk CTF Exchange V2" },
        Approval { token: ApproveToken::Pusd,       spender: Contracts::NEG_RISK_ADAPTER,         label: "V2 / pUSD → Neg Risk Adapter" },
        Approval { token: ApproveToken::UsdcE,      spender: Contracts::COLLATERAL_ONRAMP,        label: "V2 / USDC.e → Collateral Onramp" },
    ]
}

struct ApprovalSubmission {
    label:   &'static str,
    tx_hash: String,
}

struct ApprovalReport {
    pre_existing: Vec<&'static str>,        // labels of approvals already on-chain
    newly_set:    Vec<ApprovalSubmission>,  // submitted in this run, with tx hashes
    would_set:    Vec<&'static str>,        // dry-run only: labels that would be submitted
}

impl ApprovalReport {
    fn was_no_op(&self) -> bool {
        self.newly_set.is_empty() && self.would_set.is_empty()
    }

    fn newly_set_labels(&self) -> Vec<&'static str> {
        self.newly_set.iter().map(|s| s.label).collect()
    }

    fn newly_set_json(&self) -> Vec<serde_json::Value> {
        self.newly_set.iter()
            .map(|s| serde_json::json!({ "label": s.label, "tx": s.tx_hash }))
            .collect()
    }
}

pub async fn run(dry_run: bool) -> Result<()> {
    match run_inner(dry_run).await {
        Ok(()) => Ok(()),
        Err(e) => {
            // GEN-001: emit structured error to stdout so external Agents can parse.
            println!("{}", super::error_response(&e, Some("setup-proxy"), None));
            Ok(())
        }
    }
}

async fn run_inner(dry_run: bool) -> Result<()> {
    let client = Client::new();

    // Geo check — WARNING only, do not abort. Users in restricted regions can still
    // set up a proxy wallet; trading commands (buy/sell) will hard-fail separately.
    if let Some(geo_msg) = crate::api::check_clob_access(&client).await {
        eprintln!("[polymarket] WARNING: {}", geo_msg);
        eprintln!("[polymarket] Continuing setup — proxy wallet creation does not require trading access.");
    }

    let signer_addr = crate::onchainos::get_wallet_address().await?;
    let mut creds = crate::auth::ensure_credentials(&client, &signer_addr).await?;

    // ── Branch A: cached creds say a proxy exists ─────────────────────────────
    if let Some(ref proxy) = creds.proxy_wallet {
        let proxy = proxy.clone();
        let was_in_proxy_mode = creds.mode == crate::config::TradingMode::PolyProxy;

        eprintln!("[polymarket] Proxy wallet found in creds: {}. Checking approvals...", proxy);
        let report = ensure_proxy_approvals(&proxy, dry_run).await
            .context("Failed to verify or set proxy approvals")?;

        // Save mode=PolyProxy ONLY after approvals are confirmed (atomicity Bug #8).
        if !dry_run && !was_in_proxy_mode {
            creds.mode = crate::config::TradingMode::PolyProxy;
            crate::config::save_credentials(&creds)?;
        }

        let status = if was_in_proxy_mode {
            if report.was_no_op() { "already_configured" } else { "approvals_topped_up" }
        } else {
            "mode_switched"
        };
        let note = match (status, dry_run) {
            ("already_configured", _) =>
                "Proxy wallet already configured. All 10 approvals confirmed on-chain.".to_string(),
            ("approvals_topped_up", true) =>
                format!("dry-run: would submit {} missing approval(s) on-chain (no state written).", report.would_set.len()),
            ("approvals_topped_up", false) =>
                format!("Proxy wallet was already set up but missing {} approval(s); they were just submitted on-chain.", report.newly_set.len()),
            ("mode_switched", true) =>
                "dry-run: would switch to POLY_PROXY mode and ensure approvals (no state written).".to_string(),
            ("mode_switched", false) =>
                format!("Switched to POLY_PROXY mode. {} new approval(s) submitted; {} were already set. Deposit USDC.e with `polymarket deposit --amount <N>`.",
                    report.newly_set.len(), report.pre_existing.len()),
            _ => String::new(),
        };

        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": dry_run,
                "data": {
                    "status": status,
                    "proxy_wallet": proxy,
                    "mode": "poly_proxy",
                    "approvals": {
                        "pre_existing": report.pre_existing,
                        "newly_set": report.newly_set_json(),
                        "would_set": report.would_set,
                    },
                    "note": note
                }
            })
        );
        return Ok(());
    }

    // ── Branch B: no cached proxy — probe on-chain ───────────────────────────
    // RPC failure here MUST be fatal; we cannot tell if a proxy exists.
    eprintln!("[polymarket] Probing PROXY_FACTORY for proxy wallet state...");
    let probe = crate::onchainos::get_existing_proxy(&signer_addr).await
        .map_err(|e| anyhow::anyhow!(
            "On-chain proxy lookup failed across all RPCs: {}. \
             Cannot safely determine proxy state. Retry when an RPC supporting \
             debug_traceCall is available (drpc.org, polygon-bor-rpc.publicnode.com).",
            e
        ))?;

    let (proxy_addr, exists_on_chain) = match probe {
        Some(pair) => pair,
        None => anyhow::bail!(
            "PROXY_FACTORY.proxy([]) trace returned no sub-call for signer {}. \
             This is unexpected for the current Polymarket factory; please retry or \
             report. Aborting to avoid unsafe assumptions about proxy state.",
            signer_addr
        ),
    };

    if exists_on_chain {
        eprintln!("[polymarket] Found existing proxy on-chain: {}", proxy_addr);
    } else {
        eprintln!(
            "[polymarket] Proxy address resolved: {} (not yet deployed — will deploy atomically with the first approve tx).",
            proxy_addr
        );
    }

    // ── Dry-run: query chain for actual approval state, but submit nothing ──
    // Use the same per-pair logic as the live path so the agent sees the precise
    // set of approvals that would actually be sent (not the static all-10 list).
    if dry_run {
        let report = ensure_proxy_approvals(&proxy_addr, true).await
            .context("Failed to query approval state for dry-run report")?;
        let status = if exists_on_chain { "would_top_up" } else { "would_create" };
        let note = if exists_on_chain {
            format!("dry-run: proxy on-chain at {}; {} approval(s) already set, {} would be submitted.",
                proxy_addr, report.pre_existing.len(), report.would_set.len())
        } else {
            format!("dry-run: proxy not yet deployed. setup-proxy would atomically deploy it via the first approve tx ({} approval(s) total — no separate deploy tx).",
                report.would_set.len())
        };
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "status": status,
                    "proxy_wallet": proxy_addr,
                    "proxy_exists_on_chain": exists_on_chain,
                    "would_deploy_with_first_approve": !exists_on_chain,
                    "approvals": {
                        "pre_existing": report.pre_existing,
                        "would_set": report.would_set,
                    },
                    "signer": signer_addr,
                    "note": note,
                }
            })
        );
        return Ok(());
    }

    // ── Persist proxy_wallet first so retry won't redeploy. Mode is delayed
    //     until approvals confirm, so a partial failure leaves us in a
    //     re-runnable state (Bug #8 atomicity).
    creds.proxy_wallet = Some(proxy_addr.clone());
    crate::config::save_credentials(&creds)?;

    let report = ensure_proxy_approvals(&proxy_addr, false).await
        .context(if exists_on_chain {
            "Failed to verify or set approvals on existing proxy"
        } else {
            "Approval flow failed before completion. Re-run setup-proxy — \
             per-pair idempotency will only retry the missing approvals."
        })?;

    creds.mode = crate::config::TradingMode::PolyProxy;
    crate::config::save_credentials(&creds)?;

    let status = if exists_on_chain { "recovered" } else { "deployed_inline" };
    let deploy_tx = if !exists_on_chain {
        report.newly_set.first().map(|s| s.tx_hash.clone())
    } else {
        None
    };
    let note = if exists_on_chain {
        format!(
            "Existing proxy wallet found on-chain and saved to creds. {} new approval(s) submitted; {} were already set.",
            report.newly_set.len(), report.pre_existing.len()
        )
    } else {
        format!(
            "Proxy wallet deployed atomically with the first approve tx ({}). {} approval(s) submitted in total.",
            deploy_tx.as_deref().unwrap_or("?"),
            report.newly_set.len()
        )
    };

    let mut data = serde_json::json!({
        "status": status,
        "proxy_wallet": proxy_addr,
        "mode": "poly_proxy",
        "approvals": {
            "pre_existing": report.pre_existing,
            "newly_set": report.newly_set_json(),
            "would_set": report.would_set,
        },
        "next_step": "Deposit USDC.e with: polymarket deposit --amount <N>",
        "note": note,
    });
    if let Some(dtx) = deploy_tx {
        data["deploy_tx"] = serde_json::json!(dtx);
    }

    println!(
        "{}",
        serde_json::json!({ "ok": true, "data": data })
    );
    Ok(())
}

/// Check each (token, spender) pair individually, only submitting the missing approvals.
/// Returns a report distinguishing what was already on-chain vs what was just sent.
///
/// Bug #3 fix: previously this function used the first approval's allowance as a "group
/// probe" — if that one was set, the function skipped the other 5 V1 / 3 V2 approvals.
/// A partial failure (e.g. tx 1 succeeds, tx 3 times out) would then be permanent: the
/// retry's group probe sees "already done" and never reapproves the missing ones.
///
/// Bug #6 fix: previously RPC errors on the allowance reads were swallowed by
/// `unwrap_or(0)`, causing the function to falsely report "not approved" and resubmit
/// all 10 approvals on a transient RPC blip (≈ $0.01 wasted POL per blip).
async fn ensure_proxy_approvals(proxy_addr: &str, dry_run: bool) -> Result<ApprovalReport> {
    let approvals = full_approval_list();
    let mut report = ApprovalReport {
        pre_existing: Vec::new(),
        newly_set:    Vec::new(),
        would_set:    Vec::new(),
    };

    for a in &approvals {
        let already_set = is_approval_set(proxy_addr, a).await
            .with_context(|| format!(
                "Could not verify '{}' allowance on-chain. Polygon RPC may be unavailable. \
                 Refusing to resubmit approvals blindly — re-run setup-proxy when the RPC \
                 is reachable.",
                a.label
            ))?;

        if already_set {
            report.pre_existing.push(a.label);
            continue;
        }

        if dry_run {
            report.would_set.push(a.label);
            continue;
        }

        eprintln!("[polymarket] Approving {} ...", a.label);
        let tx = submit_approval(a).await
            .with_context(|| format!("Submitting '{}' approval failed", a.label))?;
        eprintln!("[polymarket] tx: {}", tx);
        crate::onchainos::wait_for_tx_receipt(&tx, 60)
            .await
            .with_context(|| format!(
                "'{}' approval tx {} did not confirm in 60s. \
                 Re-run setup-proxy to verify and continue from where it left off.",
                a.label, tx
            ))?;
        report.newly_set.push(ApprovalSubmission { label: a.label, tx_hash: tx });
    }

    if dry_run {
        eprintln!(
            "[polymarket] dry-run: {} approval(s) already on-chain, {} would be submitted.",
            report.pre_existing.len(), report.would_set.len()
        );
    } else if report.newly_set.is_empty() {
        eprintln!("[polymarket] All {} approvals already on-chain — no action needed.", report.pre_existing.len());
    } else {
        eprintln!(
            "[polymarket] Approvals confirmed: {} new, {} pre-existing. (newly_set labels: {:?})",
            report.newly_set.len(),
            report.pre_existing.len(),
            report.newly_set_labels()
        );
    }
    Ok(report)
}

async fn is_approval_set(proxy_addr: &str, a: &Approval) -> Result<bool> {
    match a.token {
        ApproveToken::UsdcE => {
            let allowance = crate::onchainos::get_usdc_allowance(proxy_addr, a.spender).await?;
            Ok(allowance > 0)
        }
        ApproveToken::Pusd => {
            let allowance = crate::onchainos::get_pusd_allowance(proxy_addr, a.spender).await?;
            Ok(allowance > 0)
        }
        ApproveToken::CtfErc1155 => {
            crate::onchainos::is_ctf_approved_for_all(proxy_addr, a.spender).await
        }
    }
}

async fn submit_approval(a: &Approval) -> Result<String> {
    match a.token {
        ApproveToken::UsdcE      => crate::onchainos::proxy_usdc_approve(a.spender).await,
        ApproveToken::Pusd       => crate::onchainos::proxy_pusd_approve(a.spender).await,
        ApproveToken::CtfErc1155 => crate::onchainos::proxy_ctf_set_approval_for_all(a.spender).await,
    }
}
