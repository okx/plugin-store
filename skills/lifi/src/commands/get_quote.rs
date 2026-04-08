use anyhow::Result;
use serde_json::Value;

use crate::api;
use crate::onchainos;

pub async fn execute(
    from_chain: u64,
    to_chain: u64,
    from_token: &str,
    to_token: &str,
    amount: &str,
    slippage: f64,
    from: Option<&str>,
) -> Result<Value> {
    // Resolve wallet address — needed for the quote (transactionRequest.from)
    let wallet = if let Some(f) = from {
        f.to_string()
    } else {
        onchainos::resolve_wallet(from_chain)?
    };

    let resp = api::get_quote(from_chain, to_chain, from_token, to_token, amount, &wallet, slippage).await?;

    // Extract key fields for user-friendly display
    let estimate = &resp["estimate"];
    let action = &resp["action"];
    let tx_req = &resp["transactionRequest"];

    Ok(serde_json::json!({
        "ok": true,
        "from": {
            "chain": from_chain,
            "token": action["fromToken"]["symbol"],
            "amount": estimate["fromAmount"],
            "amountUSD": estimate["fromAmountUSD"]
        },
        "to": {
            "chain": to_chain,
            "token": action["toToken"]["symbol"],
            "amount": estimate["toAmount"],
            "amountUSD": estimate["toAmountUSD"]
        },
        "tool": resp["toolDetails"]["name"],
        "toolKey": resp["toolDetails"]["key"],
        "type": resp["type"],
        "feeCosts": estimate["feeCosts"],
        "gasCosts": estimate["gasCosts"],
        "executionDuration": estimate["executionDuration"],
        "transactionRequest": {
            "to": tx_req["to"],
            "data": tx_req["data"],
            "value": tx_req["value"],
            "chainId": tx_req["chainId"],
            "gasLimit": tx_req["gasLimit"]
        },
        "approvalAddress": resp["estimate"]["approvalAddress"]
    }))
}
