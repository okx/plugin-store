use clap::Args;
use std::collections::HashMap;
use crate::api::{get_spot_clearinghouse_state, get_spot_meta, get_all_mids};
use crate::config::{info_url, CHAIN_ID};
use crate::onchainos::resolve_wallet;

#[derive(Args)]
pub struct SpotBalancesArgs {
    /// Wallet address to query (default: connected onchainos wallet)
    #[arg(long)]
    pub address: Option<String>,

    /// Include zero balances (default: hide them)
    #[arg(long)]
    pub show_zero: bool,
}

pub async fn run(args: SpotBalancesArgs) -> anyhow::Result<()> {
    let info = info_url();

    let address = match &args.address {
        Some(a) => a.clone(),
        None => resolve_wallet(CHAIN_ID)?,
    };

    // Fetch in parallel
    let (state, spot_meta, mids) = tokio::try_join!(
        get_spot_clearinghouse_state(info, &address),
        get_spot_meta(info),
        get_all_mids(info),
    )?;

    let empty_vec = vec![];
    let balances = state["balances"].as_array().unwrap_or(&empty_vec);
    let tokens = spot_meta["tokens"].as_array().unwrap_or(&empty_vec);
    let universe = spot_meta["universe"].as_array().unwrap_or(&empty_vec);

    // Build token_index -> market_index map (for mid price lookup via "@{market_idx}")
    let tok_to_mkt: HashMap<usize, usize> = universe
        .iter()
        .filter_map(|m| {
            let base_idx = m["tokens"].as_array()?.first()?.as_u64()? as usize;
            let mkt_idx = m["index"].as_u64()? as usize;
            Some((base_idx, mkt_idx))
        })
        .collect();

    // Build token_index -> token name map for display
    let tok_name: HashMap<usize, &str> = tokens
        .iter()
        .filter_map(|t| {
            let idx = t["index"].as_u64()? as usize;
            let name = t["name"].as_str()?;
            Some((idx, name))
        })
        .collect();

    // USDC token index is always 0
    const USDC_IDX: usize = 0;

    let mut output_balances = Vec::new();
    let mut total_usd = 0.0_f64;

    for bal in balances {
        let total: f64 = bal["total"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let hold: f64 = bal["hold"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);

        if !args.show_zero && total == 0.0 {
            continue;
        }

        let tok_idx = bal["token"].as_u64().unwrap_or(0) as usize;
        let coin = bal["coin"].as_str().unwrap_or(
            tok_name.get(&tok_idx).copied().unwrap_or("?"),
        );

        let (price_usd, usd_value) = if tok_idx == USDC_IDX {
            (1.0_f64, total)
        } else if let Some(&mkt_idx) = tok_to_mkt.get(&tok_idx) {
            let key = format!("@{}", mkt_idx);
            let price: f64 = mids
                .get(&key)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            (price, total * price)
        } else {
            (0.0, 0.0)
        };

        total_usd += usd_value;

        let available = total - hold;
        output_balances.push(serde_json::json!({
            "coin": coin,
            "total": format!("{}", total),
            "available": format!("{}", available),
            "hold": format!("{}", hold),
            "priceUsd": format!("{:.6}", price_usd),
            "valueUsd": format!("{:.2}", usd_value)
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "address": address,
            "totalValueUsd": format!("{:.2}", total_usd),
            "balances": output_balances
        }))?
    );

    Ok(())
}
