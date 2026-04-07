use crate::config::get_chain_config;
use crate::onchainos;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    token: &str,
    spender: &str,
    dry_run: bool,
) -> Result<()> {
    let _cfg = get_chain_config(chain_id)?;

    // approve(address,uint256) — max uint256 for unlimited approval
    let calldata = onchainos::encode_approve(spender, u128::MAX)?;

    println!("Approve {} to spend {} (unlimited)", spender, token);
    println!("Token contract: {}", token);
    println!("Calldata: {}", calldata);

    let result = onchainos::wallet_contract_call(
        chain_id,
        token,
        &calldata,
        None,
        dry_run,
    )
    .await?;

    if dry_run {
        println!("Dry run result: {}", serde_json::to_string_pretty(&result)?);
    } else {
        let tx_hash = onchainos::extract_tx_hash(&result);
        println!("Approve submitted. TxHash: {}", tx_hash);
        println!("Full result: {}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}
