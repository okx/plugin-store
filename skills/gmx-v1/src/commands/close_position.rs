use crate::abi;
use crate::config::{get_chain_config, EXECUTION_FEE_WEI};
use crate::onchainos;
use anyhow::Result;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    chain_id: u64,
    collateral_token: &str,
    index_token: &str,
    collateral_delta: u128,
    size_delta_usd: f64,
    is_long: bool,
    acceptable_price: u128,
    min_out: u128,
    execution_fee: Option<u64>,
    withdraw_eth: bool,
    dry_run: bool,
) -> Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let exec_fee = execution_fee.unwrap_or(EXECUTION_FEE_WEI);

    // sizeDelta in GMX V1 = USD * 10^30
    let size_delta: u128 = (size_delta_usd * 1e30) as u128;

    let wallet = if dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    // path: [indexToken, collateralToken] for receiving collateral token
    let path = [index_token, collateral_token];
    let calldata = abi::encode_create_decrease_position(
        &path,
        index_token,
        collateral_delta,
        size_delta,
        is_long,
        &wallet,
        acceptable_price,
        min_out,
        exec_fee,
        withdraw_eth,
    )?;

    println!(
        "Close {} position: size ${:.2} on {}",
        if is_long { "LONG" } else { "SHORT" },
        size_delta_usd,
        index_token
    );
    println!("Execution fee: {} wei (0.0001 ETH)", exec_fee);
    println!("PositionRouter: {}", cfg.position_router);
    println!("Calldata: {}", calldata);
    println!(
        "\nIMPORTANT: This operation requires {} wei ETH as execution fee.",
        exec_fee
    );

    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.position_router,
        &calldata,
        Some(exec_fee),
        dry_run,
    )
    .await?;

    if dry_run {
        println!("Dry run result: {}", serde_json::to_string_pretty(&result)?);
    } else {
        let tx_hash = onchainos::extract_tx_hash(&result);
        println!("Close position submitted. TxHash: {}", tx_hash);
        println!("Full result: {}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}
