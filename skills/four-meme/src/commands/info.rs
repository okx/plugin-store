use clap::Args;
use serde_json::Value;

use crate::config::{API_TOKEN_GET, BSC_RPC, TOKEN_MANAGER_HELPER_V3, ZERO_ADDRESS};
use crate::calldata::{build_get_token_info_calldata, format_wei_as_bnb, format_token_amount};
use crate::rpc::{eth_call, decode_uint256, decode_address, decode_bool};

#[derive(Args)]
pub struct InfoArgs {
    /// Token contract address on BSC
    #[arg(long)]
    pub token: String,
}

pub async fn run(args: &InfoArgs) -> anyhow::Result<()> {
    let token = &args.token;
    let client = reqwest::Client::new();

    // 1. Fetch off-chain metadata from four.meme API
    let api_url = format!("{}?address={}", API_TOKEN_GET, token);
    let api_resp: Value = client
        .get(&api_url)
        .header("User-Agent", "four-meme-plugin/0.1.0")
        .send()
        .await?
        .json()
        .await?;

    // 2. Fetch on-chain info from helper contract
    let calldata = build_get_token_info_calldata(token);
    let onchain_raw = eth_call(TOKEN_MANAGER_HELPER_V3, &calldata, BSC_RPC).await
        .unwrap_or_else(|_| "0x".to_string());

    println!("Four.meme Token Info");
    println!("{}", "=".repeat(60));
    println!("Token Address: {}", token);

    // Parse API response
    let api_code = api_resp["code"].as_i64().unwrap_or(-1);
    if api_code == 0 {
        let d = &api_resp["data"];
        let name = d["name"].as_str().unwrap_or("?");
        let symbol_short = d["shortName"].as_str().unwrap_or("?");
        let base_symbol = d["symbol"].as_str().unwrap_or("BNB");
        let desc = d["descr"].as_str().unwrap_or("");
        let status = d["status"].as_str().unwrap_or("?");
        let version = d["version"].as_str().unwrap_or("?");

        println!("\nName:         {} ({})", name, symbol_short);
        println!("Base Token:   {}", base_symbol);
        println!("Status:       {}", status);
        println!("Version:      {}", version);

        if !desc.is_empty() {
            println!("Description:  {}", desc);
        }

        if let Some(web) = d["webUrl"].as_str() {
            if !web.is_empty() { println!("Website:      {}", web); }
        }
        if let Some(twitter) = d["twitterUrl"].as_str() {
            if !twitter.is_empty() { println!("Twitter:      {}", twitter); }
        }
        if let Some(tg) = d["telegramUrl"].as_str() {
            if !tg.is_empty() { println!("Telegram:     {}", tg); }
        }

        // Price info
        if let Some(price_info) = d["tokenPrice"].as_object() {
            println!("\n--- Price & Market ---");
            if let Some(price) = price_info.get("price").and_then(|v| v.as_str()) {
                println!("Price:        {} BNB", price);
            }
            if let Some(mcap) = price_info.get("marketCap").and_then(|v| v.as_str()) {
                println!("Market Cap:   {} BNB", mcap);
            }
            if let Some(trading) = price_info.get("trading").and_then(|v| v.as_str()) {
                println!("Total Volume: {} BNB", trading);
            }
            if let Some(bnb_raised) = price_info.get("bnbAmount").and_then(|v| v.as_str()) {
                println!("BNB Raised:   {} BNB", bnb_raised);
            }
            if let Some(progress) = price_info.get("progress").and_then(|v| v.as_str()) {
                // Display progress as percentage (ratio from API)
                let pct_str = if let Ok(p) = progress.parse::<f64>() {
                    format!("{:.1}%", p * 100.0)
                } else {
                    progress.to_string()
                };
                println!("BC Progress:  {}", pct_str);
            }
            if let Some(day_inc) = price_info.get("dayIncrease").and_then(|v| v.as_str()) {
                println!("24h Change:   {}%", day_inc);
            }
        }
    } else {
        println!("\nAPI: Token not found or unavailable (code {})", api_code);
    }

    // On-chain data from helper
    if onchain_raw.len() > 10 {
        println!("\n--- On-Chain Data ---");
        // getTokenInfo returns 12 slots:
        // 0: version, 1: tokenManager, 2: quote, 3: lastPrice
        // 4: tradingFeeRate, 5: minTradingFee, 6: launchTime
        // 7: offers, 8: maxOffers, 9: funds, 10: maxFunds, 11: liquidityAdded
        let version = decode_uint256(&onchain_raw, 0);
        let token_manager = decode_address(&onchain_raw, 1);
        let quote = decode_address(&onchain_raw, 2);
        let last_price = decode_uint256(&onchain_raw, 3);
        let fee_rate = decode_uint256(&onchain_raw, 4);
        let offers = decode_uint256(&onchain_raw, 7);
        let max_offers = decode_uint256(&onchain_raw, 8);
        let funds = decode_uint256(&onchain_raw, 9);
        let max_funds = decode_uint256(&onchain_raw, 10);
        let liquidity_added = decode_bool(&onchain_raw, 11);

        if version > 0 {
            println!("TM Version:   V{}", version);
            println!("TM Address:   {}", token_manager);
            let quote_display = if quote == ZERO_ADDRESS {
                "BNB (native)".to_string()
            } else {
                format!("ERC20 ({})", quote)
            };
            println!("Quote:        {}", quote_display);

            if last_price > 0 {
                println!("Last Price:   {} wei", last_price);
            }
            // fee_rate is in bps/100 (100 = 1%, so divide by 10000 for percentage)
            let fee_num = fee_rate * 100 / 10000;
            let fee_frac = fee_rate * 100 % 10000 / 100;
            println!("Fee Rate:     {}.{:02}%", fee_num, fee_frac);

            if max_offers > 0 {
                // offers/maxOffers are raw token amounts (with 18 decimals embedded)
                let sold = max_offers.saturating_sub(offers);
                // Calculate percentage without float: sold * 1000 / max (gives 0-1000)
                let pct_x10 = {
                    // Scale down to avoid u128 overflow
                    let sold_k = sold / 1_000_000_000_000_000u128;
                    let max_k = max_offers / 1_000_000_000_000_000u128;
                    if max_k > 0 { sold_k * 1000 / max_k } else { 0 }
                };
                println!("Tokens Sold: {} / {} ({}.{}%)",
                    format_token_amount(sold),
                    format_token_amount(max_offers),
                    pct_x10 / 10,
                    pct_x10 % 10,
                );
            }

            if max_funds > 0 {
                println!("BNB Raised:   {} / {} BNB",
                    format_wei_as_bnb(funds),
                    format_wei_as_bnb(max_funds)
                );
            }

            if liquidity_added {
                println!("Status:       Graduated (PancakeSwap pair created)");
            } else {
                println!("Status:       On bonding curve");
            }
        }
    }

    println!("\nSource: {}", api_url);
    Ok(())
}
