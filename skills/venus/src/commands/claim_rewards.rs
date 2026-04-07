// Venus — claim-rewards command (claim XVS)

use crate::{config, onchainos};
use anyhow::Result;

pub async fn execute(
    chain_id: u64,
    wallet: Option<String>,
    dry_run: bool,
) -> Result<()> {
    config::get_rpc(chain_id)?;

    // claimVenus(address) selector: 0xadcd5fb9
    // We need the holder address — resolve after dry_run guard
    if dry_run {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "action": "claim_rewards",
                "token": "XVS",
                "comptroller": config::COMPTROLLER,
                "note": "Claim XVS rewards from Venus Comptroller"
            })
        );
        return Ok(());
    }

    // Resolve wallet after dry_run guard
    let holder = match wallet {
        Some(w) => w,
        None => onchainos::resolve_wallet(chain_id)?,
    };

    let holder_clean = &holder[2..];
    let calldata = format!(
        "0xadcd5fb9{:0>64}",
        holder_clean
    );

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
            "action": "claim_rewards",
            "token": "XVS",
            "holder": holder,
            "tx_hash": tx_hash
        })
    );

    Ok(())
}
