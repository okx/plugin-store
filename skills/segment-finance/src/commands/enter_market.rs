// Segment Finance — enter-market command
// Enables an asset as collateral: Comptroller.enterMarkets([seToken])
// Selector: 0xc2998238

use crate::{config, onchainos};
use anyhow::Result;

pub async fn execute(
    chain_id: u64,
    asset: &str,
    dry_run: bool,
) -> Result<()> {
    config::get_rpc(chain_id)?;
    let (setoken_addr, _, _, _) = config::resolve_asset(asset)?;

    // enterMarkets(address[]) selector: 0xc2998238
    // ABI-encode: offset (32), length (1), address
    let setoken_clean = &setoken_addr[2..];
    let calldata = format!(
        "0xc2998238\
         0000000000000000000000000000000000000000000000000000000000000020\
         0000000000000000000000000000000000000000000000000000000000000001\
         {:0>64}",
        setoken_clean
    );

    if dry_run {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "action": "enter_market",
                "asset": asset,
                "setoken": setoken_addr,
                "comptroller": config::COMPTROLLER,
                "calldata": calldata
            })
        );
        return Ok(());
    }

    // Resolve wallet after dry_run guard
    let _wallet = onchainos::resolve_wallet(chain_id)?;

    // ask user to confirm before executing on-chain
    let result = onchainos::wallet_contract_call(
        chain_id,
        config::COMPTROLLER,
        &calldata,
        None,
        false,
    )
    .await?;
    let tx_hash = onchainos::extract_tx_hash(&result);

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "action": "enter_market",
            "asset": asset,
            "setoken": setoken_addr,
            "comptroller": config::COMPTROLLER,
            "tx_hash": tx_hash
        })
    );

    Ok(())
}
