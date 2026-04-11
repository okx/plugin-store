/// get-pool-list: Paginated list of Raydium pools with sorting options.
use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::DATA_API_BASE;

#[derive(Args, Debug)]
pub struct GetPoolListArgs {
    /// Pool type: all, concentrated, standard, allFarm (default: all)
    #[arg(long, default_value = "all")]
    pub pool_type: String,

    /// Sort field: default, liquidity, volume24h, apr24h (default: liquidity)
    #[arg(long, default_value = "liquidity")]
    pub sort_field: String,

    /// Sort direction: desc or asc (default: desc)
    #[arg(long, default_value = "desc")]
    pub sort_type: String,

    /// Page size (default: 10, max: 1000)
    #[arg(long, default_value_t = 10)]
    pub page_size: u32,

    /// Page number, 1-based (default: 1)
    #[arg(long, default_value_t = 1)]
    pub page: u32,
}

pub async fn execute(args: &GetPoolListArgs) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/pools/info/list", DATA_API_BASE);
    let resp: Value = client
        .get(&url)
        .query(&[
            ("poolType", args.pool_type.as_str()),
            ("poolSortField", args.sort_field.as_str()),
            ("sortType", args.sort_type.as_str()),
            ("pageSize", &args.page_size.to_string()),
            ("page", &args.page.to_string()),
        ])
        .send()
        .await?
        .json()
        .await?;

    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
