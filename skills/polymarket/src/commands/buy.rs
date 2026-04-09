use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::api::{
    compute_buy_worst_price, get_balance_allowance, get_clob_market, get_orderbook,
    get_tick_size, post_order, round_amount_down, round_price, round_size_down, to_token_units,
    OrderBody, OrderRequest,
};
use crate::auth::ensure_credentials;
use crate::config::{get_or_create_signing_key, signing_key_address};
use crate::onchainos::{approve_usdc_max, ensure_operator_approval, get_wallet_address};
use crate::signing::{sign_order, OrderParams};

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

    // Load/generate local signing key and derive its address
    let signing_key = get_or_create_signing_key()?;
    let signer_addr = signing_key_address(&signing_key);

    // Resolve onchainos wallet (holds USDC.e)
    let wallet_addr = get_wallet_address().await?;

    // Ensure local signing key is approved as operator for the onchainos wallet
    ensure_operator_approval(&wallet_addr, &signer_addr, false).await?;

    // Get/derive credentials via local signing key (no onchainos EIP-712)
    let creds = ensure_credentials(&client, &signing_key).await?;

    // Resolve market
    let (condition_id, token_id, neg_risk) =
        resolve_market_token(&client, market_id, outcome).await?;

    // Get tick size
    let tick_size = get_tick_size(&client, &token_id).await?;

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
        round_price(p, tick_size)
    } else {
        let book = get_orderbook(&client, &token_id).await?;
        compute_buy_worst_price(&book.asks, usdc_amount)
            .ok_or_else(|| anyhow::anyhow!("No asks available in the order book"))?
    };

    // Check USDC allowance and auto-approve if needed
    let allowance_info =
        get_balance_allowance(&client, &signer_addr, &creds, "COLLATERAL", None).await?;
    let allowance_raw = allowance_info
        .allowance
        .as_deref()
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);
    let usdc_needed_raw = to_token_units(usdc_amount);

    if allowance_raw < usdc_needed_raw || auto_approve {
        eprintln!("[polymarket] Approving USDC.e for CTF Exchange...");
        let tx_hash = approve_usdc_max(neg_risk).await?;
        eprintln!("[polymarket] Approval tx: {}", tx_hash);
    }

    // Build order amounts
    let rounded_usdc = round_amount_down(usdc_amount, tick_size);
    let maker_amount_raw = to_token_units(rounded_usdc);
    let shares = rounded_usdc / limit_price;
    let rounded_shares = round_size_down(shares);
    let taker_amount_raw = to_token_units(rounded_shares);

    let salt = rand_salt();

    let params = OrderParams {
        salt,
        maker: wallet_addr.clone(), // onchainos wallet holds USDC.e
        signer: signer_addr.clone(), // local key signs the order
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw,
        taker_amount: taker_amount_raw,
        expiration: 0,
        nonce: 0,
        fee_rate_bps: 0,
        side: 0, // BUY
        signature_type: 0, // EOA
    };

    let signature = sign_order(&signing_key, &params, neg_risk)?;

    let order_body = OrderBody {
        salt: salt.to_string(),
        maker: wallet_addr.clone(),
        signer: signer_addr.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw.to_string(),
        taker_amount: taker_amount_raw.to_string(),
        expiration: "0".to_string(),
        nonce: "0".to_string(),
        fee_rate_bps: "0".to_string(),
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
pub async fn resolve_market_token(
    client: &Client,
    market_id: &str,
    outcome: &str,
) -> Result<(String, String, bool)> {
    if market_id.starts_with("0x") || market_id.starts_with("0X") {
        let market = get_clob_market(client, market_id).await?;
        let is_yes = outcome.to_lowercase() == "yes";
        let token = market
            .tokens
            .iter()
            .find(|t| {
                if is_yes {
                    t.outcome.to_lowercase() == "yes"
                } else {
                    t.outcome.to_lowercase() == "no"
                }
            })
            .ok_or_else(|| anyhow::anyhow!("Could not find {} token in market", outcome))?;
        Ok((market.condition_id.clone(), token.token_id.clone(), market.neg_risk))
    } else {
        let gamma = crate::api::get_gamma_market_by_slug(client, market_id).await?;
        let condition_id = gamma
            .condition_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No condition_id in Gamma market response"))?;
        let token_ids = gamma.token_ids();
        let outcomes = gamma.outcome_list();
        let is_yes = outcome.to_lowercase() == "yes";
        let idx = outcomes
            .iter()
            .position(|o| {
                if is_yes {
                    o.to_lowercase() == "yes"
                } else {
                    o.to_lowercase() == "no"
                }
            })
            .ok_or_else(|| anyhow::anyhow!("Could not find {} outcome in market", outcome))?;
        let token_id = token_ids
            .get(idx)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No token_id for outcome index {}", idx))?;
        Ok((condition_id, token_id, gamma.neg_risk))
    }
}

fn rand_salt() -> u128 {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    u128::from_le_bytes(bytes)
}
