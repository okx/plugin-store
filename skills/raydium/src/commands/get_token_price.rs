use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::DATA_API_BASE;

#[derive(Args, Debug)]
pub struct GetTokenPriceArgs {
    /// Comma-separated list of token mint addresses
    #[arg(long)]
    pub mints: String,
}

pub async fn execute(args: &GetTokenPriceArgs) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/mint/price", DATA_API_BASE);
    let resp: Value = client
        .get(&url)
        .query(&[("mints", &args.mints)])
        .send()
        .await?
        .json()
        .await?;

    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
