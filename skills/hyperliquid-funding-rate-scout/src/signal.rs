use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalCard {
    pub rank: usize,
    pub asset: String,
    pub direction: Direction,
    pub conviction: Conviction,
    pub hourly_rate: f64,
    pub hourly_rate_pct: String,
    pub apr_pct: String,
    pub current_price: String,
    pub thesis: String,
    pub supporting_factors: Vec<String>,
    pub risk_factors: Vec<String>,
    pub entry_context: String,
    pub next_settlement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Direction {
    Long,
    Short,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Long => write!(f, "LONG"),
            Direction::Short => write!(f, "SHORT"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Conviction {
    Elevated,
    High,
    Extreme,
}

impl std::fmt::Display for Conviction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Conviction::Elevated => write!(f, "Elevated"),
            Conviction::High => write!(f, "High"),
            Conviction::Extreme => write!(f, "Extreme"),
        }
    }
}

impl Conviction {
    pub fn emoji(&self) -> &str {
        match self {
            Conviction::Elevated => "🟡",
            Conviction::High => "🟠",
            Conviction::Extreme => "🔴",
        }
    }
}

/// Classify conviction from hourly rate (decimal, not percent).
/// Thresholds per SKILL.md:
///   > 0.0005 = Extreme (0.05%)
///   > 0.0003 = High    (0.03%)
///   else     = Elevated (> 0.01%)
pub fn classify_conviction(abs_rate: f64) -> Conviction {
    if abs_rate > 0.0005 {
        Conviction::Extreme
    } else if abs_rate > 0.0003 {
        Conviction::High
    } else {
        Conviction::Elevated
    }
}

/// Build the mean reversion thesis string.
pub fn build_thesis(coin: &str, rate: f64, abs_rate: f64, conviction: &Conviction) -> String {
    let direction_str = if rate > 0.0 {
        "longs are paying shorts"
    } else {
        "shorts are paying longs"
    };
    let side = if rate > 0.0 { "short" } else { "long" };
    let squeeze = if rate > 0.0 {
        "longs may be forced to close, accelerating a downside move"
    } else {
        "shorts may be forced to close, accelerating an upside move"
    };
    let apr = abs_rate * 3.0 * 365.0 * 100.0;

    let conviction_note = match conviction {
        Conviction::Extreme => " This is among the most extreme funding conditions on Hyperliquid.",
        Conviction::High => " This level of crowding is historically significant.",
        Conviction::Elevated => " This level of crowding historically precedes mean reversion.",
    };

    format!(
        "{coin} perp funding is at {rate:.4}% per hour ({apr:.1}% APR), meaning \
        {direction_str} at an unsustainable rate.{conviction_note} \
        A {side} position here collects funding while waiting for the imbalance to unwind — \
        if momentum reverses, {squeeze}.",
        coin = coin,
        rate = abs_rate * 100.0,
        apr = apr,
        direction_str = direction_str,
        conviction_note = conviction_note,
        side = side,
        squeeze = squeeze,
    )
}

/// Build supporting factors list.
pub fn build_supporting_factors(direction: &Direction, conviction: &Conviction) -> Vec<String> {
    let mut factors = vec![
        format!(
            "Funding rate deviation classified as {} — statistically significant outlier",
            conviction
        ),
        "Mean reversion thesis: overcrowded side typically unwinds toward equilibrium".to_string(),
    ];

    if *conviction == Conviction::Extreme {
        factors.push(
            "Rate > 0.05% hourly — historically among the top 5% of extreme readings".to_string(),
        );
    }

    match direction {
        Direction::Short => {
            factors.push(
                "Longs paying premium incentivizes new shorts to enter and collect".to_string(),
            );
        }
        Direction::Long => {
            factors.push(
                "Shorts paying premium incentivizes new longs to enter and collect".to_string(),
            );
        }
    }

    factors
}

/// Build risk factors list.
pub fn build_risk_factors(direction: &Direction, conviction: &Conviction) -> Vec<String> {
    let mut risks = vec![
        "Strong price momentum can delay or override funding normalization".to_string(),
        "High open interest increases risk of volatile liquidation cascade".to_string(),
    ];

    match direction {
        Direction::Short => {
            risks.push(
                "If price continues rising, short position faces mark-to-market loss \
                despite collecting funding"
                    .to_string(),
            );
        }
        Direction::Long => {
            risks.push(
                "If price continues falling, long position faces mark-to-market loss \
                despite collecting funding"
                    .to_string(),
            );
        }
    }

    match conviction {
        Conviction::Extreme => {
            risks.push(
                "Extreme rates may persist longer than expected in trending markets".to_string(),
            );
        }
        _ => {
            risks.push(
                "Funding may normalize before sufficient premium is collected".to_string(),
            );
        }
    }

    risks
}