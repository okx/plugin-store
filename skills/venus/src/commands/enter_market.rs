// Venus — enter-market command (enable asset as collateral)

use crate::{config, onchainos};
use anyhow::Result;

pub async fn execute(
    chain_id: u64,
    asset: &str,
    dry_run: bool,
) -> Result<()> {
    config::get_rpc(chain_id)?;
    let (vtoken_addr, _, _, _) = config::resolve_asset(asset)?;

    // enterMarkets(address[]) selector: 0xc2998238
    // ABI-encode: offset (32), length (1), address
    let vtoken_clean = &vtoken_addr[2..];
    let calldata = format!(
        "0xc2998238\
         0000000000000000000000000000000000000000000000000000000000000020\
         0000000000000000000000000000000000000000000000000000000000000001\
         {:0>64}",
        vtoken_clean
    );

    if dry_run {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "action": "enter_market",
                "asset": asset,
                "vtoken": vtoken_addr,
                "comptroller": config::COMPTROLLER,
                "calldata": calldata
            })
        );
        return Ok(());
    }

    // Resolve wallet after dry_run guard
    let wallet = onchainos::resolve_wallet(chain_id)?;
    let _ = wallet;

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
            "vtoken": vtoken_addr,
            "tx_hash": tx_hash
        })
    );

    Ok(())
}
