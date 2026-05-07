use anyhow::Result;
use reqwest::Client;

use crate::api::get_positions;
use crate::onchainos::{get_pol_balance, get_usdc_balance, get_wallet_address};

pub async fn run(address: Option<&str>) -> Result<()> {
    match run_inner(address).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("get-positions"), None)); Ok(()) }
    }
}

async fn run_inner(address: Option<&str>) -> Result<()> {
    let client = Client::new();

    // Determine which wallet to query and whether to show EOA balances.
    //
    // Rules (when no --address override):
    //   - proxy wallet exists in creds → query proxy wallet (no balance info needed;
    //     proxy holds CTF tokens and USDC.e managed by the relayer)
    //   - no proxy wallet → query EOA + show POL and USDC.e balances
    //
    // If --address is explicitly provided, use it as-is without balance augmentation.
    let (wallet_addr, show_eoa_balances) = if let Some(a) = address {
        (a.to_string(), false)
    } else {
        let eoa = get_wallet_address().await?;
        let creds = crate::config::load_credentials_for(&eoa).ok().flatten();
        let use_proxy = creds.as_ref().and_then(|c| match c.mode {
            crate::config::TradingMode::PolyProxy => c.proxy_wallet.clone(),
            crate::config::TradingMode::DepositWallet => c.deposit_wallet.clone(),
            _ => None,
        });
        match use_proxy {
            Some(p) => (p, false),
            None    => (eoa, true),
        }
    };

    let positions = get_positions(&client, &wallet_addr).await?;

    let output: Vec<serde_json::Value> = positions
        .iter()
        // Filter out resolved losing positions: redeemable but worth $0.
        // The Data API does not clear these after on-chain redeem — they persist indefinitely
        // as noise. Winning redeemable positions (current_value > 0) are always kept.
        .filter(|p| {
            let is_zero_value_resolved = p.redeemable
                && p.current_value.unwrap_or(0.0) < 0.000_001;
            !is_zero_value_resolved
        })
        .map(|p| {
            let redeemable_value = p.current_value.unwrap_or(0.0);
            let redeemable_note = if p.redeemable && redeemable_value < 0.000_001 {
                Some("resolved — losing outcome, redemption would receive $0")
            } else if p.redeemable {
                Some("resolved — winning outcome, redeem to collect USDC.e")
            } else {
                None
            };
            serde_json::json!({
                "title": p.title,
                "slug": p.slug,
                "icon": p.icon,
                "event_id": p.event_id,
                "event_slug": p.event_slug,
                "outcome": p.outcome,
                "outcome_index": p.outcome_index,
                "opposite_outcome": p.opposite_outcome,
                "opposite_asset": p.opposite_asset,
                "condition_id": p.condition_id,
                "token_id": p.asset,
                "size": p.size,
                "avg_price": p.avg_price,
                "initial_value": p.initial_value,
                "total_bought": p.total_bought,
                "cur_price": p.cur_price,
                "current_value": p.current_value,
                "cash_pnl": p.cash_pnl,
                "percent_pnl": p.percent_pnl,
                "realized_pnl": p.realized_pnl,
                "percent_realized_pnl": p.percent_realized_pnl,
                "redeemable": p.redeemable,
                "redeemable_note": redeemable_note,
                "mergeable": p.mergeable,
                "end_date": p.end_date,
                "negative_risk": p.negative_risk,
            })
        })
        .collect();

    let data = if show_eoa_balances {
        // Fetch POL and USDC.e balances in parallel
        let (pol_result, usdc_result) = tokio::join!(
            get_pol_balance(&wallet_addr),
            get_usdc_balance(&wallet_addr),
        );
        let pol_balance = match pol_result {
            Ok(v)  => format!("{:.4} POL", v),
            Err(e) => format!("error: {}", e),
        };
        let usdc_balance = match usdc_result {
            Ok(v)  => format!("${:.2}", v),
            Err(e) => format!("error: {}", e),
        };
        serde_json::json!({
            "wallet": wallet_addr,
            "mode": "eoa",
            "pol_balance": pol_balance,
            "usdc_e_balance": usdc_balance,
            "position_count": output.len(),
            "positions": output,
        })
    } else {
        // Add mode metadata and a helpful note where applicable.
        let eoa_for_mode = get_wallet_address().await.unwrap_or_default();
        let creds = crate::config::load_credentials_for(&eoa_for_mode).ok().flatten();
        let (mode_label, mode_note): (&str, Option<String>) = match creds.as_ref().map(|c| &c.mode) {
            Some(crate::config::TradingMode::PolyProxy) => ("proxy", None),
            Some(crate::config::TradingMode::DepositWallet) => {
                let dw = creds.as_ref().and_then(|c| c.deposit_wallet.as_deref()).unwrap_or("unknown");
                ("deposit_wallet", Some(format!(
                    "DEPOSIT_WALLET mode: positions are held at the deposit wallet ({dw}). \
                     Use `polymarket redeem` to collect resolved winnings."
                )))
            }
            Some(crate::config::TradingMode::Eoa) if output.is_empty() => {
                let has_proxy = creds.as_ref().and_then(|c| c.proxy_wallet.as_ref()).is_some();
                if has_proxy {
                    ("eoa", Some("Currently in EOA mode. If you placed orders in POLY_PROXY mode, \
                          switch with `polymarket switch-mode --mode proxy` to see those positions.".to_string()))
                } else { ("eoa", None) }
            }
            _ => ("eoa", None),
        };
        let mut obj = serde_json::json!({
            "wallet": wallet_addr,
            "mode": mode_label,
            "position_count": output.len(),
            "positions": output,
        });
        if let Some(note) = mode_note {
            obj["note"] = serde_json::Value::String(note);
        }
        obj
    };

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": data,
    }))?);
    Ok(())
}
