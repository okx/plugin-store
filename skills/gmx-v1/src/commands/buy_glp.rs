use crate::abi;
use crate::config::get_chain_config;
use crate::onchainos;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    token: &str,
    amount: u128,
    min_usdg: u128,
    min_glp: u128,
    dry_run: bool,
) -> Result<()> {
    let cfg = get_chain_config(chain_id)?;

    // First: approve token to GlpManager if needed (user should confirm)
    // The buy-glp command encodes the mintAndStakeGlp call.
    // Note: GlpManager must be approved as spender for the input token.
    let calldata = abi::encode_mint_and_stake_glp(token, amount, min_usdg, min_glp)?;

    println!("Buy GLP: {} units of token {}", amount, token);
    println!("Min USDG: {}, Min GLP: {}", min_usdg, min_glp);
    println!("RewardRouter: {}", cfg.reward_router);
    println!("Calldata: {}", calldata);
    println!(
        "Note: Ensure token is approved to GlpManager ({}) before executing.",
        cfg.glp_manager
    );

    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.reward_router,
        &calldata,
        None, // no ETH value for token-based GLP buy
        dry_run,
    )
    .await?;

    if dry_run {
        println!("Dry run result: {}", serde_json::to_string_pretty(&result)?);
    } else {
        let tx_hash = onchainos::extract_tx_hash(&result);
        println!("Buy GLP submitted. TxHash: {}", tx_hash);
        println!("Full result: {}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}
