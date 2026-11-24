use clap::Args;
use crate::strategies::STRATEGIES;

#[derive(Args)]
pub struct StrategiesArgs {}

pub async fn run(_args: StrategiesArgs) -> anyhow::Result<()> {
    let out: Vec<serde_json::Value> = STRATEGIES.iter().map(|s| serde_json::json!({
        "symbol":      s.symbol,
        "description": s.description,
        "token":       s.token,
        "strategy":    s.strategy,
        "decimals":    s.decimals,
    })).collect();
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
