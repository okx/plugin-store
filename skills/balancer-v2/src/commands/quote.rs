/// quote command — get swap quote via BalancerQueries.querySwap

use anyhow::Result;
use serde::Serialize;

use crate::config;
use crate::rpc;

#[derive(Debug, Serialize)]
struct QuoteResult {
    pool_id: String,
    asset_in: String,
    asset_out: String,
    amount_in: String,
    amount_out: String,
    amount_out_human: String,
    chain_id: u64,
}

pub async fn run(
    from_token: &str,
    to_token: &str,
    amount: f64,
    pool_id: &str,
    chain_id: u64,
) -> Result<()> {
    let rpc_url = config::rpc_url(chain_id);
    let queries_contract = config::BALANCER_QUERIES_ADDRESS;

    let asset_in = config::resolve_token_address(from_token, chain_id);
    let asset_out = config::resolve_token_address(to_token, chain_id);

    // Get decimals for tokenIn
    let decimals_in = rpc::get_decimals(&asset_in, rpc_url).await.unwrap_or(18);
    let decimals_out = rpc::get_decimals(&asset_out, rpc_url).await.unwrap_or(18);

    let amount_in = (amount * 10f64.powi(decimals_in as i32)) as u128;

    // querySwap (GIVEN_IN)
    let amount_out = rpc::query_swap(
        queries_contract,
        pool_id,
        &asset_in,
        &asset_out,
        amount_in,
        rpc_url,
    )
    .await?;

    let amount_out_human = format!("{:.6}", amount_out as f64 / 10f64.powi(decimals_out as i32));

    let result = QuoteResult {
        pool_id: pool_id.to_string(),
        asset_in,
        asset_out,
        amount_in: amount_in.to_string(),
        amount_out: amount_out.to_string(),
        amount_out_human,
        chain_id,
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
