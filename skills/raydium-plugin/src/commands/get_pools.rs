/// get-pools: Query Raydium pool info by pool IDs or by token mint addresses.
use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::DATA_API_BASE;

#[derive(Args, Debug)]
pub struct GetPoolsArgs {
    /// Comma-separated pool IDs (e.g. --ids "id1,id2")
    #[arg(long)]
    pub ids: Option<String>,

    /// First token mint address (required if not using --ids)
    #[arg(long)]
    pub mint1: Option<String>,

    /// Second token mint address (optional filter)
    #[arg(long)]
    pub mint2: Option<String>,

    /// Pool type filter: all, concentrated, standard, allFarm (default: all)
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

pub async fn execute(args: &GetPoolsArgs) -> Result<()> {
    let client = reqwest::Client::new();

    let resp: Value = if let Some(ref ids) = args.ids {
        // Query by pool IDs
        let url = format!("{}/pools/info/ids", DATA_API_BASE);
        client
            .get(&url)
            .query(&[("ids", ids.as_str())])
            .send()
            .await?
            .json()
            .await?
    } else if let Some(ref mint1) = args.mint1 {
        // Query by mint addresses
        let url = format!("{}/pools/info/mint", DATA_API_BASE);
        let mut query: Vec<(&str, String)> = vec![
            ("mint1", mint1.clone()),
            ("poolType", args.pool_type.clone()),
            ("poolSortField", args.sort_field.clone()),
            ("sortType", args.sort_type.clone()),
            ("pageSize", args.page_size.to_string()),
            ("page", args.page.to_string()),
        ];
        if let Some(ref mint2) = args.mint2 {
            query.push(("mint2", mint2.clone()));
        }
        client
            .get(&url)
            .query(&query)
            .send()
            .await?
            .json()
            .await?
    } else {
        anyhow::bail!("Either --ids or --mint1 must be provided");
    };

    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
