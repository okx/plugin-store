use anyhow::Context;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};

use crate::calldata;
use crate::config::QUOTER;

async fn eth_call(to: &str, data: &str, rpc: &str) -> anyhow::Result<String> {
    let client = Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });
    let resp = client
        .post(rpc)
        .json(&body)
        .send()
        .await
        .context("RPC request failed")?;
    let val: Value = resp.json().await.context("RPC response parse failed")?;
    if let Some(err) = val.get("error") {
        return Err(anyhow::anyhow!("RPC error: {}", err));
    }
    Ok(val["result"].as_str().unwrap_or("0x").to_string())
}

pub async fn get_erc20_balance(token: &str, account: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = calldata::encode_balance_of(account);
    let result = eth_call(token, &data, rpc).await?;
    calldata::decode_u128(&result).context("Failed to decode balance")
}

pub async fn get_erc20_decimals(token: &str, rpc: &str) -> anyhow::Result<u8> {
    let data = calldata::encode_decimals();
    let result = eth_call(token, &data, rpc).await?;
    calldata::decode_u8(&result).context("Failed to decode decimals")
}

pub async fn get_erc20_symbol(token: &str, rpc: &str) -> anyhow::Result<String> {
    let data = calldata::encode_symbol();
    let result = eth_call(token, &data, rpc).await?;
    calldata::decode_string(&result).context("Failed to decode symbol")
}

pub async fn get_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc: &str,
) -> anyhow::Result<u128> {
    let data = calldata::encode_allowance(owner, spender);
    let result = eth_call(token, &data, rpc).await?;
    calldata::decode_u128(&result).context("Failed to decode allowance")
}

pub async fn get_matic_balance(account: &str, rpc: &str) -> anyhow::Result<u128> {
    let client = Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [account, "latest"],
        "id": 1
    });
    let resp = client
        .post(rpc)
        .json(&body)
        .send()
        .await
        .context("eth_getBalance request failed")?;
    let val: Value = resp.json().await.context("eth_getBalance parse failed")?;
    if let Some(err) = val.get("error") {
        return Err(anyhow::anyhow!("RPC error: {}", err));
    }
    let hex_str = val["result"].as_str().unwrap_or("0x0");
    calldata::decode_u128(hex_str).context("Failed to decode MATIC balance")
}

pub async fn quote_exact_input_single(
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    rpc: &str,
) -> anyhow::Result<u128> {
    let data = calldata::encode_quote_exact_input_single(token_in, token_out, amount_in);
    let result = eth_call(QUOTER, &data, rpc).await.map_err(|e| {
        anyhow::anyhow!(
            "Quoter call failed (pool may not exist for this pair): {}",
            e
        )
    })?;
    if result == "0x" || result.is_empty() {
        return Err(anyhow::anyhow!(
            "Quoter returned empty result — no pool exists for this token pair on QuickSwap V3"
        ));
    }
    // Result is (uint256 amountOut, uint16 fee) packed in 64 bytes.
    // decode_u128 reads the LAST 16 bytes so we must truncate to first 32 bytes (= first 64 hex chars + "0x").
    let first32 = if result.len() > 66 {
        result[..66].to_string()
    } else {
        result.clone()
    };
    calldata::decode_u128(&first32).context("Failed to decode quote amountOut")
}

pub async fn wait_for_tx(rpc: &str, tx_hash: &str) -> anyhow::Result<()> {
    let client = Client::new();
    for _ in 0..30 {
        sleep(Duration::from_secs(2)).await;
        let body = json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });
        let resp = client
            .post(rpc)
            .json(&body)
            .send()
            .await
            .context("eth_getTransactionReceipt request failed")?;
        let val: Value = resp.json().await.context("receipt parse failed")?;
        if let Some(receipt) = val.get("result") {
            if !receipt.is_null() {
                // Check status
                if let Some(status) = receipt.get("status") {
                    let s = status.as_str().unwrap_or("0x0");
                    if s == "0x0" {
                        return Err(anyhow::anyhow!("Transaction {} reverted", tx_hash));
                    }
                }
                return Ok(());
            }
        }
    }
    Err(anyhow::anyhow!(
        "Transaction {} not confirmed after 60 seconds",
        tx_hash
    ))
}
