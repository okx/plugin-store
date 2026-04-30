use anyhow::Result;
use crate::api;
use crate::signal::{
    Direction, SignalCard,
    build_thesis, build_supporting_factors, build_risk_factors, classify_conviction,
};
use chrono::Timelike;

pub struct Scout {
    threshold: f64,
    limit: usize,
    asset_filter: Vec<String>,
    pub last_scanned_count: usize,
}

impl Scout {
    pub fn new(threshold: f64, limit: usize, asset_filter: Vec<String>) -> Self {
        Self { threshold, limit, asset_filter, last_scanned_count: 0 }
    }

    pub fn scan(&mut self) -> Result<Vec<SignalCard>> {
        let (meta, ctxs) = api::get_meta_and_asset_ctxs()?;
        let mids = api::get_all_mids()?;

        let universe = meta["universe"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing universe in meta"))?;

        self.last_scanned_count = universe.len();

        let mut candidates: Vec<(String, f64, f64)> = Vec::new();

        for (i, asset_meta) in universe.iter().enumerate() {
            let coin = match asset_meta["name"].as_str() {
                Some(c) => c.to_string(),
                None => continue,
            };

            if !self.asset_filter.is_empty()
                && !self.asset_filter.contains(&coin.to_uppercase())
            {
                continue;
            }

            let ctx = match ctxs.get(i) {
                Some(c) => c,
                None => continue,
            };

            let funding_rate: f64 = match ctx["funding"].as_str() {
                Some(f) => match f.parse() {
                    Ok(v) => v,
                    Err(_) => continue,
                },
                None => continue,
            };

            if funding_rate.abs() < self.threshold {
                continue;
            }

            let price = mids[&coin]
                .as_str()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);

            candidates.push((coin, funding_rate, price));
        }

        candidates.sort_by(|a, b| {
            b.1.abs().partial_cmp(&a.1.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(self.limit);

        let next_settlement = get_next_settlement();

        Ok(candidates
            .into_iter()
            .enumerate()
            .map(|(i, (coin, rate, price))| {
                build_signal_card(i + 1, coin, rate, price, &next_settlement)
            })
            .collect())
    }
}

fn build_signal_card(
    rank: usize,
    coin: String,
    rate: f64,
    price: f64,
    next_settlement: &str,
) -> SignalCard {
    let abs_rate = rate.abs();
    let direction = if rate > 0.0 { Direction::Short } else { Direction::Long };
    let conviction = classify_conviction(abs_rate);

    let hourly_rate_pct = format!(
        "{}{:.4}%",
        if rate >= 0.0 { "+" } else { "" },
        rate * 100.0
    );
    let apr = rate * 3.0 * 365.0 * 100.0;
    let apr_pct = format!("{}{:.1}%", if apr >= 0.0 { "+" } else { "" }, apr);

    let current_price = if price > 0.0 {
        format!("${:.4}", price)
    } else {
        "N/A".to_string()
    };

    let thesis = build_thesis(&coin, rate, abs_rate, &conviction);
    let supporting_factors = build_supporting_factors(&direction, &conviction);
    let risk_factors = build_risk_factors(&direction, &conviction);

    SignalCard {
        rank,
        asset: coin,
        direction,
        conviction,
        hourly_rate: rate,
        hourly_rate_pct,
        apr_pct,
        current_price: current_price.clone(),
        thesis,
        supporting_factors,
        risk_factors,
        entry_context: current_price,
        next_settlement: next_settlement.to_string(),
    }
}

fn get_next_settlement() -> String {
    let now = chrono::Utc::now();
    let current_minutes = now.hour() * 60 + now.minute();
    let settlement_minutes = [0u32, 480, 960];

    let next = settlement_minutes
        .iter()
        .find(|&&m| m > current_minutes)
        .copied()
        .unwrap_or(settlement_minutes[0] + 24 * 60);

    let diff = next - current_minutes;
    format!("in {}h {}m", diff / 60, diff % 60)
}