use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::{DATA_API_BASE, SOL_NATIVE_MINT, SOL_SYSTEM_PROGRAM};

#[derive(Args, Debug)]
pub struct GetTokenPriceArgs {
    /// Comma-separated list of token mint addresses
    #[arg(long)]
    pub mints: String,
}

pub async fn execute(args: &GetTokenPriceArgs) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/mint/price", DATA_API_BASE);

    // Rewrite native SOL system program address to WSOL in the mints list
    let mints: String = args
        .mints
        .split(',')
        .map(|m| {
            let m = m.trim();
            if m == SOL_SYSTEM_PROGRAM { SOL_NATIVE_MINT } else { m }
        })
        .collect::<Vec<_>>()
        .join(",");

    let resp: Value = client
        .get(&url)
        .query(&[("mints", &mints)])
        .send()
        .await?
        .json()
        .await?;

    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
