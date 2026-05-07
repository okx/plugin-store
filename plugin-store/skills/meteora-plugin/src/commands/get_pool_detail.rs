use clap::Args;
use crate::api::MeteoraClient;
use crate::config::APY_RISK_WARN_THRESHOLD;

#[derive(Args, Debug)]
pub struct GetPoolDetailArgs {
    /// Pool address (Solana pubkey)
    #[arg(long)]
    pub address: String,
}

pub async fn execute(args: &GetPoolDetailArgs) -> anyhow::Result<()> {
    let client = MeteoraClient::new();
    let pool = client.get_pool_detail(&args.address).await?;

    let apy_warn = pool.apy > APY_RISK_WARN_THRESHOLD;

    let output = serde_json::json!({
        "ok": true,
        "pool": {
            "address": pool.address,
            "name": pool.name,
            "token_x": {
                "address": pool.token_x.address,
                "symbol": pool.token_x.symbol,
                "name": pool.token_x.name,
                "decimals": pool.token_x.decimals,
                "price_usd": pool.token_x.price,
            },
            "token_y": {
                "address": pool.token_y.address,
                "symbol": pool.token_y.symbol,
                "name": pool.token_y.name,
                "decimals": pool.token_y.decimals,
                "price_usd": pool.token_y.price,
            },
            "reserves": {
                "token_x_amount": pool.token_x_amount,
                "token_y_amount": pool.token_y_amount,
                "reserve_x_account": pool.reserve_x,
                "reserve_y_account": pool.reserve_y,
            },
            "pool_config": {
                "bin_step": pool.pool_config.bin_step,
                "base_fee_pct": pool.pool_config.base_fee_pct,
                "max_fee_pct": pool.pool_config.max_fee_pct,
                "protocol_fee_pct": pool.pool_config.protocol_fee_pct,
            },
            "dynamic_fee_pct": pool.dynamic_fee_pct,
            "tvl_usd": pool.tvl,
            "current_price": pool.current_price,
            "apr": pool.apr,
            "apy": pool.apy,
            "apy_risk_warning": if apy_warn { Some("High APY may indicate elevated impermanent loss risk") } else { None },
            "has_farm": pool.has_farm,
            "farm_apr": pool.farm_apr,
            "farm_apy": pool.farm_apy,
            "volume": pool.volume,
            "fees": pool.fees,
            "cumulative_metrics": pool.cumulative_metrics,
            "is_blacklisted": pool.is_blacklisted,
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
