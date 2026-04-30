use anyhow::{bail, Result};
use reqwest::Client;

use crate::api::{
    get_clob_version, get_rfq_quote, post_rfq_confirm, post_rfq_request, OrderBodyV2,
};
use crate::auth::ensure_credentials;
use crate::config::OrderVersion;
use crate::onchainos::get_wallet_address;
use crate::signing::{sign_order_v2_via_onchainos, OrderParamsV2, BYTES32_ZERO};

use super::buy::{resolve_from_gamma, resolve_market_token};

/// Request-for-Quote (RFQ) for a block trade with a Polymarket market maker.
///
/// RFQ is designed for large orders where standard CLOB liquidity may be insufficient.
/// A market maker provides a firm quote; the user can accept it by re-running with `--confirm`.
///
/// Flow:
///   1. POST /rfq/request → receive a quote_id
///   2. GET /rfq/quote/{quote_id} → display price, amount, expiry
///   3. Re-run with --confirm → sign a V2 order at the quoted price, POST /rfq/confirm
pub async fn run(
    market_id: &str,
    outcome: &str,
    amount: &str,
    confirm: bool,
    dry_run: bool,
    strategy_id: Option<&str>,
) -> Result<()> {
    match run_inner(market_id, outcome, amount, confirm, dry_run, strategy_id).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("rfq"), None)); Ok(()) }
    }
}

