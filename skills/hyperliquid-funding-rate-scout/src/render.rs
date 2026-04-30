use crate::signal::{Direction, SignalCard};
use chrono::Timelike;

const DIVIDER: &str = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";

pub fn render_signal_cards(signals: &[SignalCard], total_scanned: usize, threshold: f64) {
    let threshold_pct = threshold * 100.0;
    let now = chrono::Utc::now();

    println!("\n📡 Hyperliquid Funding Rate Scout");
    println!("⏰ Scan time: {} UTC", now.format("%Y-%m-%d %H:%M:%S"));
    println!(
        "📊 Markets scanned: {} | Signals found: {} | Threshold: >{:.4}% hourly\n",
        total_scanned,
        signals.len(),
        threshold_pct
    );

    // Settlement urgency warning
    if let Some(warning) = get_settlement_warning() {
        println!("{}\n", warning);
    }

    for signal in signals {
        render_card(signal);
    }

    println!("\n{}", DIVIDER);
    println!("✋  STOP: DO NOT EXECUTE WITHOUT USER CONFIRMATION");
    println!("{}", DIVIDER);
    println!("\nWould you like to act on any of these signals?");
    println!("If yes, provide: asset, position size (USDC), leverage, and stop-loss price.\n");
}

fn render_card(signal: &SignalCard) {
    let dir_emoji = match signal.direction {
        Direction::Short => "📉",
        Direction::Long => "📈",
    };

    let conviction_emoji = signal.conviction.emoji();

    println!("\n{}", DIVIDER);
    println!("{} SIGNAL #{} — {}/USD-PERP", dir_emoji, signal.rank, signal.asset);
    println!("{}", DIVIDER);
    println!("Direction:       {}", signal.direction);
    println!("Signal Type:     Mean Reversion — Funding Rate Extreme");
    println!("Conviction:      {} {}", conviction_emoji, signal.conviction);
    println!();
    println!(
        "Funding Rate:    {} hourly  ({} APR)",
        signal.hourly_rate_pct, signal.apr_pct
    );
    println!("Current Price:   {}", signal.current_price);
    println!("Next Settlement: {}", signal.next_settlement);
    println!();
    println!("📌 Thesis");
    println!("{}", wrap_text(&signal.thesis, 70));
    println!();
    println!("✅ Supporting Factors");
    for f in &signal.supporting_factors {
        println!("  • {}", f);
    }
    println!();
    println!("⚠️  Risk Factors");
    for r in &signal.risk_factors {
        println!("  • {}", r);
    }
    println!();
    println!("Entry Context:   ~{} (current market)", signal.current_price);
}

pub fn render_no_signals(threshold: f64, total_scanned: usize) {
    let threshold_pct = threshold * 100.0;
    println!("\n📡 Hyperliquid Funding Rate Scout");
    println!("\nNo statistically significant funding rate imbalances detected.");
    println!();
    println!("Markets scanned:  {}", total_scanned);
    println!("Signals found:    0");
    println!("Threshold used:   >{:.4}% hourly", threshold_pct);
    println!();
    println!("Markets appear to be near equilibrium.");
    println!("Funding rates spike most around settlement windows: 00:00, 08:00, 16:00 UTC.");
    println!("Consider checking back then, or lower --threshold to see moderate signals.\n");
}

fn get_settlement_warning() -> Option<String> {
    let now = chrono::Utc::now();
    let current_minutes = now.hour() * 60 + now.minute();
    let settlement_minutes = [0u32, 480, 960];

    let next = settlement_minutes
        .iter()
        .find(|&&m| m > current_minutes)
        .copied()
        .unwrap_or(settlement_minutes[0] + 24 * 60);

    let diff = next - current_minutes;

    if diff <= 30 {
        Some(format!(
            "🔴 Settlement in {} minutes — funding rates at peak. Act now or wait for next window.",
            diff
        ))
    } else if diff <= 90 {
        Some(format!(
            "⏰ Settlement in ~{} minutes — high-conviction scan window approaching.",
            diff
        ))
    } else {
        None
    }
}

fn wrap_text(text: &str, width: usize) -> String {
    let mut lines = Vec::new();
    let mut line = String::new();

    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            lines.push(line.clone());
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }

    lines.join("\n")
}