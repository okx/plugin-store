use anyhow::Result;
use reqwest::Client;

use crate::api::get_positions;
use crate::onchainos::get_wallet_address;

pub async fn run(address: Option<&str>) -> Result<()> {
    let client = Client::new();

    let wallet_addr = match address {
        Some(a) => a.to_string(),
        None => get_wallet_address().await?,
    };

    let positions = get_positions(&client, &wallet_addr).await?;

    let output: Vec<serde_json::Value> = positions
        .iter()
        .map(|p| {
            // Annotate redeemable positions so agents can distinguish winning from losing.
            // redeemable: true with current_value ≈ 0 means the market resolved against
            // this outcome — redeeming would cost gas and receive nothing.
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
                "outcome": p.outcome,
                "outcome_index": p.outcome_index,
                "condition_id": p.condition_id,
                "token_id": p.asset,
                "size": p.size,
                "avg_price": p.avg_price,
                "cur_price": p.cur_price,
                "current_value": p.current_value,
                "cash_pnl": p.cash_pnl,
                "percent_pnl": p.percent_pnl,
                "realized_pnl": p.realized_pnl,
                "redeemable": p.redeemable,
                "redeemable_note": redeemable_note,
                "end_date": p.end_date,
                "negative_risk": p.negative_risk,
            })
        })
        .collect();

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "wallet": wallet_addr,
            "position_count": output.len(),
            "positions": output,
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
