use crate::abi;
use crate::config::get_chain_config;
use crate::onchainos;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    token_out: &str,
    glp_amount: u128,
    min_out: u128,
    dry_run: bool,
) -> Result<()> {
    let cfg = get_chain_config(chain_id)?;

    let wallet = if dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    let calldata =
        abi::encode_unstake_and_redeem_glp(token_out, glp_amount, min_out, &wallet)?;

    println!("Sell GLP: {} GLP tokens, receive {}", glp_amount, token_out);
    println!("Min output: {}", min_out);
    println!("RewardRouter: {}", cfg.reward_router);
    println!("Calldata: {}", calldata);

    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.reward_router,
        &calldata,
        None,
        dry_run,
    )
    .await?;

    if dry_run {
        println!("Dry run result: {}", serde_json::to_string_pretty(&result)?);
    } else {
        let tx_hash = onchainos::extract_tx_hash(&result);
        println!("Sell GLP submitted. TxHash: {}", tx_hash);
        println!("Full result: {}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}
