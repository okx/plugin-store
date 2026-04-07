use crate::abi;
use crate::config::get_chain_config;
use crate::onchainos;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    input_token: &str,
    input_amount: u128,
    output_token: &str,
    min_output: u128,
    dry_run: bool,
) -> Result<()> {
    let cfg = get_chain_config(chain_id)?;

    let wallet = if dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    // Build path: [inputToken, outputToken]
    let path = [input_token, output_token];
    let calldata = abi::encode_swap(&path, input_amount, min_output, &wallet)?;

    println!("Swap {} units of {} to {}", input_amount, input_token, output_token);
    println!("Router: {}", cfg.router);
    println!("Calldata: {}", calldata);

    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.router,
        &calldata,
        None, // no ETH value for token-to-token swap
        dry_run,
    )
    .await?;

    if dry_run {
        println!("Dry run result: {}", serde_json::to_string_pretty(&result)?);
    } else {
        let tx_hash = onchainos::extract_tx_hash(&result);
        println!("Swap submitted. TxHash: {}", tx_hash);
        println!("Full result: {}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}
