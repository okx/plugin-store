use clap::Args;
use crate::api::MeteoraClient;
use crate::config::APY_RISK_WARN_THRESHOLD;

#[derive(Args, Debug)]
pub struct GetPoolsArgs {
    /// Page number (default: 1)
    #[arg(long, default_value = "1")]
    pub page: u32,

    /// Results per page (default: 10, max: 100)
    #[arg(long, default_value = "10")]
    pub page_size: u32,

    /// Sort by: tvl, volume, fee_tvl_ratio, apr
    #[arg(long, default_value = "tvl")]
    pub sort_key: String,

    /// Sort order: asc or desc
    #[arg(long, default_value = "desc")]
    pub order_by: String,

    /// Search term: token symbol or pool address
    #[arg(long)]
    pub search_term: Option<String>,
}

pub async fn execute(args: &GetPoolsArgs) -> anyhow::Result<()> {
    let client = MeteoraClient::new();
    let resp = client
        .get_pools(
            Some(args.page),
            Some(args.page_size),
            Some(&args.sort_key),
            Some(&args.order_by),
            args.search_term.as_deref(),
        )
        .await?;

    let pools: Vec<serde_json::Value> = resp
        .data
        .iter()
        .map(|p| {
            let apy_warn = p.apy > APY_RISK_WARN_THRESHOLD;
            serde_json::json!({
                "address": p.address,
                "name": p.name,
                "token_x": {
                    "address": p.token_x.address,
                    "symbol": p.token_x.symbol,
                    "decimals": p.token_x.decimals,
                },
                "token_y": {
                    "address": p.token_y.address,
                    "symbol": p.token_y.symbol,
                    "decimals": p.token_y.decimals,
                },
                "tvl_usd": p.tvl,
                "current_price": p.current_price,
                "bin_step": p.pool_config.bin_step,
                "base_fee_pct": p.pool_config.base_fee_pct,
                "apr": p.apr,
                "apy": p.apy,
                "apy_risk_warning": if apy_warn { Some("High APY may indicate elevated impermanent loss risk") } else { None },
                "has_farm": p.has_farm,
                "volume_24h": p.volume.as_ref().map(|v| v.h24).unwrap_or(0.0),
                "fees_24h": p.fees.as_ref().map(|f| f.h24).unwrap_or(0.0),
            })
        })
        .collect();

    let output = serde_json::json!({
        "ok": true,
        "total": resp.total,
        "pages": resp.pages,
        "current_page": resp.current_page,
        "page_size": resp.page_size,
        "pools": pools,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
