use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use crate::config::SOLANA_RPC;

/// Make a Solana JSON-RPC call
pub async fn solana_rpc(method: &str, params: Value) -> Result<Value> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });
    let resp = client
        .post(SOLANA_RPC)
        .json(&body)
        .send()
        .await?
        .json::<Value>()
        .await?;

    if let Some(err) = resp.get("error") {
        return Err(anyhow!("RPC error: {}", err));
    }
    Ok(resp["result"].clone())
}

/// Get account data as base64-decoded bytes
pub async fn get_account_data(address: &str) -> Result<Vec<u8>> {
    let result = solana_rpc(
        "getAccountInfo",
        json!([address, {"encoding": "base64"}]),
    )
    .await?;

    let b64 = result["value"]["data"][0]
        .as_str()
        .ok_or_else(|| anyhow!("No account data for {}", address))?;

    use base64::{engine::general_purpose::STANDARD, Engine as _};
    Ok(STANDARD.decode(b64)?)
}

/// Get token account balance (returns ui_amount as f64 and raw amount as u64)
pub async fn get_token_balance(address: &str) -> Result<(f64, u64)> {
    let result = solana_rpc(
        "getTokenAccountBalance",
        json!([address]),
    )
    .await?;

    let value = &result["value"];
    let ui_amount = value["uiAmount"]
        .as_f64()
        .unwrap_or(0.0);
    let amount_str = value["amount"].as_str().unwrap_or("0");
    let raw: u64 = amount_str.parse().unwrap_or(0);
    Ok((ui_amount, raw))
}

/// Get latest blockhash
pub async fn get_latest_blockhash() -> Result<String> {
    let result = solana_rpc(
        "getLatestBlockhash",
        json!([{"commitment": "finalized"}]),
    )
    .await?;
    result["value"]["blockhash"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Failed to get blockhash"))
}

/// Get the largest JitoSOL token account for a given wallet owner.
/// Returns (ui_balance, raw_balance, account_pubkey)
pub async fn get_token_accounts_by_owner(wallet: &str, mint: &str) -> Result<(f64, u64, String)> {
    let result = solana_rpc(
        "getTokenAccountsByOwner",
        json!([wallet, {"mint": mint}, {"encoding": "jsonParsed"}]),
    )
    .await?;

    let accounts = result["value"]
        .as_array()
        .ok_or_else(|| anyhow!("No token accounts"))?;

    // Find account with highest balance
    let mut best_ui = 0.0f64;
    let mut best_raw = 0u64;
    let mut best_addr = String::new();

    for acc in accounts {
        let amount = &acc["account"]["data"]["parsed"]["info"]["tokenAmount"];
        let ui = amount["uiAmount"].as_f64().unwrap_or(0.0);
        let raw_str = amount["amount"].as_str().unwrap_or("0");
        let raw: u64 = raw_str.parse().unwrap_or(0);
        let addr = acc["pubkey"].as_str().unwrap_or("").to_string();

        if raw > best_raw {
            best_ui = ui;
            best_raw = raw;
            best_addr = addr;
        }
    }

    if best_addr.is_empty() {
        return Err(anyhow!("No JitoSOL token accounts found for wallet {}", wallet));
    }

    Ok((best_ui, best_raw, best_addr))
}

/// Parse SPL Stake Pool account data (611 bytes)
/// Returns (total_lamports, pool_token_supply, reserve_stake_bytes, manager_fee_account_bytes,
///          validator_list_bytes, pool_mint_bytes)
pub fn parse_stake_pool(data: &[u8]) -> Result<StakePoolInfo> {
    if data.len() < 298 {
        return Err(anyhow!("Stake pool account data too short: {}", data.len()));
    }

    // account_type: byte 0
    // manager: bytes 1-32 (32 bytes)
    // staker: bytes 33-64 (32 bytes)
    // stake_deposit_authority: bytes 65-96 (32 bytes)
    // stake_withdraw_bump_seed: byte 97
    // validator_list: bytes 98-129 (32 bytes)
    // reserve_stake: bytes 130-161 (32 bytes)
    // pool_mint: bytes 162-193 (32 bytes)
    // manager_fee_account: bytes 194-225 (32 bytes)
    // token_program_id: bytes 226-257 (32 bytes)
    // total_lamports: bytes 258-265 (u64 LE)
    // pool_token_supply: bytes 266-273 (u64 LE)

    let validator_list = data[98..130].to_vec();
    let reserve_stake = data[130..162].to_vec();
    let pool_mint = data[162..194].to_vec();
    let manager_fee_account = data[194..226].to_vec();

    let total_lamports = u64::from_le_bytes(data[258..266].try_into().unwrap());
    let pool_token_supply = u64::from_le_bytes(data[266..274].try_into().unwrap());

    Ok(StakePoolInfo {
        validator_list,
        reserve_stake,
        pool_mint,
        manager_fee_account,
        total_lamports,
        pool_token_supply,
    })
}

pub struct StakePoolInfo {
    #[allow(dead_code)]
    pub validator_list: Vec<u8>,
    pub reserve_stake: Vec<u8>,
    pub pool_mint: Vec<u8>,
    pub manager_fee_account: Vec<u8>,
    pub total_lamports: u64,
    pub pool_token_supply: u64,
}
