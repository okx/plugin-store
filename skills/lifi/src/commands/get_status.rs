use anyhow::Result;
use serde_json::Value;

use crate::api;

pub async fn execute(
    tx_hash: &str,
    from_chain: Option<u64>,
    to_chain: Option<u64>,
    bridge: Option<&str>,
) -> Result<Value> {
    let resp = api::get_status(tx_hash, from_chain, to_chain, bridge).await?;

    Ok(serde_json::json!({
        "ok": true,
        "status": resp["status"],
        "substatus": resp["substatus"],
        "substatusMessage": resp["substatusMessage"],
        "sending": {
            "txHash": resp["sending"]["txHash"],
            "chainId": resp["sending"]["chainId"],
            "amount": resp["sending"]["amount"]
        },
        "receiving": {
            "txHash": resp["receiving"]["txHash"],
            "chainId": resp["receiving"]["chainId"],
            "amount": resp["receiving"]["amount"]
        },
        "tool": resp["tool"],
        "lifiExplorer": format!("https://scan.li.fi/tx/{}", tx_hash)
    }))
}
