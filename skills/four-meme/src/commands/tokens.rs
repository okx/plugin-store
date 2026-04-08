use clap::Args;
use serde_json::Value;

use crate::config::API_CONFIG;

#[derive(Args)]
pub struct TokensArgs {
    /// Show supported base tokens (BNB, CAKE, etc.) from the Four.meme platform config
    #[arg(long)]
    pub base_tokens: bool,
}

pub async fn run(args: &TokensArgs) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    // Fetch platform config (supported base tokens)
    let resp: Value = client
        .get(API_CONFIG)
        .header("User-Agent", "four-meme-plugin/0.1.0")
        .send()
        .await?
        .json()
        .await?;

    let code = resp["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        anyhow::bail!("API error: {}", resp);
    }

    let tokens = resp["data"].as_array().cloned().unwrap_or_default();

    println!("Four.meme Supported Base Tokens (BSC)");
    println!("{}", "=".repeat(60));

    let published: Vec<&Value> = tokens.iter()
        .filter(|t| t["status"].as_str().unwrap_or("") == "PUBLISH")
        .collect();

    if published.is_empty() {
        println!("No tokens found.");
        return Ok(());
    }

    for token in &published {
        let symbol = token["symbol"].as_str().unwrap_or("?");
        let address = token["symbolAddress"].as_str().unwrap_or("?");
        let total_b = token["totalBAmount"].as_str().unwrap_or("?");
        let buy_fee = token["buyFee"].as_str().unwrap_or("?");
        let levels = token["tradeLevel"].as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
            .unwrap_or_default();

        println!("\n  Symbol:       {}", symbol);
        println!("  Address:      {}", address);
        println!("  Graduation:   {} {} to raise", total_b, symbol);
        println!("  Trading Fee:  {}%", buy_fee);
        if !levels.is_empty() {
            println!("  Trade Levels: {} {}", levels, symbol);
        }
    }

    if !args.base_tokens {
        println!("\n{}", "-".repeat(60));
        println!("Use `four-meme info --token <ADDRESS>` to get details for a specific token.");
        println!("Source: {}", API_CONFIG);
    }

    Ok(())
}