async fn run_inner(
    market_id: &str,
    outcome: &str,
    amount: &str,
    confirm: bool,
    dry_run: bool,
    strategy_id: Option<&str>,
) -> Result<()> {
    let usdc_amount: f64 = amount.parse().map_err(|_| anyhow::anyhow!("invalid amount: {}", amount))?;
    if usdc_amount <= 0.0 {
        bail!("amount must be positive");
    }

    let client = Client::new();

    // Resolve market — supports condition_id, slug, or series ID (e.g. btc-5m).
    let (condition_id, token_id, neg_risk, _fee) =
        if crate::series::is_series_id(market_id) {
            let gamma = crate::series::resolve_to_market(&client, market_id).await?;
            resolve_from_gamma(&client, gamma, outcome).await?
        } else {
            resolve_market_token(&client, market_id, outcome).await?
        };

    let side = "BUY"; // RFQ always requests the buy side; sell-side RFQ uses the counterparty flow

    if dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "condition_id": condition_id,
                    "token_id": token_id,
                    "outcome": outcome,
                    "side": side,
                    "amount_usdc": usdc_amount,
                    "note": "dry-run: would POST /rfq/request and display the quote"
                }
            }))?
        );
        return Ok(());
    }

    // Step 1: request a quote.
    eprintln!("[polymarket] Requesting RFQ quote for {} {} @ ${:.2}...", side, outcome, usdc_amount);
    let quote_id = post_rfq_request(&client, &condition_id, &token_id, side, usdc_amount).await?;
    eprintln!("[polymarket] Quote ID: {}", quote_id);

    // Step 2: fetch the quote.
    let quote = get_rfq_quote(&client, &quote_id).await?;

    let price_str = quote.price.as_deref().unwrap_or("?");
    let amount_str = quote.amount.as_deref().unwrap_or("?");
    let expires_at = quote.expires_at.unwrap_or(0);
    let maker = quote.maker.as_deref().unwrap_or("unknown");
    let status = quote.status.as_deref().unwrap_or("pending");

    // Show the quote.
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "quote_id": quote_id,
                "status": status,
                "condition_id": condition_id,
                "outcome": outcome,
                "side": side,
                "price": price_str,
                "amount_usdc": amount_str,
                "maker": maker,
                "expires_at": expires_at,
                "note": if confirm {
                    "Confirming quote — signing and submitting order..."
                } else {
                    "Quote received. Re-run with --confirm to accept."
                }
            }
        }))?
    );

    if !confirm {
        return Ok(());
    }

    // Step 3: confirm the quote — sign a V2 order at the quoted price.
    let quoted_price: f64 = quote.price.as_deref()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("Quote has no valid price"))?;

    if quoted_price <= 0.0 || quoted_price >= 1.0 {
        bail!("Quoted price {} is out of range (0, 1)", quoted_price);
    }

    if status != "active" && status != "pending" {
        bail!("Quote is no longer active (status: {}). Request a new quote.", status);
    }

    let signer_addr = get_wallet_address().await?;
    let creds = ensure_credentials(&client, &signer_addr).await?;

    // Confirm flow always uses V2 signing (RFQ is a V2-only feature).
    let clob_version_raw = get_clob_version(&client).await?;
    let _clob_version = if clob_version_raw == 2 { OrderVersion::V2 } else { OrderVersion::V1 };

    // Resolve maker address from trading mode.
    use crate::config::TradingMode;
    let (maker_addr, sig_type) = match &creds.mode {
        TradingMode::PolyProxy => {
            let proxy = creds.proxy_wallet.as_ref()
                .ok_or_else(|| anyhow::anyhow!(
                    "POLY_PROXY mode requires a proxy wallet. Run `polymarket setup-proxy` first."
                ))?.clone();
            (proxy, 1u8)
        }
        TradingMode::Eoa => (signer_addr.clone(), 0u8),
    };

    // Compute amounts from quoted price and usdc amount.
    let maker_amount_raw = (usdc_amount * 1_000_000.0).round() as u64;
    let taker_amount_raw = (usdc_amount / quoted_price * 1_000_000.0).round() as u64;

    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Generate random salt.
    let mut salt_bytes = [0u8; 8];
    getrandom::getrandom(&mut salt_bytes).expect("getrandom failed");
    let salt = u64::from_le_bytes(salt_bytes) & 0x001F_FFFF_FFFF_FFFF;

    let params = OrderParamsV2 {
        salt,
        maker: maker_addr.clone(),
        signer: signer_addr.clone(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw,
        taker_amount: taker_amount_raw,
        side: 0, // BUY
        signature_type: sig_type,
        timestamp_ms,
        metadata: BYTES32_ZERO.to_string(),
        builder: BYTES32_ZERO.to_string(),
    };

    eprintln!("[polymarket] Signing RFQ order at price {}...", quoted_price);
    let signature = sign_order_v2_via_onchainos(&params, neg_risk).await?;

    let order_body = OrderBodyV2 {
        salt,
        maker: maker_addr.clone(),
        signer: signer_addr.clone(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw.to_string(),
        taker_amount: taker_amount_raw.to_string(),
        side: "BUY".to_string(),
        signature_type: sig_type,
        timestamp: timestamp_ms.to_string(),
        metadata: BYTES32_ZERO.to_string(),
        builder: BYTES32_ZERO.to_string(),
        signature,
    };

    eprintln!("[polymarket] Submitting RFQ confirmation...");
    let result = post_rfq_confirm(&client, &signer_addr, &creds, &quote_id, &order_body).await?;

    // Attribution: report every RFQ confirm that produced an order_id (CLOB-shaped response).
    // strategy_id is optional — empty string when not provided so backend still receives a record.
    // RFQ-specific quote_id is included as a supplementary field for cross-system correlation.
    let rfq_order_id = result.get("orderID").and_then(|v| v.as_str())
        .or_else(|| result.get("order_id").and_then(|v| v.as_str()))
        .unwrap_or("");
    if !rfq_order_id.is_empty() {
        let tx_hashes: Vec<String> = result.get("transactionsHashes")
            .or_else(|| result.get("tx_hashes"))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let ts_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let sid = strategy_id.unwrap_or("");
        let actual_shares = taker_amount_raw as f64 / 1_000_000.0;
        let report_payload = serde_json::json!({
            "wallet": signer_addr,
            "proxyAddress": creds.proxy_wallet.as_deref().unwrap_or(""),
            "order_id": rfq_order_id,
            "tx_hashes": tx_hashes,
            "market_id": condition_id,
            "asset_id": token_id,
            "side": "BUY",
            "amount": format!("{}", actual_shares),
            "symbol": "USDC.e",
            "price": format!("{}", quoted_price),
            "timestamp": ts_now,
            "strategy_id": sid,
            "rfq_quote_id": quote_id,
            "plugin_name": "polymarket-plugin",
        });
        if let Err(e) = crate::onchainos::report_plugin_info(&report_payload).await {
            eprintln!("[polymarket] Warning: report-plugin-info failed: {}", e);
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "quote_id": quote_id,
                "condition_id": condition_id,
                "outcome": outcome,
                "price": quoted_price,
                "usdc_amount": usdc_amount,
                "result": result,
            }
        }))?
    );
    Ok(())
}
