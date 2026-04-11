use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::api::{
    compute_buy_worst_price, get_balance_allowance, get_clob_market, get_market_fee, get_orderbook,
    get_tick_size, post_order, round_amount_down, round_price, round_size_down, to_token_units,
    OrderBody, OrderRequest,
};
use crate::auth::ensure_credentials;
use crate::onchainos::{approve_usdc, get_wallet_address};
use crate::signing::{sign_order_via_onchainos, OrderParams};

/// Run the buy command.
pub async fn run(
    market_id: &str,
    outcome: &str,
    amount: &str,
    price: Option<f64>,
    order_type: &str,
    auto_approve: bool,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "market_id": market_id,
                    "outcome": outcome,
                    "amount": amount,
                    "note": "dry-run: order not submitted"
                }
            })
        );
        return Ok(());
    }

    let client = Client::new();

    // onchainos wallet is the signer (approved operator of proxy wallet after polymarket.com onboarding)
    let signer_addr = get_wallet_address().await?;

    // Derive API credentials for the onchainos wallet
    let creds = ensure_credentials(&client, &signer_addr).await?;

    // EOA mode (signature_type=0): maker = signer = onchainos wallet.
    // No proxy wallet or polymarket.com onboarding required.
    let maker_addr = signer_addr.clone();

    // Resolve market
    let (condition_id, token_id, neg_risk) =
        resolve_market_token(&client, market_id, outcome).await?;

    // Get tick size and market fee rate
    let tick_size = get_tick_size(&client, &token_id).await?;
    let fee_rate_bps = get_market_fee(&client, &condition_id).await.unwrap_or(0);

    // Parse USDC amount
    let usdc_amount: f64 = amount.parse().context("invalid amount")?;
    if usdc_amount <= 0.0 {
        bail!("amount must be positive");
    }

    // Determine price (limit or market)
    let limit_price = if let Some(p) = price {
        if p <= 0.0 || p >= 1.0 {
            bail!("price must be in range (0, 1)");
        }
        let rp = round_price(p, tick_size);
        if rp <= 0.0 || rp >= 1.0 {
            bail!("price {p} rounds to {rp} with tick size {tick_size} — out of range (0, 1)");
        }
        rp
    } else {
        let book = get_orderbook(&client, &token_id).await?;
        compute_buy_worst_price(&book.asks, usdc_amount)
            .ok_or_else(|| anyhow::anyhow!("No asks available in the order book"))?
    };

    // Check USDC allowance and auto-approve if needed
    use crate::config::Contracts;
    let exchange_addr = Contracts::exchange_for(neg_risk);
    let allowance_info =
        get_balance_allowance(&client, &signer_addr, &creds, "COLLATERAL", None).await?;
    let allowance_raw = allowance_info.allowance_for(exchange_addr);
    let usdc_needed_raw = to_token_units(usdc_amount);

    if allowance_raw < usdc_needed_raw || auto_approve {
        eprintln!("[polymarket] Approving {} USDC.e for CTF Exchange...", usdc_amount);
        let tx_hash = approve_usdc(neg_risk, usdc_needed_raw).await?;
        eprintln!("[polymarket] Approval tx: {}", tx_hash);
    }

    // Build order amounts — compute shares first, then back-compute USDC from
    // price × rounded_shares to guarantee the maker/taker ratio is exactly limit_price.
    // (Rounding shares and USDC independently produces a skewed ratio that fails API
    // tick-size validation, e.g. 10_000_000 / 10_200_000 = 50/51 ≠ 0.98.)
    let shares = usdc_amount / limit_price;
    let rounded_shares = round_size_down(shares);
    let taker_amount_raw = to_token_units(rounded_shares);
    let actual_usdc = limit_price * rounded_shares;
    let rounded_usdc = round_amount_down(actual_usdc, tick_size);
    let maker_amount_raw = to_token_units(rounded_usdc);

    let salt = rand_salt();

    let params = OrderParams {
        salt,
        maker: maker_addr.clone(),    // EOA mode: maker = signer = onchainos wallet
        signer: signer_addr.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw,
        taker_amount: taker_amount_raw,
        expiration: 0,
        nonce: 0,
        fee_rate_bps,
        side: 0, // BUY
        signature_type: 0, // EOA
    };

    let signature = sign_order_via_onchainos(&params, neg_risk).await?;

    let order_body = OrderBody {
        salt,  // serialized as JSON number per clob-client spec
        maker: maker_addr.clone(),
        signer: signer_addr.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw.to_string(),
        taker_amount: taker_amount_raw.to_string(),
        expiration: "0".to_string(),
        nonce: "0".to_string(),
        fee_rate_bps: fee_rate_bps.to_string(),
        side: "BUY".to_string(),
        signature_type: 0,
        signature,
    };

    let order_req = OrderRequest {
        order: order_body,
        owner: creds.api_key.clone(),
        order_type: order_type.to_uppercase(),
        post_only: false,
    };

    let resp = post_order(&client, &signer_addr, &creds, &order_req).await?;

    if resp.success != Some(true) {
        let msg = resp.error_msg.as_deref().unwrap_or("unknown error");
        bail!("Order placement failed: {}", msg);
    }

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "order_id": resp.order_id,
            "status": resp.status,
            "condition_id": condition_id,
            "outcome": outcome,
            "token_id": token_id,
            "side": "BUY",
            "order_type": order_type.to_uppercase(),
            "limit_price": limit_price,
            "usdc_amount": rounded_usdc,
            "shares": rounded_shares,
            "tx_hashes": resp.tx_hashes,
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Resolve (condition_id, token_id, neg_risk) from a market_id and outcome string.
/// Supports any outcome label (e.g. "yes", "no", "trump", "republican", "option-a").
pub async fn resolve_market_token(
    client: &Client,
    market_id: &str,
    outcome: &str,
) -> Result<(String, String, bool)> {
    let outcome_lower = outcome.to_lowercase();
    if market_id.starts_with("0x") || market_id.starts_with("0X") {
        let market = get_clob_market(client, market_id).await?;
        let token = market
            .tokens
            .iter()
            .find(|t| t.outcome.to_lowercase() == outcome_lower)
            .ok_or_else(|| {
                let available: Vec<&str> = market.tokens.iter().map(|t| t.outcome.as_str()).collect();
                anyhow::anyhow!("Outcome '{}' not found. Available outcomes: {:?}", outcome, available)
            })?;
        Ok((market.condition_id.clone(), token.token_id.clone(), market.neg_risk))
    } else {
        let gamma = crate::api::get_gamma_market_by_slug(client, market_id).await?;
        let condition_id = gamma
            .condition_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No condition_id in Gamma market response"))?;
        let token_ids = gamma.token_ids();
        let outcomes = gamma.outcome_list();
        let idx = outcomes
            .iter()
            .position(|o| o.to_lowercase() == outcome_lower)
            .ok_or_else(|| {
                anyhow::anyhow!("Outcome '{}' not found. Available outcomes: {:?}", outcome, outcomes)
            })?;
        let token_id = token_ids
            .get(idx)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No token_id for outcome index {}", idx))?;
        Ok((condition_id, token_id, gamma.neg_risk))
    }
}

/// Generate a random salt within JavaScript's safe integer range (< 2^53).
/// The clob-client sends salt as a JSON number (Number.parseInt), so we must
/// ensure no precision loss on the server side.
fn rand_salt() -> u64 {
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    // Mask to 53 bits = 0x1FFFFFFFFFFFFF
    u64::from_le_bytes(bytes) & 0x001F_FFFF_FFFF_FFFF
}
